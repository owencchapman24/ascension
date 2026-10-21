//! Additive organization must preserve every valid Stage 4 capacity boundary.
use ascension_lib::{domain::*, service::WorkspaceService};
use rusqlite::Connection;
use std::path::Path;
use uuid::Uuid;

fn restore(service: &mut WorkspaceService, bytes: &[u8]) -> Workspace {
    let preview = service
        .preview_import(bytes, "synthetic-capacity.json".into())
        .unwrap();
    service
        .restore_import(RestoreInput {
            token: preview.token,
            expected_generation: preview.generation,
        })
        .unwrap()
        .workspace
}

fn legacy_seed(count: usize, notes: bool) -> Vec<u8> {
    let mut records: RecordSet =
        serde_json::from_slice(include_bytes!("../../contracts/fixtures/sample-v1.json")).unwrap();
    let original = records.objectives[0].clone();
    let note = records.notes[0].clone();
    records.projects.truncate(1);
    records.objectives.clear();
    records.actions.clear();
    records.notes.clear();
    for index in 0..count {
        let mut objective = original.clone();
        objective.id = Uuid::new_v4().to_string();
        objective.title = format!("Synthetic capacity objective {index}");
        objective.criterion = "A synthetic bounded result exists.".into();
        if notes {
            let mut row = note.clone();
            row.id = Uuid::new_v4().to_string();
            row.objective_id = objective.id.clone();
            row.text = "Synthetic note".into();
            records.notes.push(row);
        }
        records.objectives.push(objective);
    }
    records.selected_objective_id = records.objectives.first().map(|o| o.id.clone());
    serde_json::to_vec_pretty(&records).unwrap()
}

fn downgrade_to_stage4_database(path: &Path) {
    let database = Connection::open(path).unwrap();
    database.execute_batch("PRAGMA foreign_keys=OFF; DROP TABLE checkpoint_commands; DROP TABLE work_checkpoints; DROP TABLE objective_planning; PRAGMA user_version=5;").unwrap();
}

#[test]
fn maximum_stage4_objective_count_migrates_and_modern_record_cap_remains_bounded() {
    let directory = tempfile::tempdir().unwrap();
    let database = directory.path().join("capacity.sqlite3");
    let backups = directory.path().join("backups");
    let mut service = WorkspaceService::open(&database, &backups).unwrap();
    let before = restore(&mut service, &legacy_seed(2499, false));
    assert_eq!(before.objectives.len(), 2499);
    let mut legacy = RecordSet::from(&before);
    legacy.schema_version = 4;
    legacy.daily = None;
    assert_eq!(
        legacy.projects.len()
            + legacy.objectives.len()
            + legacy.progress.as_ref().unwrap().record_count(),
        9997
    );
    let legacy_bytes = serde_json::to_vec_pretty(&legacy).unwrap();
    assert!(legacy_bytes.len() < LEGACY_MAX_IMPORT_BYTES);
    let mut oversized_legacy = legacy.clone();
    for _ in 0..4 {
        let mut project = oversized_legacy.projects[0].clone();
        project.id = Uuid::new_v4().to_string();
        oversized_legacy.projects.push(project);
    }
    // 10,001 old rows would become exactly 12,500 current rows, so only the
    // retained legacy bound can reject this otherwise valid envelope.
    assert_eq!(
        service
            .preview_import(
                &serde_json::to_vec_pretty(&oversized_legacy).unwrap(),
                "legacy-too-many.json".into()
            )
            .unwrap_err()
            .code,
        "invalid_import"
    );
    let expected = service.export_bytes().unwrap();
    drop(service);
    downgrade_to_stage4_database(&database);
    let mut reopened = WorkspaceService::open(&database, &backups).unwrap();
    assert_eq!(reopened.export_bytes().unwrap(), expected);
    assert_eq!(reopened.load_workspace().unwrap(), before);
    let mut modern = RecordSet::from(&before);
    // 9,997 old rows + 2,499 planning rows = 12,496 rows. Four more fit.
    for _ in 0..4 {
        let mut project = modern.projects[0].clone();
        project.id = Uuid::new_v4().to_string();
        modern.projects.push(project);
    }
    let accepted = serde_json::to_vec_pretty(&modern).unwrap();
    reopened
        .preview_import(&accepted, "synthetic-modern-boundary.json".into())
        .unwrap();
    let mut extra = modern.projects[0].clone();
    extra.id = Uuid::new_v4().to_string();
    modern.projects.push(extra);
    assert_eq!(
        reopened
            .preview_import(
                &serde_json::to_vec_pretty(&modern).unwrap(),
                "too-many-records.json".into()
            )
            .unwrap_err()
            .code,
        "invalid_import"
    );
    assert_eq!(reopened.export_bytes().unwrap(), expected);
}

#[test]
fn near_ten_mib_stage4_payload_migrates_without_losing_notes_or_portability() {
    let directory = tempfile::tempdir().unwrap();
    let database = directory.path().join("bytes.sqlite3");
    let backups = directory.path().join("backups");
    let mut service = WorkspaceService::open(&database, &backups).unwrap();
    let seeded = restore(&mut service, &legacy_seed(1300, true));
    let mut legacy = RecordSet::from(&seeded);
    legacy.schema_version = 4;
    legacy.daily = None;
    let target = LEGACY_MAX_IMPORT_BYTES - 16;
    let mut remaining = target - serde_json::to_vec_pretty(&legacy).unwrap().len();
    for note in &mut legacy.notes {
        let added = remaining.min(8000 - note.text.len());
        note.text.push_str(&"x".repeat(added));
        remaining -= added;
    }
    assert_eq!(
        remaining, 0,
        "The synthetic note fields have enough valid capacity"
    );
    let bytes = serde_json::to_vec_pretty(&legacy).unwrap();
    assert_eq!(bytes.len(), target);
    let before = restore(&mut service, &bytes);
    let expected = service.export_bytes().unwrap();
    assert!(expected.len() > LEGACY_MAX_IMPORT_BYTES);
    assert!(expected.len() < MAX_IMPORT_BYTES);
    drop(service);
    downgrade_to_stage4_database(&database);
    let reopened = WorkspaceService::open(&database, &backups).unwrap();
    assert_eq!(reopened.export_bytes().unwrap(), expected);
    assert_eq!(reopened.load_workspace().unwrap(), before);
    let mut target = WorkspaceService::open(
        &directory.path().join("restored.sqlite3"),
        &directory.path().join("restored-backups"),
    )
    .unwrap();
    restore(&mut target, &expected);
    assert_eq!(target.export_bytes().unwrap(), expected);
}

#[test]
fn raw_byte_limits_remain_version_specific_and_preview_failures_preserve_state() {
    let directory = tempfile::tempdir().unwrap();
    let mut service = WorkspaceService::open(
        &directory.path().join("bytes.sqlite3"),
        &directory.path().join("backups"),
    )
    .unwrap();
    let before = service.load_workspace().unwrap();
    let mut legacy = include_bytes!("../../contracts/fixtures/empty-v1.json").to_vec();
    legacy.resize(LEGACY_MAX_IMPORT_BYTES, b' ');
    service
        .preview_import(&legacy, "legacy-at-limit.json".into())
        .unwrap();
    legacy.push(b' ');
    assert_eq!(
        service
            .preview_import(&legacy, "legacy-too-large.json".into())
            .unwrap_err()
            .code,
        "invalid_import"
    );
    let mut modern = service.export_bytes().unwrap();
    modern.resize(MAX_IMPORT_BYTES, b' ');
    service
        .preview_import(&modern, "modern-at-limit.json".into())
        .unwrap();
    modern.push(b' ');
    assert_eq!(
        service
            .preview_import(&modern, "modern-too-large.json".into())
            .unwrap_err()
            .code,
        "invalid_import"
    );
    assert_eq!(service.load_workspace().unwrap(), before);
}
