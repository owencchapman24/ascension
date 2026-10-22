//! Failed storage opens preserve the primary, committed WAL, and recovery points.
use ascension_lib::{domain::*, service::WorkspaceService};
use rusqlite::Connection;
use std::fs;

#[test]
fn only_absent_database_paths_initialize_without_an_explicit_recovery_decision() {
    let directory = tempfile::tempdir().unwrap();
    let backups = directory.path().join("backups");
    let new_database = directory.path().join("new.sqlite3");
    assert!(!new_database.exists());
    let service = WorkspaceService::open(&new_database, &backups).unwrap();
    let initial = service.load_workspace().unwrap();
    assert!(initial.projects.is_empty());
    drop(service);
    assert_eq!(
        WorkspaceService::open(&new_database, &backups)
            .unwrap()
            .load_workspace()
            .unwrap(),
        initial
    );

    let zero = directory.path().join("preexisting-zero.sqlite3");
    fs::write(&zero, []).unwrap();
    assert_eq!(
        WorkspaceService::open(&zero, &backups).err().unwrap().code,
        "storage_error"
    );
    assert!(fs::read(&zero).unwrap().is_empty());

    let empty_sqlite = directory.path().join("preexisting-empty.sqlite3");
    let connection = Connection::open(&empty_sqlite).unwrap();
    connection.execute_batch("VACUUM;").unwrap();
    assert_eq!(
        connection
            .pragma_query_value(None, "user_version", |r| r.get::<_, u32>(0))
            .unwrap(),
        0
    );
    drop(connection);
    let original = fs::read(&empty_sqlite).unwrap();
    assert!(!original.is_empty());
    assert_eq!(
        WorkspaceService::open(&empty_sqlite, &backups)
            .err()
            .unwrap()
            .code,
        "storage_error"
    );
    assert_eq!(fs::read(&empty_sqlite).unwrap(), original);
}

#[test]
fn truncated_primary_is_not_recreated_and_its_verified_snapshot_remains_recoverable() {
    let directory = tempfile::tempdir().unwrap();
    let database = directory.path().join("damaged.sqlite3");
    let backups = directory.path().join("backups");
    let mut service = WorkspaceService::open(&database, &backups).unwrap();
    service
        .create_project(CreateProjectInput {
            expected_generation: service.load_workspace().unwrap().generation,
            title: "Synthetic history preserved after primary truncation".into(),
        })
        .unwrap();
    let expected = service.export_bytes().unwrap();
    // Closing joins the finite snapshot worker and checkpoints/removes the WAL.
    drop(service);
    assert!(!database.with_extension("sqlite3-wal").exists());
    assert!(!database.with_extension("sqlite3-shm").exists());
    let snapshots: Vec<_> = fs::read_dir(&backups)
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .filter(|path| {
            path.file_name()
                .unwrap()
                .to_string_lossy()
                .starts_with("daily-v1-")
        })
        .map(|path| {
            let bytes = fs::read(&path).unwrap();
            (path, bytes)
        })
        .collect();
    assert_eq!(snapshots.len(), 1);
    fs::write(&database, []).unwrap();
    for _ in 0..2 {
        assert_eq!(
            WorkspaceService::open(&database, &backups)
                .err()
                .unwrap()
                .code,
            "storage_error"
        );
        assert!(fs::read(&database).unwrap().is_empty());
        for (path, bytes) in &snapshots {
            assert_eq!(&fs::read(path).unwrap(), bytes);
        }
    }
    let fresh = directory.path().join("explicit-fresh-profile.sqlite3");
    let mut target = WorkspaceService::open(&fresh, &backups).unwrap();
    let preview = target.preview_recovery().unwrap();
    let result = target
        .restore_import(RestoreInput {
            token: preview.token,
            expected_generation: preview.generation,
        })
        .unwrap();
    assert!(result.backup_created);
    assert_eq!(target.export_bytes().unwrap(), expected);
    assert!(fs::read(&database).unwrap().is_empty());
    for (path, bytes) in &snapshots {
        assert_eq!(&fs::read(path).unwrap(), bytes);
    }
}

#[test]
fn corrupt_primary_with_partial_committed_wal_is_preserved_before_read_write_open() {
    let directory = tempfile::tempdir().unwrap();
    let source = directory.path().join("source.sqlite3");
    let damaged = directory.path().join("damaged.sqlite3");
    let backups = directory.path().join("backups");
    let mut service = WorkspaceService::open(&source, &backups).unwrap();
    service
        .create_project(CreateProjectInput {
            expected_generation: service.load_workspace().unwrap().generation,
            title: "Synthetic committed history survives failed startup".into(),
        })
        .unwrap();
    let expected = service.export_bytes().unwrap();
    drop(service);
    let snapshots: Vec<_> = fs::read_dir(&backups)
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .filter(|path| {
            path.file_name()
                .unwrap()
                .to_string_lossy()
                .starts_with("daily-v1-")
        })
        .map(|path| {
            let bytes = fs::read(&path).unwrap();
            (path, bytes)
        })
        .collect();
    assert_eq!(snapshots.len(), 1);

    // An ordinary WAL contains only changed pages, not a complete database.
    // Page 1 in the recent commit makes user_version readable even when older
    // required pages in the copied primary have been lost. Copy while idle and
    // open so closing the source cannot checkpoint the damaged fixture's WAL.
    let writer = Connection::open(&source).unwrap();
    writer
        .execute_batch(
            "PRAGMA wal_autocheckpoint=0; PRAGMA wal_checkpoint(TRUNCATE);
             CREATE TABLE synthetic_wal_marker(value TEXT NOT NULL);
             PRAGMA user_version=6;
             INSERT INTO synthetic_wal_marker VALUES('Synthetic recent WAL page');",
        )
        .unwrap();
    for suffix in ["sqlite3", "sqlite3-wal", "sqlite3-shm"] {
        fs::copy(
            source.with_extension(suffix),
            damaged.with_extension(suffix),
        )
        .unwrap();
    }
    drop(writer);
    let corrupt_main = b"Synthetic invalid SQLite primary";
    fs::write(&damaged, corrupt_main).unwrap();
    let wal = damaged.with_extension("sqlite3-wal");
    let committed_wal = fs::read(&wal).unwrap();
    assert!(!committed_wal.is_empty());
    let probe = directory.path().join("fixture-probe.sqlite3");
    for suffix in ["sqlite3", "sqlite3-wal", "sqlite3-shm"] {
        fs::copy(damaged.with_extension(suffix), probe.with_extension(suffix)).unwrap();
    }
    let read_only =
        Connection::open_with_flags(&probe, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY).unwrap();
    assert_eq!(
        read_only
            .pragma_query_value(None, "user_version", |row| row.get::<_, u32>(0))
            .unwrap(),
        6
    );
    assert!(
        read_only
            .query_row("PRAGMA quick_check", [], |row| row.get::<_, String>(0))
            .is_err()
    );
    drop(read_only);

    for _ in 0..2 {
        assert_eq!(
            WorkspaceService::open(&damaged, &backups)
                .err()
                .unwrap()
                .code,
            "storage_error"
        );
        assert!(
            fs::read(&damaged).unwrap() == corrupt_main,
            "failed open changed the main file"
        );
        assert!(
            fs::read(&wal).unwrap() == committed_wal,
            "failed open changed the committed WAL"
        );
        for (path, bytes) in &snapshots {
            assert!(
                &fs::read(path).unwrap() == bytes,
                "failed open changed a recovery snapshot"
            );
        }
    }

    // Preview uses the same snapshot-to-portable-export function as recovery
    // export, without opening the failed primary. Restore to a separate profile
    // proves that the complete validated export still contains the original work.
    let fresh = directory.path().join("explicit-fresh-profile.sqlite3");
    let mut target = WorkspaceService::open(&fresh, &backups).unwrap();
    let preview = target.preview_recovery().unwrap();
    target
        .restore_import(RestoreInput {
            token: preview.token,
            expected_generation: preview.generation,
        })
        .unwrap();
    assert_eq!(target.export_bytes().unwrap(), expected);
    assert!(
        fs::read(&damaged).unwrap() == corrupt_main,
        "recovery changed the failed main file"
    );
    assert!(
        fs::read(&wal).unwrap() == committed_wal,
        "recovery changed the failed WAL"
    );
    for (path, bytes) in &snapshots {
        assert!(
            &fs::read(path).unwrap() == bytes,
            "recovery changed its source snapshot"
        );
    }
}
