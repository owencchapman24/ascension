use ascension_lib::domain::*;
use ascension_lib::service::WorkspaceService;
use rusqlite::{Connection, params};
use serde_json::{Value, json};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Barrier};
use tempfile::TempDir;

const SAMPLE: &[u8] = include_bytes!("../../contracts/fixtures/sample-v1.json");
const EMPTY: &[u8] = include_bytes!("../../contracts/fixtures/empty-v1.json");

struct Fixture {
    _directory: TempDir,
    database: PathBuf,
    backups: PathBuf,
    service: WorkspaceService,
}

impl Fixture {
    fn new() -> Self {
        let directory = tempfile::tempdir().unwrap();
        let database = directory.path().join("workspace.sqlite3");
        let backups = directory.path().join("backups");
        let service = WorkspaceService::open(&database, &backups).unwrap();
        Self {
            _directory: directory,
            database,
            backups,
            service,
        }
    }

    fn populate(&mut self) -> Workspace {
        let workspace = self.service.load_workspace().unwrap();
        let workspace = self
            .service
            .create_project(CreateProjectInput {
                expected_generation: workspace.generation,
                title: "Synthetic handbook".into(),
            })
            .unwrap();
        let workspace = self
            .service
            .create_objective(CreateObjectiveInput {
                expected_generation: workspace.generation,
                project_id: workspace.projects[0].id.clone(),
                title: "Prepare one example".into(),
                criterion: "A reader can follow the example from start to finish.".into(),
            })
            .unwrap();
        self.service
            .save_work(SaveWorkInput {
                expected_generation: workspace.generation,
                objective_id: workspace.objectives[0].id.clone(),
                description: "Draft the opening example".into(),
                stop_condition: "One example contains all three steps.".into(),
                note: Some("Resume at step two; the inputs are already listed.".into()),
            })
            .unwrap()
    }

    fn restore(&mut self, bytes: &[u8]) -> RestoreResult {
        let preview = self
            .service
            .preview_import(bytes, "synthetic.json".into())
            .unwrap();
        self.service
            .restore_import(RestoreInput {
                token: preview.token,
                expected_generation: preview.generation,
            })
            .unwrap()
    }

    fn backup_paths(&self) -> Vec<PathBuf> {
        fs::read_dir(&self.backups)
            .map(|entries| entries.map(|entry| entry.unwrap().path()).collect())
            .unwrap_or_default()
    }
}

fn save_input(workspace: &Workspace, note: Option<String>) -> SaveWorkInput {
    SaveWorkInput {
        expected_generation: workspace.generation.clone(),
        objective_id: workspace.objectives[0].id.clone(),
        description: workspace.actions[0].description.clone(),
        stop_condition: workspace.actions[0].stop_condition.clone(),
        note,
    }
}

#[test]
fn initialization_and_restart_preserve_authoritative_records_and_selection() {
    let mut fixture = Fixture::new();
    let initial = fixture.service.load_workspace().unwrap();
    assert!(initial.projects.is_empty());
    assert!(initial.selected_objective_id.is_none());
    let workspace = fixture.populate();
    assert_eq!(
        workspace.selected_objective_id.as_ref(),
        Some(&workspace.objectives[0].id)
    );
    assert_eq!(workspace.actions[0].revision, 1);
    assert_eq!(workspace.notes[0].created_at, workspace.notes[0].updated_at);
    let expected = fixture.service.export_bytes().unwrap();
    drop(fixture.service);
    let reopened = WorkspaceService::open(&fixture.database, &fixture.backups).unwrap();
    assert_eq!(reopened.load_workspace().unwrap(), workspace);
    assert_eq!(reopened.export_bytes().unwrap(), expected);
    let connection = Connection::open(&fixture.database).unwrap();
    assert_eq!(
        connection
            .pragma_query_value(None, "user_version", |row| row.get::<_, u32>(0))
            .unwrap(),
        2
    );
}

#[test]
fn edits_keep_identity_and_only_changed_records_advance_revision() {
    let mut fixture = Fixture::new();
    let original = fixture.populate();
    let unchanged = fixture
        .service
        .save_work(save_input(&original, Some(original.notes[0].text.clone())))
        .unwrap();
    assert_eq!(unchanged, original);
    let workspace = fixture
        .service
        .update_project(UpdateProjectInput {
            expected_generation: original.generation.clone(),
            id: original.projects[0].id.clone(),
            title: "Revised synthetic handbook".into(),
        })
        .unwrap();
    assert_eq!(workspace.projects[0].revision, 2);
    assert_eq!(workspace.projects[0].id, original.projects[0].id);
    assert_eq!(
        workspace.projects[0].created_at,
        original.projects[0].created_at
    );
    assert_eq!(workspace.objectives, original.objectives);
    let workspace = fixture
        .service
        .update_objective(UpdateObjectiveInput {
            expected_generation: workspace.generation,
            id: original.objectives[0].id.clone(),
            title: original.objectives[0].title.clone(),
            criterion: "The complete example has four visible steps.".into(),
        })
        .unwrap();
    assert_eq!(workspace.objectives[0].revision, 2);
    assert_eq!(workspace.objectives[0].id, original.objectives[0].id);
    assert_eq!(
        workspace.objectives[0].created_at,
        original.objectives[0].created_at
    );
    let mut input = save_input(
        &workspace,
        Some("Step two now has a complete input.".into()),
    );
    input.stop_condition = "Four complete steps are present.".into();
    let updated = fixture.service.save_work(input).unwrap();
    assert_eq!(updated.actions[0].id, original.actions[0].id);
    assert_eq!(updated.notes[0].id, original.notes[0].id);
    assert_eq!(updated.actions[0].revision, 2);
    assert_eq!(updated.notes[0].revision, 2);
    assert_eq!(
        updated.actions[0].created_at,
        original.actions[0].created_at
    );
    assert_eq!(updated.notes[0].created_at, original.notes[0].created_at);
}

#[test]
fn note_clear_and_recreate_is_explicit_and_selection_is_durable() {
    let mut fixture = Fixture::new();
    let original = fixture.populate();
    let cleared = fixture
        .service
        .save_work(save_input(&original, Some(" \n\t ".into())))
        .unwrap();
    assert!(cleared.notes.is_empty());
    assert_eq!(cleared.actions, original.actions);
    let recreated = fixture
        .service
        .save_work(save_input(&cleared, Some("A new resumption note.".into())))
        .unwrap();
    assert_ne!(recreated.notes[0].id, original.notes[0].id);
    assert_eq!(recreated.notes[0].revision, 1);
    let deselected = fixture
        .service
        .select_objective(SelectObjectiveInput {
            expected_generation: recreated.generation,
            id: None,
        })
        .unwrap();
    assert!(deselected.selected_objective_id.is_none());
    assert_eq!(
        fixture
            .service
            .select_objective(SelectObjectiveInput {
                expected_generation: deselected.generation.clone(),
                id: None
            })
            .unwrap(),
        deselected
    );
    drop(fixture.service);
    let mut reopened = WorkspaceService::open(&fixture.database, &fixture.backups).unwrap();
    assert_eq!(reopened.load_workspace().unwrap(), deselected);
    let selected = reopened
        .select_objective(SelectObjectiveInput {
            expected_generation: deselected.generation,
            id: Some(original.objectives[0].id.clone()),
        })
        .unwrap();
    assert_eq!(
        selected.selected_objective_id,
        Some(original.objectives[0].id.clone())
    );
}

#[test]
fn separate_connections_reject_stale_writes_without_overwriting_new_state() {
    let mut fixture = Fixture::new();
    let old = fixture.populate();
    let mut second = WorkspaceService::open(&fixture.database, &fixture.backups).unwrap();
    assert_eq!(second.load_workspace().unwrap(), old);
    let current = fixture
        .service
        .update_project(UpdateProjectInput {
            expected_generation: old.generation.clone(),
            id: old.projects[0].id.clone(),
            title: "Saved by the first connection".into(),
        })
        .unwrap();
    let error = second
        .update_project(UpdateProjectInput {
            expected_generation: old.generation,
            id: old.projects[0].id.clone(),
            title: "Stale second connection input".into(),
        })
        .unwrap_err();
    assert_eq!(error.code, "conflict");
    assert_eq!(second.load_workspace().unwrap(), current);
}

#[test]
fn invalid_mutations_preserve_state_and_unicode_limits_count_scalar_values() {
    let mut fixture = Fixture::new();
    let initial = fixture.service.load_workspace().unwrap();
    let unicode_title = "🌱".repeat(120);
    let workspace = fixture
        .service
        .create_project(CreateProjectInput {
            expected_generation: initial.generation,
            title: unicode_title.clone(),
        })
        .unwrap();
    assert_eq!(workspace.projects[0].title, unicode_title);
    for title in [" \n ".into(), "a\0b".into(), "🌱".repeat(121)] {
        let error = fixture
            .service
            .update_project(UpdateProjectInput {
                expected_generation: workspace.generation.clone(),
                id: workspace.projects[0].id.clone(),
                title,
            })
            .unwrap_err();
        assert_eq!(error.code, "invalid_input");
        assert_eq!(fixture.service.load_workspace().unwrap(), workspace);
    }
    assert!(
        fixture
            .service
            .create_objective(CreateObjectiveInput {
                expected_generation: workspace.generation.clone(),
                project_id: uuid::Uuid::new_v4().to_string(),
                title: "Synthetic objective".into(),
                criterion: "One visible output exists.".into(),
            })
            .is_err()
    );
    assert_eq!(fixture.service.load_workspace().unwrap(), workspace);
}

#[test]
fn exact_fixture_restore_roundtrip_replay_and_reordered_arrays_preserve_identity() {
    let mut fixture = Fixture::new();
    let original = fixture.populate();
    let original_bytes = fixture.service.export_bytes().unwrap();
    let restored = fixture.restore(SAMPLE);
    assert!(restored.backup_created && !restored.unchanged);
    assert_ne!(restored.workspace.generation, original.generation);
    let expected: RecordSet = serde_json::from_slice(SAMPLE).unwrap();
    assert_eq!(RecordSet::from(&restored.workspace), expected);
    let backups = fixture.backup_paths();
    assert_eq!(backups.len(), 1);
    assert_eq!(fs::read(&backups[0]).unwrap(), original_bytes);
    let exported = fixture.service.export_bytes().unwrap();
    assert!(
        !String::from_utf8(exported.clone())
            .unwrap()
            .contains("generation")
    );
    let replayed = fixture.restore(&exported);
    assert!(replayed.unchanged && !replayed.backup_created);
    assert_eq!(replayed.workspace, restored.workspace);
    let mut reordered: RecordSet = serde_json::from_slice(SAMPLE).unwrap();
    reordered.projects.reverse();
    reordered.objectives.reverse();
    reordered.actions.reverse();
    reordered.notes.reverse();
    let replayed = fixture.restore(&serde_json::to_vec(&reordered).unwrap());
    assert_eq!(replayed.workspace, restored.workspace);
    assert_eq!(fixture.backup_paths().len(), 1);
    let mut clean = Fixture::new();
    assert_eq!(
        RecordSet::from(&clean.restore(&exported).workspace),
        expected
    );
}

#[test]
fn empty_restore_is_explicit_and_backup_can_restore_previous_records() {
    let mut fixture = Fixture::new();
    let original = fixture.populate();
    let empty = fixture.restore(EMPTY);
    assert!(empty.workspace.projects.is_empty());
    assert!(empty.workspace.selected_objective_id.is_none());
    let backup = fs::read(&fixture.backup_paths()[0]).unwrap();
    let recovered = fixture.restore(&backup);
    assert_eq!(
        RecordSet::from(&recovered.workspace),
        RecordSet::from(&original)
    );
}

#[test]
fn malformed_and_inconsistent_imports_do_not_change_any_saved_state() {
    let mut fixture = Fixture::new();
    let original = fixture.populate();
    let sample: Value = serde_json::from_slice(SAMPLE).unwrap();
    let mut variants = vec![
        b"not json".to_vec(),
        b"{\"format\":".to_vec(),
        vec![0xff],
        vec![b' '; MAX_IMPORT_BYTES + 1],
    ];
    let changes: Vec<fn(&mut Value)> = vec![
        |v| {
            v["schema_version"] = json!(99);
        },
        |v| {
            v["format"] = json!("other-format");
        },
        |v| {
            v["projects"][1]["id"] = v["projects"][0]["id"].clone();
        },
        |v| {
            v["actions"][0]["id"] = v["projects"][0]["id"].clone();
        },
        |v| {
            v["objectives"][0]["project_id"] = json!(uuid::Uuid::new_v4().to_string());
        },
        |v| {
            v["actions"][1]["objective_id"] = v["actions"][0]["objective_id"].clone();
        },
        |v| {
            v["notes"][1]["objective_id"] = v["notes"][0]["objective_id"].clone();
        },
        |v| {
            v["notes"][0]["objective_id"] = json!(uuid::Uuid::new_v4().to_string());
        },
        |v| {
            v["selected_objective_id"] = json!(uuid::Uuid::new_v4().to_string());
        },
        |v| {
            v.as_object_mut().unwrap().remove("selected_objective_id");
        },
        |v| {
            v["unknown_field"] = json!(true);
        },
        |v| {
            v["projects"][0]["unknown_field"] = json!(true);
        },
        |v| {
            v["projects"][0]["revision"] = json!(0);
        },
        |v| {
            v["projects"][0]["revision"] = json!(4_294_967_296_u64);
        },
        |v| {
            v["projects"][0]["created_at"] = json!("2026-10-16T12:00:00+00:00");
        },
        |v| {
            v["projects"][0]["id"] = json!("invalid-id");
        },
        |v| {
            v["objectives"][0]["criterion"] = json!("  ");
        },
        |v| {
            v["notes"][0]["text"] = json!("x".repeat(8_001));
        },
    ];
    for change in changes {
        let mut value = sample.clone();
        change(&mut value);
        variants.push(serde_json::to_vec(&value).unwrap());
    }
    variants.push(
        String::from_utf8(EMPTY.to_vec())
            .unwrap()
            .replacen(
                "\"schema_version\": 1",
                "\"schema_version\": 1, \"schema_version\": 1",
                1,
            )
            .into_bytes(),
    );
    for bytes in variants {
        let error = fixture
            .service
            .preview_import(&bytes, "invalid.json".into())
            .unwrap_err();
        assert!(["invalid_import", "unsupported_schema"].contains(&error.code.as_str()));
        assert_eq!(fixture.service.load_workspace().unwrap(), original);
        assert!(fixture.backup_paths().is_empty());
    }
}

#[test]
fn previews_expire_on_supersession_failure_success_or_workspace_change() {
    let mut fixture = Fixture::new();
    let original = fixture.populate();
    let first = fixture
        .service
        .preview_import(SAMPLE, "first.json".into())
        .unwrap();
    let second = fixture
        .service
        .preview_import(EMPTY, "second.json".into())
        .unwrap();
    assert_eq!(
        fixture
            .service
            .restore_import(RestoreInput {
                token: first.token,
                expected_generation: first.generation
            })
            .unwrap_err()
            .code,
        "conflict"
    );
    fixture
        .service
        .preview_import(b"invalid", "invalid.json".into())
        .unwrap_err();
    assert!(
        fixture
            .service
            .restore_import(RestoreInput {
                token: second.token,
                expected_generation: second.generation
            })
            .is_err()
    );
    let stale = fixture
        .service
        .preview_import(SAMPLE, "stale.json".into())
        .unwrap();
    let current = fixture
        .service
        .update_project(UpdateProjectInput {
            expected_generation: original.generation,
            id: original.projects[0].id.clone(),
            title: "A saved intervening edit".into(),
        })
        .unwrap();
    assert_eq!(
        fixture
            .service
            .restore_import(RestoreInput {
                token: stale.token,
                expected_generation: stale.generation
            })
            .unwrap_err()
            .code,
        "conflict"
    );
    assert_eq!(fixture.service.load_workspace().unwrap(), current);
    let preview = fixture
        .service
        .preview_import(SAMPLE, "valid.json".into())
        .unwrap();
    let input = RestoreInput {
        token: preview.token,
        expected_generation: preview.generation,
    };
    fixture.service.restore_import(input.clone()).unwrap();
    assert!(fixture.service.restore_import(input).is_err());
}

#[test]
fn preview_retains_exact_validated_bytes_and_cross_connection_restore_is_stale() {
    let mut fixture = Fixture::new();
    let original = fixture.populate();
    let mut bytes = SAMPLE.to_vec();
    let preview = fixture
        .service
        .preview_import(&bytes, "chosen.json".into())
        .unwrap();
    bytes.fill(b' ');
    let restored = fixture
        .service
        .restore_import(RestoreInput {
            token: preview.token,
            expected_generation: preview.generation,
        })
        .unwrap();
    assert_eq!(
        RecordSet::from(&restored.workspace),
        serde_json::from_slice::<RecordSet>(SAMPLE).unwrap()
    );
    let stale = fixture
        .service
        .preview_import(EMPTY, "empty.json".into())
        .unwrap();
    let mut second = WorkspaceService::open(&fixture.database, &fixture.backups).unwrap();
    let changed = second
        .create_project(CreateProjectInput {
            expected_generation: restored.workspace.generation,
            title: original.projects[0].title.clone(),
        })
        .unwrap();
    assert_eq!(
        fixture
            .service
            .restore_import(RestoreInput {
                token: stale.token,
                expected_generation: stale.generation
            })
            .unwrap_err()
            .code,
        "conflict"
    );
    assert_eq!(fixture.service.load_workspace().unwrap(), changed);
}

#[test]
fn backup_failure_and_transaction_failure_preserve_original_workspace() {
    let mut fixture = Fixture::new();
    let original = fixture.populate();
    fs::write(
        &fixture.backups,
        b"synthetic file blocks directory creation",
    )
    .unwrap();
    let preview = fixture
        .service
        .preview_import(SAMPLE, "sample.json".into())
        .unwrap();
    let input = RestoreInput {
        token: preview.token,
        expected_generation: preview.generation,
    };
    assert_eq!(
        fixture
            .service
            .restore_import(input.clone())
            .unwrap_err()
            .code,
        "backup_failed"
    );
    assert_eq!(fixture.service.load_workspace().unwrap(), original);
    fs::remove_file(&fixture.backups).unwrap();
    let connection = Connection::open(&fixture.database).unwrap();
    connection.execute_batch("CREATE TRIGGER reject_project_insert BEFORE INSERT ON projects BEGIN SELECT RAISE(ABORT, 'synthetic failure'); END;").unwrap();
    let expected_backup = fixture.service.export_bytes().unwrap();
    assert_eq!(
        fixture.service.restore_import(input).unwrap_err().code,
        "storage_error"
    );
    assert_eq!(fixture.service.load_workspace().unwrap(), original);
    assert_eq!(
        fs::read(&fixture.backup_paths()[0]).unwrap(),
        expected_backup
    );
    connection.execute_batch("CREATE TRIGGER reject_project_update BEFORE UPDATE ON projects BEGIN SELECT RAISE(ABORT, 'synthetic failure'); END;").unwrap();
    assert_eq!(
        fixture
            .service
            .update_project(UpdateProjectInput {
                expected_generation: original.generation.clone(),
                id: original.projects[0].id.clone(),
                title: "An edit which must roll back".into(),
            })
            .unwrap_err()
            .code,
        "storage_error"
    );
    assert_eq!(fixture.service.load_workspace().unwrap(), original);
    connection
        .execute_batch("DROP TRIGGER reject_project_insert; DROP TRIGGER reject_project_update;")
        .unwrap();
    assert!(!fixture.restore(SAMPLE).unchanged);
}

#[test]
fn revision_overflow_is_controlled_and_clock_rollback_is_valid_import_data() {
    let mut fixture = Fixture::new();
    let mut records: RecordSet = serde_json::from_slice(SAMPLE).unwrap();
    records.projects[0].revision = u32::MAX;
    records.projects[0].updated_at = "2020-01-01T00:00:00.000Z".into();
    let workspace = fixture
        .restore(&serde_json::to_vec(&records).unwrap())
        .workspace;
    assert_eq!(workspace.projects[0].updated_at, "2020-01-01T00:00:00.000Z");
    let error = fixture
        .service
        .update_project(UpdateProjectInput {
            expected_generation: workspace.generation.clone(),
            id: workspace.projects[0].id.clone(),
            title: "Revision overflow attempt".into(),
        })
        .unwrap_err();
    assert_eq!(error.code, "invalid_input");
    assert_eq!(fixture.service.load_workspace().unwrap(), workspace);
}

#[test]
fn input_deserialization_rejects_unknown_fields_and_requires_explicit_nullables() {
    let generation = uuid::Uuid::new_v4().to_string();
    assert!(
        serde_json::from_value::<CreateProjectInput>(
            json!({"expected_generation": generation, "title": "Example", "sql": "unused"})
        )
        .is_err()
    );
    assert!(
        serde_json::from_value::<SelectObjectiveInput>(json!({"expected_generation": generation}))
            .is_err()
    );
    assert!(serde_json::from_value::<SaveWorkInput>(json!({"expected_generation": generation, "objective_id": generation, "description": "Draft", "stop_condition": "One example"})).is_err());
}

#[test]
fn record_count_limit_rejects_oversized_collection_without_changing_state() {
    let mut fixture = Fixture::new();
    let original = fixture.populate();
    let mut records: RecordSet = serde_json::from_slice(EMPTY).unwrap();
    for _ in 0..=MAX_RECORDS {
        records.projects.push(Project {
            id: uuid::Uuid::new_v4().to_string(),
            title: "Synthetic project".into(),
            revision: 1,
            created_at: "2026-10-16T12:00:00.000Z".into(),
            updated_at: "2026-10-16T12:00:00.000Z".into(),
        });
    }
    let bytes = serde_json::to_vec(&records).unwrap();
    assert!(bytes.len() < MAX_IMPORT_BYTES);
    assert_eq!(
        fixture
            .service
            .preview_import(&bytes, "too-many.json".into())
            .unwrap_err()
            .code,
        "invalid_import"
    );
    assert_eq!(fixture.service.load_workspace().unwrap(), original);
}

#[test]
fn every_saved_workspace_remains_within_the_reimportable_export_size_limit() {
    let mut fixture = Fixture::new();
    let sample: RecordSet = serde_json::from_slice(SAMPLE).unwrap();
    let mut records: RecordSet = serde_json::from_slice(EMPTY).unwrap();
    records.projects.push(sample.projects[0].clone());
    for _ in 0..1_300 {
        let mut objective = sample.objectives[0].clone();
        objective.id = uuid::Uuid::new_v4().to_string();
        objective.project_id = records.projects[0].id.clone();
        let mut note = sample.notes[0].clone();
        note.id = uuid::Uuid::new_v4().to_string();
        note.objective_id = objective.id.clone();
        note.text = "x".into();
        records.objectives.push(objective);
        records.notes.push(note);
    }
    let target_bytes = MAX_IMPORT_BYTES - 16;
    let mut remaining = target_bytes - serde_json::to_vec_pretty(&records).unwrap().len();
    for note in &mut records.notes {
        let additional = remaining.min(7_999);
        note.text.push_str(&"x".repeat(additional));
        remaining -= additional;
    }
    assert_eq!(remaining, 0);
    records.normalize();
    let bytes = serde_json::to_vec_pretty(&records).unwrap();
    assert_eq!(bytes.len(), target_bytes);
    let original = fixture.restore(&bytes).workspace;
    assert_eq!(fixture.service.export_bytes().unwrap().len(), target_bytes);
    let error = fixture
        .service
        .create_project(CreateProjectInput {
            expected_generation: original.generation.clone(),
            title: "One more synthetic project exceeds the file size budget".into(),
        })
        .unwrap_err();
    assert_eq!(error.code, "invalid_input");
    assert_eq!(fixture.service.load_workspace().unwrap(), original);
    assert!(fixture.restore(&bytes).unchanged);
}

fn seed_v1(path: &Path) -> RecordSet {
    let connection = Connection::open(path).unwrap();
    connection
        .execute_batch(include_str!("../migrations/001_records.sql"))
        .unwrap();
    connection.pragma_update(None, "user_version", 1).unwrap();
    let mut records: RecordSet = serde_json::from_slice(SAMPLE).unwrap();
    records.selected_objective_id = None;
    for p in &records.projects {
        connection
            .execute(
                "INSERT INTO projects VALUES (?1,?2,?3,?4,?5)",
                params![p.id, p.title, p.revision, p.created_at, p.updated_at],
            )
            .unwrap();
    }
    for o in &records.objectives {
        connection
            .execute(
                "INSERT INTO objectives VALUES (?1,?2,?3,?4,?5,?6,?7)",
                params![
                    o.id,
                    o.project_id,
                    o.title,
                    o.criterion,
                    o.revision,
                    o.created_at,
                    o.updated_at
                ],
            )
            .unwrap();
    }
    for a in &records.actions {
        connection
            .execute(
                "INSERT INTO actions VALUES (?1,?2,?3,?4,?5,?6,?7)",
                params![
                    a.id,
                    a.objective_id,
                    a.description,
                    a.stop_condition,
                    a.revision,
                    a.created_at,
                    a.updated_at
                ],
            )
            .unwrap();
    }
    for n in &records.notes {
        connection
            .execute(
                "INSERT INTO notes VALUES (?1,?2,?3,?4,?5,?6)",
                params![
                    n.id,
                    n.objective_id,
                    n.text,
                    n.revision,
                    n.created_at,
                    n.updated_at
                ],
            )
            .unwrap();
    }
    records
}

#[test]
fn populated_v1_migration_preserves_records_and_creates_readable_sqlite_backup() {
    let directory = tempfile::tempdir().unwrap();
    let database = directory.path().join("v1.sqlite3");
    let backups = directory.path().join("backups");
    let expected = seed_v1(&database);
    let service = WorkspaceService::open(&database, &backups).unwrap();
    assert_eq!(
        RecordSet::from(&service.load_workspace().unwrap()),
        expected
    );
    let backup_paths: Vec<_> = fs::read_dir(&backups)
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .collect();
    assert_eq!(backup_paths.len(), 1);
    let backup = Connection::open(&backup_paths[0]).unwrap();
    assert_eq!(
        backup
            .pragma_query_value(None, "user_version", |row| row.get::<_, u32>(0))
            .unwrap(),
        1
    );
    assert_eq!(
        backup
            .query_row("SELECT COUNT(*) FROM projects", [], |row| row
                .get::<_, u32>(0))
            .unwrap(),
        2
    );
    let sample = &expected.notes[0];
    let row: (String, u32, String, String) = backup
        .query_row(
            "SELECT text,revision,created_at,updated_at FROM notes WHERE id=?1",
            [&sample.id],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
        )
        .unwrap();
    assert_eq!(
        row,
        (
            sample.text.clone(),
            sample.revision,
            sample.created_at.clone(),
            sample.updated_at.clone()
        )
    );
    assert_eq!(
        backup
            .query_row("PRAGMA integrity_check", [], |row| row.get::<_, String>(0))
            .unwrap(),
        "ok"
    );
}

#[test]
fn migration_backup_includes_committed_wal_changes_with_a_live_connection() {
    let directory = tempfile::tempdir().unwrap();
    let database = directory.path().join("wal-v1.sqlite3");
    let backups = directory.path().join("backups");
    let original = seed_v1(&database);
    let writer = Connection::open(&database).unwrap();
    writer.pragma_update(None, "journal_mode", "WAL").unwrap();
    writer.pragma_update(None, "wal_autocheckpoint", 0).unwrap();
    let updated_note = "Synthetic edit committed in the WAL before migration.";
    writer
        .execute(
            "UPDATE notes SET text=?1,revision=revision+1 WHERE id=?2",
            params![updated_note, original.notes[0].id],
        )
        .unwrap();
    let wal_path = PathBuf::from(format!("{}-wal", database.display()));
    assert!(fs::metadata(wal_path).unwrap().len() > 0);
    let service = WorkspaceService::open(&database, &backups).unwrap();
    let workspace = service.load_workspace().unwrap();
    let note = workspace
        .notes
        .iter()
        .find(|note| note.id == original.notes[0].id)
        .unwrap();
    assert_eq!(note.text, updated_note);
    assert_eq!(note.revision, original.notes[0].revision + 1);
    let paths: Vec<_> = fs::read_dir(backups)
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .collect();
    assert_eq!(paths.len(), 1);
    let backup = Connection::open(&paths[0]).unwrap();
    assert_eq!(
        backup
            .pragma_query_value(None, "user_version", |row| row.get::<_, u32>(0))
            .unwrap(),
        1
    );
    let saved: (String, u32) = backup
        .query_row(
            "SELECT text,revision FROM notes WHERE id=?1",
            [&original.notes[0].id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .unwrap();
    assert_eq!(saved, (updated_note.into(), original.notes[0].revision + 1));
    assert_eq!(
        backup
            .query_row("PRAGMA integrity_check", [], |row| row.get::<_, String>(0))
            .unwrap(),
        "ok"
    );
}

#[test]
fn migration_failure_rolls_back_and_backup_failure_does_not_start_migration() {
    for fail_backup in [false, true] {
        let directory = tempfile::tempdir().unwrap();
        let database = directory.path().join("v1.sqlite3");
        let backups = directory.path().join("backups");
        let records = seed_v1(&database);
        let connection = Connection::open(&database).unwrap();
        if fail_backup {
            fs::write(&backups, b"synthetic blocker").unwrap();
        } else {
            connection
                .execute_batch("CREATE TABLE workspace_state (synthetic_marker TEXT);")
                .unwrap();
        }
        let result = WorkspaceService::open(&database, &backups);
        assert!(result.is_err());
        assert_eq!(
            connection
                .pragma_query_value(None, "user_version", |row| row.get::<_, u32>(0))
                .unwrap(),
            1
        );
        assert_eq!(
            connection
                .query_row(
                    "SELECT title FROM projects WHERE id=?1",
                    [&records.projects[0].id],
                    |row| row.get::<_, String>(0)
                )
                .unwrap(),
            records.projects[0].title
        );
        assert_eq!(
            connection
                .query_row("SELECT COUNT(*) FROM notes", [], |row| row.get::<_, u32>(0))
                .unwrap(),
            2
        );
    }
}

#[test]
fn invalid_legacy_record_rolls_back_schema_upgrade_and_keeps_readable_backup() {
    let directory = tempfile::tempdir().unwrap();
    let database = directory.path().join("v1.sqlite3");
    let backups = directory.path().join("backups");
    seed_v1(&database);
    let connection = Connection::open(&database).unwrap();
    connection
        .execute("UPDATE projects SET title='   '", [])
        .unwrap();
    assert!(WorkspaceService::open(&database, &backups).is_err());
    assert_eq!(
        connection
            .pragma_query_value(None, "user_version", |row| row.get::<_, u32>(0))
            .unwrap(),
        1
    );
    assert_eq!(
        connection
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE name='workspace_state'",
                [],
                |row| row.get::<_, u32>(0)
            )
            .unwrap(),
        0
    );
    assert_eq!(
        connection
            .query_row("SELECT COUNT(*) FROM notes", [], |row| row.get::<_, u32>(0))
            .unwrap(),
        2
    );
    let backups: Vec<_> = fs::read_dir(backups)
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .collect();
    assert_eq!(backups.len(), 1);
    let backup = Connection::open(&backups[0]).unwrap();
    assert_eq!(
        backup
            .query_row("PRAGMA integrity_check", [], |row| row.get::<_, String>(0))
            .unwrap(),
        "ok"
    );
    assert_eq!(
        backup
            .pragma_query_value(None, "user_version", |row| row.get::<_, u32>(0))
            .unwrap(),
        1
    );
}

#[test]
fn future_unknown_and_corrupt_databases_are_rejected_without_recreation() {
    let directory = tempfile::tempdir().unwrap();
    let backups = directory.path().join("backups");
    let future = directory.path().join("future.sqlite3");
    {
        let connection = Connection::open(&future).unwrap();
        connection.execute_batch("CREATE TABLE synthetic_marker(value TEXT); INSERT INTO synthetic_marker VALUES ('preserve'); PRAGMA user_version=99;").unwrap();
    }
    let original = fs::read(&future).unwrap();
    let error = WorkspaceService::open(&future, &backups).err().unwrap();
    assert_eq!(error.code, "unsupported_schema");
    assert_eq!(fs::read(&future).unwrap(), original);
    let unknown = directory.path().join("unknown.sqlite3");
    {
        let connection = Connection::open(&unknown).unwrap();
        connection
            .execute_batch("CREATE TABLE synthetic_marker(value TEXT);")
            .unwrap();
    }
    let original = fs::read(&unknown).unwrap();
    assert!(WorkspaceService::open(&unknown, &backups).is_err());
    assert_eq!(fs::read(&unknown).unwrap(), original);
    let views_only = directory.path().join("views-only.sqlite3");
    {
        let connection = Connection::open(&views_only).unwrap();
        connection
            .execute_batch("CREATE VIEW synthetic_marker AS SELECT 'preserve' AS value;")
            .unwrap();
    }
    let original = fs::read(&views_only).unwrap();
    assert!(WorkspaceService::open(&views_only, &backups).is_err());
    assert_eq!(fs::read(&views_only).unwrap(), original);
    let corrupt = directory.path().join("corrupt.sqlite3");
    fs::write(&corrupt, b"synthetic invalid sqlite bytes").unwrap();
    assert!(WorkspaceService::open(&corrupt, &backups).is_err());
    assert_eq!(
        fs::read(&corrupt).unwrap(),
        b"synthetic invalid sqlite bytes"
    );
}

#[test]
fn sqlite_constraints_reject_broken_relationships_duplicates_and_invalid_revisions() {
    let mut fixture = Fixture::new();
    let original = fixture.populate();
    let connection = Connection::open(&fixture.database).unwrap();
    connection
        .pragma_update(None, "foreign_keys", true)
        .unwrap();
    let statements = [
        "DELETE FROM projects",
        "DELETE FROM objectives",
        "UPDATE objectives SET project_id='missing-project'",
        "UPDATE actions SET objective_id='missing-objective'",
        "UPDATE notes SET objective_id='missing-objective'",
        "UPDATE workspace_state SET selected_objective_id='missing-objective'",
        "INSERT INTO projects SELECT * FROM projects",
        "INSERT INTO actions SELECT 'duplicate-action',objective_id,description,stop_condition,revision,created_at,updated_at FROM actions",
        "INSERT INTO notes SELECT 'duplicate-note',objective_id,text,revision,created_at,updated_at FROM notes",
        "UPDATE projects SET revision=0",
        "UPDATE objectives SET revision=-1",
        "UPDATE actions SET revision=4294967296",
        "UPDATE notes SET revision=NULL",
        "INSERT INTO workspace_state VALUES (2,'another-generation',NULL)",
    ];
    for statement in statements {
        assert_eq!(
            connection
                .execute(statement, [])
                .unwrap_err()
                .sqlite_error_code(),
            Some(rusqlite::ErrorCode::ConstraintViolation),
            "{statement}"
        );
        assert_eq!(fixture.service.load_workspace().unwrap(), original);
    }
    assert_eq!(
        connection
            .query_row("PRAGMA integrity_check", [], |row| row.get::<_, String>(0))
            .unwrap(),
        "ok"
    );
    assert_eq!(
        connection
            .query_row("SELECT COUNT(*) FROM pragma_foreign_key_check", [], |row| {
                row.get::<_, u32>(0)
            })
            .unwrap(),
        0
    );
}

#[test]
fn simultaneous_writers_commit_exactly_one_generation_and_preserve_the_winner() {
    let mut fixture = Fixture::new();
    let original = fixture.populate();
    let barrier = Arc::new(Barrier::new(3));
    let handles: Vec<_> = (0..2)
        .map(|index| {
            let database = fixture.database.clone();
            let backups = fixture.backups.clone();
            let generation = original.generation.clone();
            let project_id = original.projects[0].id.clone();
            let barrier = Arc::clone(&barrier);
            std::thread::spawn(move || {
                let mut service = WorkspaceService::open(&database, &backups).unwrap();
                barrier.wait();
                service.update_project(UpdateProjectInput {
                    expected_generation: generation,
                    id: project_id,
                    title: format!("Synthetic concurrent edit {index}"),
                })
            })
        })
        .collect();
    barrier.wait();
    let results: Vec<_> = handles
        .into_iter()
        .map(|handle| handle.join().unwrap())
        .collect();
    let successes: Vec<_> = results
        .iter()
        .filter_map(|result| result.as_ref().ok())
        .collect();
    let failures: Vec<_> = results
        .iter()
        .filter_map(|result| result.as_ref().err())
        .collect();
    assert_eq!(successes.len(), 1);
    assert_eq!(failures.len(), 1);
    assert_eq!(failures[0].code, "conflict");
    let winner = successes[0];
    assert_eq!(
        winner.projects[0].revision,
        original.projects[0].revision + 1
    );
    assert_eq!(winner.actions, original.actions);
    assert_eq!(winner.notes, original.notes);
    assert_eq!(&fixture.service.load_workspace().unwrap(), winner);
    let reopened = WorkspaceService::open(&fixture.database, &fixture.backups).unwrap();
    assert_eq!(&reopened.load_workspace().unwrap(), winner);
}

#[test]
fn action_and_note_save_rolls_back_together_on_late_validation_or_sql_failure() {
    for fail_in_sql in [false, true] {
        let mut fixture = Fixture::new();
        let mut records: RecordSet = serde_json::from_slice(SAMPLE).unwrap();
        if !fail_in_sql {
            records.notes[0].revision = u32::MAX;
        }
        let original = fixture
            .restore(&serde_json::to_vec(&records).unwrap())
            .workspace;
        let connection = Connection::open(&fixture.database).unwrap();
        if fail_in_sql {
            connection.execute_batch("CREATE TRIGGER reject_note_update BEFORE UPDATE ON notes BEGIN SELECT RAISE(ABORT, 'synthetic late failure'); END;").unwrap();
        }
        let mut input = save_input(&original, Some("Changed synthetic note".into()));
        input.description = "Changed synthetic action".into();
        input.stop_condition = "A changed stopping point is visible.".into();
        let error = fixture.service.save_work(input).unwrap_err();
        assert_eq!(
            error.code,
            if fail_in_sql {
                "storage_error"
            } else {
                "invalid_input"
            }
        );
        assert_eq!(fixture.service.load_workspace().unwrap(), original);
        drop(connection);
        drop(fixture.service);
        let reopened = WorkspaceService::open(&fixture.database, &fixture.backups).unwrap();
        assert_eq!(reopened.load_workspace().unwrap(), original);
    }
}

#[test]
fn explicit_restore_of_conflicting_local_ids_preserves_incoming_versions_and_recoverable_backup() {
    let mut fixture = Fixture::new();
    let old = fixture.populate();
    let old_export = fixture.service.export_bytes().unwrap();
    let new = fixture
        .service
        .update_project(UpdateProjectInput {
            expected_generation: old.generation.clone(),
            id: old.projects[0].id.clone(),
            title: "Newer local title for the same identity".into(),
        })
        .unwrap();
    let new_export = fixture.service.export_bytes().unwrap();
    let preview = fixture
        .service
        .preview_import(&old_export, "older-synthetic-export.json".into())
        .unwrap();
    assert_eq!(fixture.service.load_workspace().unwrap(), new);
    assert!(fixture.backup_paths().is_empty());
    let restored = fixture
        .service
        .restore_import(RestoreInput {
            token: preview.token,
            expected_generation: preview.generation,
        })
        .unwrap();
    assert!(restored.backup_created);
    assert_eq!(RecordSet::from(&restored.workspace), RecordSet::from(&old));
    let backup = fs::read(&fixture.backup_paths()[0]).unwrap();
    assert_eq!(backup, new_export);
    let recovered = fixture.restore(&backup);
    assert_eq!(RecordSet::from(&recovered.workspace), RecordSet::from(&new));
}

#[test]
fn valid_sqlite_with_corrupted_current_records_is_rejected_without_replacing_records() {
    let corruptions = [
        "UPDATE projects SET title='   '",
        "UPDATE objectives SET created_at='invalid-timestamp'",
        "UPDATE actions SET id=(SELECT id FROM projects LIMIT 1)",
        "UPDATE notes SET objective_id='missing-objective'",
        "UPDATE workspace_state SET generation='invalid-generation'",
        "UPDATE workspace_state SET selected_objective_id='missing-objective'",
        "DELETE FROM workspace_state",
        "PRAGMA ignore_check_constraints=ON; UPDATE notes SET revision=0",
    ];
    for corruption in corruptions {
        let mut fixture = Fixture::new();
        let original = fixture.populate();
        drop(fixture.service);
        let connection = Connection::open(&fixture.database).unwrap();
        // Simulate externally damaged storage; normal connections reject these writes.
        connection
            .pragma_update(None, "foreign_keys", false)
            .unwrap();
        connection.execute_batch(corruption).unwrap();
        let rows_before: Vec<(String, String, u32)> = connection
            .prepare("SELECT id,description,revision FROM actions ORDER BY id")
            .unwrap()
            .query_map([], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)))
            .unwrap()
            .collect::<rusqlite::Result<_>>()
            .unwrap();
        assert_eq!(
            WorkspaceService::open(&fixture.database, &fixture.backups)
                .err()
                .unwrap()
                .code,
            "storage_error",
            "{corruption}"
        );
        let rows_after: Vec<(String, String, u32)> = connection
            .prepare("SELECT id,description,revision FROM actions ORDER BY id")
            .unwrap()
            .query_map([], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)))
            .unwrap()
            .collect::<rusqlite::Result<_>>()
            .unwrap();
        assert_eq!(rows_after, rows_before);
        assert_eq!(rows_after[0].1, original.actions[0].description);
        assert_eq!(
            connection
                .pragma_query_value(None, "user_version", |row| row.get::<_, u32>(0))
                .unwrap(),
            2
        );
        assert!(!fixture.backups.exists());
    }
}

#[test]
fn migration_backup_recovers_as_a_complete_workspace_in_a_fresh_profile() {
    let directory = tempfile::tempdir().unwrap();
    let database = directory.path().join("legacy.sqlite3");
    let backups = directory.path().join("backups");
    let expected = seed_v1(&database);
    let migrated = WorkspaceService::open(&database, &backups).unwrap();
    assert_eq!(
        RecordSet::from(&migrated.load_workspace().unwrap()),
        expected
    );
    let backup = fs::read_dir(&backups)
        .unwrap()
        .next()
        .unwrap()
        .unwrap()
        .path();
    let recovery_database = directory.path().join("recovery.sqlite3");
    fs::copy(&backup, &recovery_database).unwrap();
    let recovered = WorkspaceService::open(
        &recovery_database,
        &directory.path().join("recovery-backups"),
    )
    .unwrap();
    assert_eq!(
        RecordSet::from(&recovered.load_workspace().unwrap()),
        expected
    );
    assert_eq!(
        recovered.export_bytes().unwrap(),
        migrated.export_bytes().unwrap()
    );
    let legacy_backup =
        Connection::open_with_flags(&backup, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY).unwrap();
    assert_eq!(
        legacy_backup
            .pragma_query_value(None, "user_version", |row| row.get::<_, u32>(0))
            .unwrap(),
        1
    );
}

#[test]
fn committed_wal_survives_process_exit_without_connection_cleanup() {
    const PROFILE_ENV: &str = "ASCENSION_TEST_ABRUPT_EXIT_PROFILE";
    if let Some(profile) = std::env::var_os(PROFILE_ENV) {
        // This subprocess intentionally exits without running the connection's
        // destructor, leaving committed records in SQLite's WAL for recovery.
        let profile = PathBuf::from(profile);
        let mut service =
            WorkspaceService::open(&profile.join("workspace.sqlite3"), &profile.join("backups"))
                .unwrap();
        let initial = service.load_workspace().unwrap();
        let project = service
            .create_project(CreateProjectInput {
                expected_generation: initial.generation,
                title: "Synthetic abrupt-exit project".into(),
            })
            .unwrap();
        let objective = service
            .create_objective(CreateObjectiveInput {
                expected_generation: project.generation,
                project_id: project.projects[0].id.clone(),
                title: "Recover a committed action".into(),
                criterion: "The same action and note appear after restarting.".into(),
            })
            .unwrap();
        let saved = service
            .save_work(SaveWorkInput {
                expected_generation: objective.generation,
                objective_id: objective.objectives[0].id.clone(),
                description: "Read the synthetic recovery example".into(),
                stop_condition: "The complete example is visible.".into(),
                note: Some("Continue from the saved second step.".into()),
            })
            .unwrap();
        fs::write(
            profile.join("expected-workspace.json"),
            serde_json::to_vec(&saved).unwrap(),
        )
        .unwrap();
        fs::write(
            profile.join("expected-export.json"),
            service.export_bytes().unwrap(),
        )
        .unwrap();
        std::process::exit(0);
    }
    let directory = tempfile::tempdir().unwrap();
    let output = std::process::Command::new(std::env::current_exe().unwrap())
        .args([
            "--exact",
            "committed_wal_survives_process_exit_without_connection_cleanup",
        ])
        .env(PROFILE_ENV, directory.path())
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "Synthetic writer failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        fs::metadata(directory.path().join("workspace.sqlite3-wal"))
            .unwrap()
            .len()
            > 0
    );
    let expected: Workspace = serde_json::from_slice(
        &fs::read(directory.path().join("expected-workspace.json")).unwrap(),
    )
    .unwrap();
    let reopened = WorkspaceService::open(
        &directory.path().join("workspace.sqlite3"),
        &directory.path().join("backups"),
    )
    .unwrap();
    assert_eq!(reopened.load_workspace().unwrap(), expected);
    assert_eq!(
        reopened.export_bytes().unwrap(),
        fs::read(directory.path().join("expected-export.json")).unwrap()
    );
}
