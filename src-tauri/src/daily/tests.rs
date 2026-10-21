use super::*;
use crate::service::WorkspaceService;
use std::path::PathBuf;

struct Fixture {
    service: WorkspaceService,
    database: PathBuf,
    backups: PathBuf,
    _directory: tempfile::TempDir,
}
impl Fixture {
    fn new() -> Self {
        let directory = tempfile::tempdir().unwrap();
        let database = directory.path().join("workspace.sqlite3");
        let backups = directory.path().join("backups");
        let mut service = WorkspaceService::open(&database, &backups).unwrap();
        let preview = service
            .preview_import(
                include_bytes!("../../../contracts/fixtures/sample-v1.json"),
                "synthetic.json".into(),
            )
            .unwrap();
        service
            .restore_import(RestoreInput {
                token: preview.token,
                expected_generation: preview.generation,
            })
            .unwrap();
        Self {
            service,
            database,
            backups,
            _directory: directory,
        }
    }
    fn workspace(&self) -> Workspace {
        self.service.load_workspace().unwrap()
    }
    fn input(&self) -> RecordCheckpointInput {
        let w = self.workspace();
        let a = &w.actions[0];
        RecordCheckpointInput {
            expected_generation: w.generation,
            command_id: new_id(),
            objective_id: a.objective_id.clone(),
            note: "Outlined the synthetic section; verification remains open.".into(),
            reference: Some("synthetic-outline.md".into()),
            description: a.description.clone(),
            stop_condition: a.stop_condition.clone(),
            resumption_note: Some("Resume at the second heading.".into()),
            supersedes: None,
            correction_reason: None,
        }
    }
    fn add_objective(&mut self) -> String {
        let w = self.workspace();
        self.service
            .create_objective(CreateObjectiveInput {
                expected_generation: w.generation,
                project_id: w.projects[0].id.clone(),
                title: "Another synthetic priority".into(),
                criterion: "A reviewed second section exists.".into(),
            })
            .unwrap()
            .selected_objective_id
            .unwrap()
    }
}

#[test]
fn lifecycle_focus_order_survive_restart_without_touching_authority() {
    let mut f = Fixture::new();
    let first = f.workspace().objectives[0].id.clone();
    let second = f.add_objective();
    let third = f.add_objective();
    let before = f.workspace();
    let mut order = vec![third.clone(), first.clone(), second.clone()];
    for objective in &before.objectives {
        if !order.contains(&objective.id) {
            order.push(objective.id.clone());
        }
    }
    let w = f
        .service
        .reorder_objectives(ReorderObjectivesInput {
            expected_generation: before.generation.clone(),
            objective_ids: order.clone(),
        })
        .unwrap();
    assert_eq!(w.progress, before.progress);
    assert_eq!(w.objectives, before.objectives);
    assert_eq!(
        f.service
            .reorder_objectives(ReorderObjectivesInput {
                expected_generation: before.generation,
                objective_ids: order
            })
            .unwrap_err()
            .code,
        "conflict"
    );
    let w = f
        .service
        .set_objective_status(SetObjectiveStatusInput {
            expected_generation: w.generation,
            objective_id: third.clone(),
            status: ObjectiveStatus::Archived,
        })
        .unwrap();
    assert_eq!(w.selected_objective_id, Some(first));
    assert_eq!(w.progress, before.progress);
    let w = f
        .service
        .set_objective_status(SetObjectiveStatusInput {
            expected_generation: w.generation,
            objective_id: second,
            status: ObjectiveStatus::Paused,
        })
        .unwrap();
    assert_eq!(
        f.service
            .select_objective(SelectObjectiveInput {
                expected_generation: w.generation.clone(),
                id: Some(third.clone())
            })
            .unwrap_err()
            .code,
        "invalid_input"
    );
    let restored = f
        .service
        .set_objective_status(SetObjectiveStatusInput {
            expected_generation: w.generation,
            objective_id: third,
            status: ObjectiveStatus::Active,
        })
        .unwrap();
    let reopened = WorkspaceService::open(&f.database, &f.backups)
        .unwrap()
        .load_workspace()
        .unwrap();
    assert_eq!(restored, reopened);
}

#[test]
fn unavailable_focus_falls_back_to_none_and_noop_preserves_generation() {
    let mut f = Fixture::new();
    let w = f.workspace();
    let id = w.objectives[0].id.clone();
    for objective in &w.objectives {
        f.service
            .set_objective_status(SetObjectiveStatusInput {
                expected_generation: f.workspace().generation,
                objective_id: objective.id.clone(),
                status: ObjectiveStatus::Paused,
            })
            .unwrap();
    }
    let paused = f.workspace();
    assert_eq!(paused.selected_objective_id, None);
    assert_eq!(
        f.service
            .set_objective_status(SetObjectiveStatusInput {
                expected_generation: paused.generation.clone(),
                objective_id: id,
                status: ObjectiveStatus::Paused
            })
            .unwrap(),
        paused
    );
}

#[test]
fn checkpoint_updates_action_atomically_and_replays_after_unrelated_edit() {
    let mut f = Fixture::new();
    let before = f.workspace();
    let mut input = f.input();
    input.description = "Review the second synthetic heading".into();
    let result = f.service.record_checkpoint(input.clone()).unwrap();
    assert_eq!(result.workspace.actions[0].id, before.actions[0].id);
    assert_eq!(
        result.workspace.actions[0].revision,
        before.actions[0].revision + 1
    );
    assert_eq!(result.workspace.progress, before.progress);
    assert_eq!(result.workspace.study, before.study);
    assert_eq!(result.workspace.world, before.world);
    assert_eq!(result.checkpoint.provenance, "self_report");
    assert_eq!(result.checkpoint.reference, input.reference);
    f.add_objective();
    let replay = f.service.record_checkpoint(input.clone()).unwrap();
    assert_eq!(replay.checkpoint, result.checkpoint);
    assert_eq!(replay.workspace.daily.checkpoints.len(), 1);
    input.note = "A changed claim under an already used identity".into();
    assert_eq!(
        f.service.record_checkpoint(input).unwrap_err().code,
        "idempotency_conflict"
    );
    assert_eq!(
        WorkspaceService::open(&f.database, &f.backups)
            .unwrap()
            .load_workspace()
            .unwrap(),
        replay.workspace
    );
}

#[test]
fn failure_after_action_and_checkpoint_insert_rolls_back_every_row() {
    let mut f = Fixture::new();
    let before = f.service.export_bytes().unwrap();
    let generation = f.workspace().generation;
    let c = Connection::open(&f.database).unwrap();
    c.execute_batch("CREATE TRIGGER synthetic_checkpoint_failure BEFORE INSERT ON checkpoint_commands BEGIN SELECT RAISE(ABORT,'synthetic failure'); END;").unwrap();
    let mut input = f.input();
    input.description = "Must roll back".into();
    input.resumption_note = None;
    assert_eq!(
        f.service.record_checkpoint(input.clone()).unwrap_err().code,
        "storage_error"
    );
    assert_eq!(f.service.export_bytes().unwrap(), before);
    assert_eq!(f.workspace().generation, generation);
    c.execute_batch("DROP TRIGGER synthetic_checkpoint_failure;")
        .unwrap();
    assert_eq!(
        f.service
            .record_checkpoint(input)
            .unwrap()
            .workspace
            .daily
            .checkpoints
            .len(),
        1
    );
}

#[test]
fn correction_is_append_only_and_requires_current_tip_and_reason() {
    let mut f = Fixture::new();
    let original = f.service.record_checkpoint(f.input()).unwrap().checkpoint;
    let mut correction = f.input();
    correction.supersedes = Some(original.id.clone());
    correction.correction_reason = Some("The first account overstated the draft.".into());
    correction.note = "Only the first heading was drafted.".into();
    let corrected = f.service.record_checkpoint(correction).unwrap();
    assert_eq!(corrected.workspace.daily.checkpoints[0], original);
    let mut branch = f.input();
    branch.supersedes = Some(original.id);
    branch.correction_reason = Some("Another correction".into());
    assert_eq!(
        f.service.record_checkpoint(branch).unwrap_err().code,
        "invalid_input"
    );
    let mut missing = f.input();
    missing.supersedes = Some(corrected.checkpoint.id);
    assert_eq!(
        f.service.record_checkpoint(missing).unwrap_err().code,
        "invalid_input"
    );
    let c = Connection::open(&f.database).unwrap();
    assert!(
        c.execute("UPDATE work_checkpoints SET payload=payload", [])
            .is_err()
    );
    assert!(
        c.execute("UPDATE checkpoint_commands SET payload=payload", [])
            .is_err()
    );
}

#[test]
fn complete_restore_preserves_daily_replay_and_rejects_unsafe_legacy() {
    let mut f = Fixture::new();
    let input = f.input();
    let result = f.service.record_checkpoint(input.clone()).unwrap();
    let bytes = f.service.export_bytes().unwrap();
    let records: RecordSet = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(records.schema_version, 5);
    let mut target = Fixture::new();
    let preview = target
        .service
        .preview_import(&bytes, "synthetic-v5.json".into())
        .unwrap();
    assert_eq!(preview.incoming_counts.checkpoints, 1);
    let restored = target
        .service
        .restore_import(RestoreInput {
            token: preview.token,
            expected_generation: preview.generation,
        })
        .unwrap();
    assert!(restored.backup_created);
    assert_eq!(restored.workspace.daily, result.workspace.daily);
    assert_eq!(restored.workspace.progress, result.workspace.progress);
    assert_eq!(
        target.service.record_checkpoint(input).unwrap().checkpoint,
        result.checkpoint
    );
    let mut legacy = records;
    legacy.schema_version = 4;
    legacy.daily = None;
    assert_eq!(
        target
            .service
            .preview_import(&serde_json::to_vec(&legacy).unwrap(), "old.json".into())
            .unwrap_err()
            .code,
        "unsafe_legacy_import"
    );
}

#[test]
fn edited_lifecycle_alone_blocks_legacy_and_modern_malformed_history_is_rejected() {
    let mut f = Fixture::new();
    let mut old: RecordSet = serde_json::from_slice(&f.service.export_bytes().unwrap()).unwrap();
    old.schema_version = 4;
    old.daily = None;
    let w = f.workspace();
    f.service
        .set_objective_status(SetObjectiveStatusInput {
            expected_generation: w.generation,
            objective_id: w.objectives[0].id.clone(),
            status: ObjectiveStatus::Archived,
        })
        .unwrap();
    assert_eq!(
        f.service
            .preview_import(&serde_json::to_vec(&old).unwrap(), "old.json".into())
            .unwrap_err()
            .code,
        "unsafe_legacy_import"
    );
    let before = f.service.export_bytes().unwrap();
    let mut current: RecordSet = serde_json::from_slice(&before).unwrap();
    current.daily.as_mut().unwrap().objectives[0].priority = 9;
    assert_eq!(
        f.service
            .preview_import(&serde_json::to_vec(&current).unwrap(), "bad.json".into())
            .unwrap_err()
            .code,
        "invalid_import"
    );
    assert_eq!(f.service.export_bytes().unwrap(), before);
}

#[test]
fn populated_v5_migration_preserves_ids_revisions_selected_focus_and_original_backup() {
    let f = Fixture::new();
    let w = f.workspace();
    let directory = tempfile::tempdir().unwrap();
    let database = directory.path().join("old.sqlite3");
    let backups = directory.path().join("backups");
    let source = Connection::open(&f.database).unwrap();
    source.backup(rusqlite::MAIN_DB, &database, None).unwrap();
    let old = Connection::open(&database).unwrap();
    old.execute_batch("PRAGMA foreign_keys=OFF; DROP TABLE checkpoint_commands; DROP TABLE work_checkpoints; DROP TABLE objective_planning; PRAGMA user_version=5;").unwrap();
    drop(old);
    let service = WorkspaceService::open(&database, &backups).unwrap();
    let migrated = service.load_workspace().unwrap();
    assert_eq!(migrated.generation, w.generation);
    assert_eq!(migrated.selected_objective_id, w.selected_objective_id);
    assert_eq!(migrated.objectives, w.objectives);
    assert_eq!(migrated.actions, w.actions);
    assert_eq!(migrated.progress, w.progress);
    assert_eq!(migrated.study, w.study);
    assert_eq!(migrated.world, w.world);
    assert_eq!(migrated.daily, from_legacy(&w.objectives));
    let backup = std::fs::read_dir(backups)
        .unwrap()
        .map(|v| v.unwrap().path())
        .find(|p| {
            p.file_name()
                .unwrap()
                .to_string_lossy()
                .starts_with("before-migration-")
        })
        .unwrap();
    assert_eq!(
        Connection::open(backup)
            .unwrap()
            .pragma_query_value(None, "user_version", |r| r.get::<_, u32>(0))
            .unwrap(),
        5
    );
}

#[test]
fn invalid_reorders_and_replayed_payload_corruption_leave_state_intact() {
    let mut f = Fixture::new();
    let w = f.workspace();
    assert_eq!(
        f.service
            .reorder_objectives(ReorderObjectivesInput {
                expected_generation: w.generation.clone(),
                objective_ids: vec![]
            })
            .unwrap_err()
            .code,
        "invalid_input"
    );
    assert_eq!(f.workspace(), w);
    f.service.record_checkpoint(f.input()).unwrap();
    let original = f.service.export_bytes().unwrap();
    let mut r: RecordSet = serde_json::from_slice(&original).unwrap();
    r.daily.as_mut().unwrap().commands[0].input.note = "Tampered payload".into();
    assert_eq!(
        f.service
            .preview_import(&serde_json::to_vec(&r).unwrap(), "bad.json".into())
            .unwrap_err()
            .code,
        "invalid_import"
    );
    assert_eq!(f.service.export_bytes().unwrap(), original);
}

#[test]
fn concurrent_reorders_reject_one_stale_writer_without_losing_priority() {
    use std::sync::{Arc, Barrier};
    let f = Fixture::new();
    let w = f.workspace();
    let mut order: Vec<_> = w
        .daily
        .objectives
        .iter()
        .map(|o| o.objective_id.clone())
        .collect();
    order.reverse();
    let barrier = Arc::new(Barrier::new(2));
    let handles: Vec<_> = (0..2)
        .map(|_| {
            let database = f.database.clone();
            let backups = f.backups.clone();
            let barrier = barrier.clone();
            let input = ReorderObjectivesInput {
                expected_generation: w.generation.clone(),
                objective_ids: order.clone(),
            };
            std::thread::spawn(move || {
                let mut service = WorkspaceService::open(&database, &backups).unwrap();
                barrier.wait();
                service.reorder_objectives(input)
            })
        })
        .collect();
    let results: Vec<_> = handles.into_iter().map(|h| h.join().unwrap()).collect();
    assert_eq!(results.iter().filter(|r| r.is_ok()).count(), 1);
    assert_eq!(
        results
            .iter()
            .filter_map(|r| r.as_ref().err())
            .next()
            .unwrap()
            .code,
        "conflict"
    );
    let after = f.workspace();
    let mut metadata = after.daily.objectives.clone();
    metadata.sort_by_key(|o| o.priority);
    assert_eq!(
        metadata
            .into_iter()
            .map(|o| o.objective_id)
            .collect::<Vec<_>>(),
        order
    );
    assert_eq!(after.progress, w.progress);
}

#[test]
fn concurrent_checkpoint_delivery_returns_one_stable_committed_result() {
    use std::sync::{Arc, Barrier};
    let f = Fixture::new();
    let before = f.workspace();
    let mut input = f.input();
    input.description = "One synthetic next action, despite concurrent delivery.".into();
    let barrier = Arc::new(Barrier::new(2));
    let handles: Vec<_> = (0..2)
        .map(|_| {
            let database = f.database.clone();
            let backups = f.backups.clone();
            let barrier = barrier.clone();
            let input = input.clone();
            std::thread::spawn(move || {
                let mut service = WorkspaceService::open(&database, &backups).unwrap();
                barrier.wait();
                service.record_checkpoint(input).unwrap().checkpoint
            })
        })
        .collect();
    let results: Vec<_> = handles.into_iter().map(|h| h.join().unwrap()).collect();
    assert_eq!(results[0], results[1]);
    let after = f.workspace();
    assert_eq!(after.daily.checkpoints.len(), 1);
    assert_eq!(after.daily.commands.len(), 1);
    assert_eq!(after.actions[0].revision, before.actions[0].revision + 1);
    assert_eq!(after.progress, before.progress);
}
