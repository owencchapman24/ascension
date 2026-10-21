//! An empty existing primary is a recovery decision, not permission to reset.
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
