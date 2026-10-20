use ascension_lib::domain::*;
use ascension_lib::service::WorkspaceService;
use rusqlite::{Connection, params};
use std::path::PathBuf;
use tempfile::TempDir;

const SAMPLE: &[u8] = include_bytes!("../../contracts/fixtures/sample-v1.json");

struct Fixture {
    _directory: TempDir,
    database: PathBuf,
    backups: PathBuf,
    service: WorkspaceService,
    observer: Connection,
}

impl Fixture {
    fn new() -> Self {
        let directory = tempfile::tempdir().unwrap();
        let database = directory.path().join("workspace.sqlite3");
        let backups = directory.path().join("backups");
        let mut service = WorkspaceService::open(&database, &backups).unwrap();
        let preview = service
            .preview_import(SAMPLE, "synthetic.json".into())
            .unwrap();
        let workspace = service
            .restore_import(RestoreInput {
                token: preview.token,
                expected_generation: preview.generation,
            })
            .unwrap()
            .workspace;
        let observer = Connection::open(&database).unwrap();
        observer.pragma_update(None, "foreign_keys", true).unwrap();
        // These are test-only dependents, standing in for future related records.
        // RESTRICT prevents delete/reinsert even when IDs would be restored later.
        observer
            .execute_batch(
                "
            CREATE TABLE sentinel_links (
                project_id TEXT NOT NULL REFERENCES projects(id) ON DELETE RESTRICT,
                objective_id TEXT NOT NULL REFERENCES objectives(id) ON DELETE RESTRICT,
                action_id TEXT NOT NULL REFERENCES actions(id) ON DELETE RESTRICT,
                marker TEXT NOT NULL
            );
            CREATE TABLE sentinel_notes (
                note_id TEXT NOT NULL REFERENCES notes(id) ON DELETE CASCADE,
                marker TEXT NOT NULL
            );
            CREATE TABLE mutation_audit (table_name TEXT, operation TEXT, record_id TEXT);
        ",
            )
            .unwrap();
        for objective in &workspace.objectives {
            let action = workspace
                .actions
                .iter()
                .find(|row| row.objective_id == objective.id)
                .unwrap();
            observer
                .execute(
                    "INSERT INTO sentinel_links VALUES (?1,?2,?3,'synthetic dependent')",
                    params![objective.project_id, objective.id, action.id],
                )
                .unwrap();
        }
        for note in &workspace.notes {
            observer
                .execute(
                    "INSERT INTO sentinel_notes VALUES (?1,'synthetic note dependent')",
                    [&note.id],
                )
                .unwrap();
        }
        // UPDATE triggers reveal unnecessary rewrites that FK checks alone miss.
        for table in [
            "projects",
            "objectives",
            "actions",
            "notes",
            "workspace_state",
        ] {
            for operation in ["INSERT", "UPDATE", "DELETE"] {
                let source = if operation == "DELETE" { "old" } else { "new" };
                let column = if table == "workspace_state" {
                    "singleton"
                } else {
                    "id"
                };
                observer.execute_batch(&format!(
                    "CREATE TRIGGER audit_{table}_{operation} AFTER {operation} ON {table}
                     BEGIN INSERT INTO mutation_audit VALUES ('{table}','{operation}',{source}.{column}); END;"
                )).unwrap();
            }
        }
        Self {
            _directory: directory,
            database,
            backups,
            service,
            observer,
        }
    }

    fn workspace(&self) -> Workspace {
        self.service.load_workspace().unwrap()
    }

    fn assert_changes(&self, records: &[(&str, &str, &str)], state_changed: bool) {
        let mut statement = self.observer.prepare(
            "SELECT table_name,operation,record_id FROM mutation_audit ORDER BY table_name,operation,record_id"
        ).unwrap();
        let actual: Vec<(String, String, String)> = statement
            .query_map([], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)))
            .unwrap()
            .collect::<Result<_, _>>()
            .unwrap();
        let mut expected: Vec<_> = records
            .iter()
            .map(|(table, operation, id)| {
                (table.to_string(), operation.to_string(), id.to_string())
            })
            .collect();
        if state_changed {
            expected.push(("workspace_state".into(), "UPDATE".into(), "1".into()));
        }
        expected.sort();
        assert_eq!(actual, expected);
        self.observer
            .execute("DELETE FROM mutation_audit", [])
            .unwrap();
        let foreign_key_errors: i64 = self
            .observer
            .query_row("SELECT COUNT(*) FROM pragma_foreign_key_check", [], |row| {
                row.get(0)
            })
            .unwrap();
        assert_eq!(foreign_key_errors, 0);
        let linked: i64 = self
            .observer
            .query_row(
                "SELECT COUNT(*) FROM sentinel_links WHERE marker='synthetic dependent'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(linked, 2);
    }

    fn assert_note_dependents(&self, expected: i64) {
        let count: i64 = self
            .observer
            .query_row(
                "SELECT COUNT(*) FROM sentinel_notes WHERE marker='synthetic note dependent'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(count, expected);
    }

    fn assert_restart(self, expected: Workspace) {
        drop(self.service);
        drop(self.observer);
        let reopened = WorkspaceService::open(&self.database, &self.backups).unwrap();
        assert_eq!(reopened.load_workspace().unwrap(), expected);
    }
}

fn save_input(workspace: &Workspace) -> SaveWorkInput {
    let objective = &workspace.objectives[0];
    let action = workspace
        .actions
        .iter()
        .find(|row| row.objective_id == objective.id)
        .unwrap();
    SaveWorkInput {
        expected_generation: workspace.generation.clone(),
        objective_id: objective.id.clone(),
        description: action.description.clone(),
        stop_condition: action.stop_condition.clone(),
        note: workspace
            .notes
            .iter()
            .find(|row| row.objective_id == objective.id)
            .map(|row| row.text.clone()),
    }
}

#[test]
fn ordinary_edits_preserve_dependent_rows_and_only_write_changed_records() {
    let mut fixture = Fixture::new();
    let original = fixture.workspace();
    let mut workspace = fixture
        .service
        .update_project(UpdateProjectInput {
            expected_generation: original.generation.clone(),
            id: original.projects[0].id.clone(),
            title: "Revised synthetic project".into(),
        })
        .unwrap();
    fixture.assert_changes(&[("projects", "UPDATE", &original.projects[0].id)], true);
    assert_eq!(
        workspace.projects[0].revision,
        original.projects[0].revision + 1
    );
    assert_eq!(workspace.projects[1], original.projects[1]);
    assert_eq!(workspace.objectives, original.objectives);
    assert_eq!(workspace.actions, original.actions);
    assert_eq!(workspace.notes, original.notes);

    workspace = fixture
        .service
        .update_objective(UpdateObjectiveInput {
            expected_generation: workspace.generation,
            id: original.objectives[0].id.clone(),
            title: "Revised synthetic objective".into(),
            criterion: "The example contains four labeled sections.".into(),
        })
        .unwrap();
    fixture.assert_changes(
        &[("objectives", "UPDATE", &original.objectives[0].id)],
        true,
    );
    assert_eq!(
        workspace.objectives[0].revision,
        original.objectives[0].revision + 1
    );
    assert_eq!(workspace.objectives[1], original.objectives[1]);

    let mut action_edit = save_input(&workspace);
    action_edit.description = "Label the fourth example section.".into();
    workspace = fixture.service.save_work(action_edit).unwrap();
    fixture.assert_changes(&[("actions", "UPDATE", &original.actions[0].id)], true);
    assert_eq!(
        workspace.actions[0].revision,
        original.actions[0].revision + 1
    );
    assert_eq!(workspace.actions[1], original.actions[1]);
    assert_eq!(workspace.notes, original.notes);

    let mut note_edit = save_input(&workspace);
    note_edit.note = Some("Continue at the fourth example section.".into());
    workspace = fixture.service.save_work(note_edit).unwrap();
    fixture.assert_changes(&[("notes", "UPDATE", &original.notes[0].id)], true);
    assert_eq!(workspace.notes[0].revision, original.notes[0].revision + 1);
    assert_eq!(workspace.notes[1], original.notes[1]);
    fixture.assert_note_dependents(2);

    for selection in [None, Some(original.objectives[0].id.clone())] {
        let before = workspace.clone();
        workspace = fixture
            .service
            .select_objective(SelectObjectiveInput {
                expected_generation: workspace.generation,
                id: selection,
            })
            .unwrap();
        fixture.assert_changes(&[], true);
        assert_eq!(workspace.projects, before.projects);
        assert_eq!(workspace.objectives, before.objectives);
        assert_eq!(workspace.actions, before.actions);
        assert_eq!(workspace.notes, before.notes);
    }
    assert_eq!(
        fixture.service.save_work(save_input(&workspace)).unwrap(),
        workspace
    );
    assert_eq!(
        fixture
            .service
            .update_project(UpdateProjectInput {
                expected_generation: workspace.generation.clone(),
                id: workspace.projects[0].id.clone(),
                title: workspace.projects[0].title.clone(),
            })
            .unwrap(),
        workspace
    );
    assert_eq!(
        fixture
            .service
            .update_objective(UpdateObjectiveInput {
                expected_generation: workspace.generation.clone(),
                id: workspace.objectives[0].id.clone(),
                title: workspace.objectives[0].title.clone(),
                criterion: workspace.objectives[0].criterion.clone(),
            })
            .unwrap(),
        workspace
    );
    assert_eq!(
        fixture
            .service
            .select_objective(SelectObjectiveInput {
                expected_generation: workspace.generation.clone(),
                id: workspace.selected_objective_id.clone(),
            })
            .unwrap(),
        workspace
    );
    fixture.assert_changes(&[], false);
    fixture.assert_restart(workspace);
}

#[test]
fn creation_and_note_clear_never_rewrite_unrelated_records() {
    let mut fixture = Fixture::new();
    let original = fixture.workspace();
    let mut workspace = fixture
        .service
        .create_project(CreateProjectInput {
            expected_generation: original.generation.clone(),
            title: "Another synthetic project".into(),
        })
        .unwrap();
    let project = workspace
        .projects
        .iter()
        .find(|row| !original.projects.contains(row))
        .unwrap()
        .clone();
    fixture.assert_changes(&[("projects", "INSERT", &project.id)], true);
    workspace = fixture
        .service
        .create_objective(CreateObjectiveInput {
            expected_generation: workspace.generation,
            project_id: project.id,
            title: "Another bounded objective".into(),
            criterion: "One complete example is visible.".into(),
        })
        .unwrap();
    let objective_id = workspace.selected_objective_id.clone().unwrap();
    fixture.assert_changes(&[("objectives", "INSERT", &objective_id)], true);
    workspace = fixture
        .service
        .save_work(SaveWorkInput {
            expected_generation: workspace.generation,
            objective_id: objective_id.clone(),
            description: "Write one synthetic example.".into(),
            stop_condition: "The example has a title and paragraph.".into(),
            note: Some("Start with the title.".into()),
        })
        .unwrap();
    let action = workspace
        .actions
        .iter()
        .find(|row| row.objective_id == objective_id)
        .unwrap();
    let note = workspace
        .notes
        .iter()
        .find(|row| row.objective_id == objective_id)
        .unwrap();
    fixture.assert_changes(
        &[
            ("actions", "INSERT", &action.id),
            ("notes", "INSERT", &note.id),
        ],
        true,
    );

    let mut clear = save_input(&original);
    clear.expected_generation = workspace.generation.clone();
    clear.note = None;
    let cleared = fixture.service.save_work(clear.clone()).unwrap();
    fixture.assert_changes(&[("notes", "DELETE", &original.notes[0].id)], true);
    fixture.assert_note_dependents(1);
    assert_eq!(cleared.actions, workspace.actions);
    assert_eq!(cleared.projects, workspace.projects);
    assert_eq!(cleared.objectives, workspace.objectives);
    assert!(cleared.notes.contains(&original.notes[1]));

    clear.expected_generation = cleared.generation.clone();
    assert_eq!(fixture.service.save_work(clear.clone()).unwrap(), cleared);
    fixture.assert_changes(&[], false);
    clear.note = Some("A newly created synthetic note.".into());
    let recreated = fixture.service.save_work(clear).unwrap();
    let note = recreated
        .notes
        .iter()
        .find(|row| row.objective_id == original.objectives[0].id)
        .unwrap();
    assert_ne!(note.id, original.notes[0].id);
    assert_eq!(note.revision, 1);
    fixture.assert_changes(&[("notes", "INSERT", &note.id)], true);
    fixture.assert_note_dependents(1);
    fixture.assert_restart(recreated);
}

#[test]
fn stale_and_late_failed_writes_leave_records_dependents_and_generation_unchanged() {
    let mut fixture = Fixture::new();
    let original = fixture.workspace();
    let mut second = WorkspaceService::open(&fixture.database, &fixture.backups).unwrap();
    let current = fixture
        .service
        .select_objective(SelectObjectiveInput {
            expected_generation: original.generation.clone(),
            id: None,
        })
        .unwrap();
    fixture.assert_changes(&[], true);
    let mut stale = save_input(&original);
    stale.description = "This stale edit must never persist.".into();
    assert_eq!(second.save_work(stale).unwrap_err().code, "conflict");
    fixture.assert_changes(&[], false);
    assert_eq!(second.load_workspace().unwrap(), current);
    drop(second);

    // Abort after the action and note UPDATEs, at the final generation write.
    // Audit inserts participate in that same transaction and must roll back too.
    fixture
        .observer
        .execute_batch(
            "CREATE TRIGGER reject_state_update BEFORE UPDATE ON workspace_state
         BEGIN SELECT RAISE(ABORT,'synthetic late failure'); END;",
        )
        .unwrap();
    let before_export = fixture.service.export_bytes().unwrap();
    let mut failed = save_input(&current);
    failed.description = "This action update must roll back.".into();
    failed.note = Some("This note update must roll back.".into());
    assert_eq!(
        fixture.service.save_work(failed).unwrap_err().code,
        "storage_error"
    );
    assert_eq!(fixture.workspace(), current);
    assert_eq!(fixture.service.export_bytes().unwrap(), before_export);
    fixture.assert_changes(&[], false);
    fixture.assert_note_dependents(2);
    fixture.assert_restart(current);
}
