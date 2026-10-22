use super::*;
use crate::domain::RestoreInput;
use crate::service::WorkspaceService;
use rusqlite::Connection;
use serde_json::Value;

fn entries(directory: &Path) -> Vec<Value> {
    fs::read_to_string(directory.join(FILE_NAME))
        .unwrap()
        .lines()
        .skip(1)
        .map(|line| serde_json::from_str(line).unwrap())
        .collect()
}

fn preview(service: &mut WorkspaceService) -> RestoreInput {
    let preview = service
        .preview_import(
            include_bytes!("../../../contracts/fixtures/sample-v1.json"),
            "synthetic.json".into(),
        )
        .unwrap();
    RestoreInput {
        token: preview.token,
        expected_generation: preview.generation,
    }
}

#[test]
fn native_diagnostics_require_both_opt_in_and_an_isolated_verification_identifier() {
    let verification = "io.ascension.verification.r0123456789abcdef0123456789abcdef";
    assert!(permitted(verification, true));
    assert!(!permitted(verification, false));
    assert!(!permitted("io.ascension.work", true));
    assert!(!permitted(
        "io.ascension.verification.rnot-an-isolated-run",
        true
    ));
    let directory = tempfile::tempdir().unwrap();
    attempt(|| Ok(())).unwrap();
    assert!(!directory.path().join(FILE_NAME).exists());
}

#[test]
fn diagnostics_never_write_unknown_files_or_exceed_the_bound() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join(FILE_NAME);
    fs::write(&path, b"unrelated synthetic file").unwrap();
    with_test_sink(directory.path(), || attempt(|| Ok(()))).unwrap();
    assert_eq!(fs::read(&path).unwrap(), b"unrelated synthetic file");
    fs::write(&path, HEADER).unwrap();
    let file = OpenOptions::new().write(true).open(&path).unwrap();
    file.set_len(MAX_BYTES).unwrap();
    drop(file);
    with_test_sink(directory.path(), || attempt(|| Ok(()))).unwrap();
    assert_eq!(fs::metadata(path).unwrap().len(), MAX_BYTES);
}

#[test]
fn arbitrary_error_messages_paths_and_table_names_are_never_serialized() {
    let directory = tempfile::tempdir().unwrap();
    let secret = "SYNTHETIC_PRIVATE_NOTE_AND_PATH_C:\\private\\record.json";
    with_test_sink(directory.path(), || {
        attempt::<()>(|| {
            table_operation("insert", secret);
            sqlite(&rusqlite::Error::SqliteFailure(
                rusqlite::ffi::Error::new(1811),
                Some(secret.into()),
            ));
            Err(CommandError {
                code: secret.into(),
                message: secret.into(),
            })
        })
        .unwrap_err();
    });
    let text = fs::read_to_string(directory.path().join(FILE_NAME)).unwrap();
    assert!(!text.contains(secret));
    assert!(!text.contains(&directory.path().to_string_lossy().to_string()));
    let values = entries(directory.path());
    assert_eq!(values[0]["operation"], "other_table");
    assert_eq!(values[0]["sqlite_primary_code"], 19);
    assert_eq!(values[0]["sqlite_extended_code"], 1811);
    assert_eq!(values[0]["command_error_code"], "other");
}

#[test]
fn displaced_backup_failure_has_safe_filesystem_phase_and_confirmed_rollback() {
    let directory = tempfile::tempdir().unwrap();
    let database = directory.path().join("workspace.sqlite3");
    let backups = directory.path().join("blocked-backups");
    fs::write(&backups, b"Synthetic blocker").unwrap();
    let mut service = WorkspaceService::open(&database, &backups).unwrap();
    let before = service.export_bytes().unwrap();
    let input = preview(&mut service);
    let failure = with_test_sink(directory.path(), || service.restore_import(input)).unwrap_err();
    assert_eq!(failure.code, "backup_failed");
    assert_eq!(service.export_bytes().unwrap(), before);
    let values = entries(directory.path());
    assert_eq!(values.len(), 1);
    assert_eq!(values[0]["phase"], "create_backup_directory");
    assert!(values[0]["filesystem_category"].is_string());
    assert_eq!(values[0]["transaction_outcome"], "rolled_back");
    assert_eq!(values[0]["displaced_backup_created"], false);
    assert!(values[0]["sqlite_primary_code"].is_null());
}

#[test]
fn inserted_row_abort_records_original_sqlite_code_and_retains_state_and_backup() {
    let directory = tempfile::tempdir().unwrap();
    let database = directory.path().join("workspace.sqlite3");
    let backups = directory.path().join("backups");
    let mut service = WorkspaceService::open(&database, &backups).unwrap();
    let before = service.export_bytes().unwrap();
    let connection = Connection::open(&database).unwrap();
    connection.execute_batch("CREATE TRIGGER synthetic_restore_abort BEFORE INSERT ON projects BEGIN SELECT RAISE(ABORT,'SYNTHETIC_PRIVATE_TRIGGER_MESSAGE'); END;").unwrap();
    let input = preview(&mut service);
    let failure = with_test_sink(directory.path(), || service.restore_import(input)).unwrap_err();
    assert_eq!(failure.code, "storage_error");
    assert_eq!(service.export_bytes().unwrap(), before);
    let values = entries(directory.path());
    assert_eq!(values[0]["phase"], "insert_projects");
    assert_eq!(values[0]["sqlite_primary_code"], 19);
    assert_eq!(values[0]["sqlite_extended_code"], 1811);
    assert_eq!(values[0]["transaction_outcome"], "rolled_back");
    assert_eq!(values[0]["displaced_backup_created"], true);
    let text = fs::read_to_string(directory.path().join(FILE_NAME)).unwrap();
    assert!(!text.contains("SYNTHETIC_PRIVATE_TRIGGER_MESSAGE"));
    let copies: Vec<_> = fs::read_dir(&backups)
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .collect();
    assert_eq!(copies.len(), 1);
    assert_eq!(fs::read(&copies[0]).unwrap(), before);
}

#[test]
fn deferred_foreign_key_commit_failure_is_distinct_and_explicit_retry_commits_exactly() {
    let directory = tempfile::tempdir().unwrap();
    let database = directory.path().join("workspace.sqlite3");
    let mut service = WorkspaceService::open(&database, &directory.path().join("backups")).unwrap();
    let before = service.export_bytes().unwrap();
    let connection = Connection::open(&database).unwrap();
    connection.execute_batch("CREATE TABLE synthetic_commit_guard(id TEXT REFERENCES projects(id) DEFERRABLE INITIALLY DEFERRED); CREATE TRIGGER synthetic_commit_fault AFTER INSERT ON projects BEGIN INSERT INTO synthetic_commit_guard VALUES('synthetic-missing-parent'); END;").unwrap();
    let input = preview(&mut service);
    with_test_sink(directory.path(), || {
        assert_eq!(
            service
                .restore_import(RestoreInput {
                    token: input.token.clone(),
                    expected_generation: input.expected_generation.clone()
                })
                .unwrap_err()
                .code,
            "storage_error"
        );
        assert_eq!(service.export_bytes().unwrap(), before);
        connection
            .execute_batch(
                "DROP TRIGGER synthetic_commit_fault; DROP TABLE synthetic_commit_guard;",
            )
            .unwrap();
        service.restore_import(input).unwrap();
        let expected = service.export_bytes().unwrap();
        let repeated = service
            .preview_import(&expected, "synthetic.json".into())
            .unwrap();
        assert!(
            service
                .restore_import(RestoreInput {
                    token: repeated.token,
                    expected_generation: repeated.generation
                })
                .unwrap()
                .unchanged
        );
        assert_eq!(service.export_bytes().unwrap(), expected);
    });
    let values = entries(directory.path());
    assert_eq!(values.len(), 3);
    assert_eq!(values[0]["phase"], "commit");
    assert_eq!(values[0]["sqlite_primary_code"], 19);
    assert_eq!(values[0]["sqlite_extended_code"], 787);
    assert_eq!(values[0]["transaction_outcome"], "rolled_back");
    assert_eq!(values[0]["displaced_backup_created"], true);
    assert_eq!(values[1]["outcome"], "success");
    assert_eq!(values[1]["transaction_outcome"], "committed");
    assert_eq!(values[2]["outcome"], "unchanged");
    assert_eq!(values[2]["transaction_outcome"], "committed");
    assert_eq!(values[2]["displaced_backup_created"], false);
    assert_eq!(values[2]["attempt_sequence"], 3);
}

#[cfg(windows)]
#[test]
fn a_diagnostic_directory_junction_is_not_followed() {
    use std::os::windows::process::CommandExt;
    let directory = tempfile::tempdir().unwrap();
    let target = directory.path().join("target");
    let junction = directory.path().join("linked");
    fs::create_dir(&target).unwrap();
    let output = std::process::Command::new("powershell.exe")
        .args(["-NoProfile", "-NonInteractive", "-Command", "New-Item -ItemType Junction -Path $env:ASCENSION_DIAGNOSTIC_LINK -Target $env:ASCENSION_DIAGNOSTIC_TARGET | Out-Null"])
        .env("ASCENSION_DIAGNOSTIC_LINK", &junction).env("ASCENSION_DIAGNOSTIC_TARGET", &target)
        .creation_flags(0x0800_0000).output().unwrap();
    assert!(
        output.status.success(),
        "synthetic directory junction creation failed"
    );
    with_test_sink(&junction, || attempt(|| Ok(()))).unwrap();
    assert!(!target.join(FILE_NAME).exists());
    fs::remove_dir(&junction).unwrap();
}

#[test]
fn external_writer_is_reported_as_busy_before_transaction_and_retry_is_explicit() {
    let directory = tempfile::tempdir().unwrap();
    let database = directory.path().join("workspace.sqlite3");
    let backups = directory.path().join("backups");
    let mut service = WorkspaceService::open(&database, &backups).unwrap();
    let before = service.load_workspace().unwrap();
    let input = preview(&mut service);
    let writer = Connection::open(&database).unwrap();
    writer.execute_batch("BEGIN IMMEDIATE;").unwrap();
    with_test_sink(directory.path(), || {
        assert_eq!(
            service
                .restore_import(RestoreInput {
                    token: input.token.clone(),
                    expected_generation: input.expected_generation.clone(),
                })
                .unwrap_err()
                .code,
            "busy"
        );
        writer.execute_batch("ROLLBACK;").unwrap();
        assert_eq!(service.load_workspace().unwrap(), before);
        assert!(!backups.exists());
        service.restore_import(input).unwrap();
    });
    let values = entries(directory.path());
    assert_eq!(values.len(), 2);
    assert_eq!(values[0]["phase"], "begin_transaction");
    assert_eq!(values[0]["command_error_code"], "busy");
    assert_eq!(values[0]["sqlite_primary_code"], 5);
    assert_eq!(values[0]["sqlite_extended_code"], 5);
    assert_eq!(values[0]["transaction_outcome"], "not_started");
    assert_eq!(values[0]["displaced_backup_created"], false);
    assert_eq!(values[1]["outcome"], "success");
    assert_eq!(values[1]["transaction_outcome"], "committed");
}
