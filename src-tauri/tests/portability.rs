use ascension_lib::domain::*;
use ascension_lib::progress::*;
use ascension_lib::service::WorkspaceService;
use rusqlite::{Connection, params};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::fs;
use std::path::{Path, PathBuf};
use tempfile::TempDir;

const SAMPLE: &[u8] = include_bytes!("../../contracts/fixtures/sample-v1.json");
const EMPTY: &[u8] = include_bytes!("../../contracts/fixtures/empty-v1.json");
const LEGACY_GENERATION: &str = "99999999-9999-4999-8999-999999999999";

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

    fn restore(&mut self, bytes: &[u8]) -> RestoreResult {
        let preview = self
            .service
            .preview_import(bytes, "synthetic.json".into())
            .unwrap();
        self.service
            .restore_import(RestoreInput {
                expected_generation: preview.generation,
                token: preview.token,
            })
            .unwrap()
    }

    fn backup_paths(&self) -> Vec<PathBuf> {
        fs::read_dir(&self.backups)
            .map(|entries| entries.map(|entry| entry.unwrap().path()).collect())
            .unwrap_or_default()
    }
}

fn seed_v2(database: &Path) -> RecordSet {
    let records: RecordSet = serde_json::from_slice(SAMPLE).unwrap();
    let connection = Connection::open(database).unwrap();
    connection
        .pragma_update(None, "foreign_keys", true)
        .unwrap();
    connection
        .execute_batch(include_str!("../migrations/001_records.sql"))
        .unwrap();
    connection
        .execute_batch(include_str!("../migrations/002_workspace_state.sql"))
        .unwrap();
    for row in &records.projects {
        connection
            .execute(
                "INSERT INTO projects VALUES (?1,?2,?3,?4,?5)",
                params![
                    row.id,
                    row.title,
                    row.revision,
                    row.created_at,
                    row.updated_at
                ],
            )
            .unwrap();
    }
    for row in &records.objectives {
        connection
            .execute(
                "INSERT INTO objectives VALUES (?1,?2,?3,?4,?5,?6,?7)",
                params![
                    row.id,
                    row.project_id,
                    row.title,
                    row.criterion,
                    row.revision,
                    row.created_at,
                    row.updated_at
                ],
            )
            .unwrap();
    }
    for row in &records.actions {
        connection
            .execute(
                "INSERT INTO actions VALUES (?1,?2,?3,?4,?5,?6,?7)",
                params![
                    row.id,
                    row.objective_id,
                    row.description,
                    row.stop_condition,
                    row.revision,
                    row.created_at,
                    row.updated_at
                ],
            )
            .unwrap();
    }
    for row in &records.notes {
        connection
            .execute(
                "INSERT INTO notes VALUES (?1,?2,?3,?4,?5,?6)",
                params![
                    row.id,
                    row.objective_id,
                    row.text,
                    row.revision,
                    row.created_at,
                    row.updated_at
                ],
            )
            .unwrap();
    }
    connection
        .execute(
            "INSERT INTO workspace_state VALUES (1,?1,?2)",
            params![LEGACY_GENERATION, records.selected_objective_id],
        )
        .unwrap();
    connection.pragma_update(None, "user_version", 2).unwrap();
    records
}

fn assert_legacy_fields(workspace: &Workspace, expected: &RecordSet) {
    assert_eq!(workspace.projects, expected.projects);
    assert_eq!(workspace.objectives, expected.objectives);
    assert_eq!(workspace.actions, expected.actions);
    assert_eq!(workspace.notes, expected.notes);
    assert_eq!(
        workspace.selected_objective_id,
        expected.selected_objective_id
    );
}

fn count(connection: &Connection, table: &str) -> i64 {
    // Table names are test constants, never renderer/user input.
    connection
        .query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |row| {
            row.get(0)
        })
        .unwrap()
}

fn accept_review(
    fixture: &mut Fixture,
    workspace: Workspace,
) -> (Workspace, AcceptProgressInput, SettlementResult) {
    let parent_id = workspace.objectives[0].id.clone();
    accept_review_for_parent(fixture, workspace, &parent_id)
}

fn accept_review_for_parent(
    fixture: &mut Fixture,
    workspace: Workspace,
    parent_id: &str,
) -> (Workspace, AcceptProgressInput, SettlementResult) {
    let criterion = workspace
        .progress
        .criteria
        .iter()
        .filter(|row| row.parent_id == parent_id)
        .max_by_key(|row| row.criterion_version)
        .unwrap()
        .clone();
    let evidence_refs = workspace
        .progress
        .evidence
        .iter()
        .filter(|row| row.criterion_id == criterion.id)
        .map(|row| row.id.clone())
        .collect();
    let review = fixture
        .service
        .review_progress(ReviewProgressInput {
            expected_generation: workspace.generation,
            parent_id: criterion.parent_id.clone(),
            criterion_lineage_id: criterion.criterion_lineage_id.clone(),
            criterion_version: criterion.criterion_version,
            evidence_refs,
            factual_summary: "The complete synthetic example satisfies the declared criterion."
                .into(),
            assistance: Assistance::None,
        })
        .unwrap();
    assert!(
        review.snapshot.eligible,
        "Missing: {:?}; conflicts: {:?}",
        review.snapshot.missing, review.snapshot.conflicts
    );
    let command = AcceptProgressInput {
        schema_version: 1,
        command_id: uuid::Uuid::new_v4().to_string(),
        expected_generation: review.workspace.generation,
        parent_id: criterion.parent_id,
        criterion_lineage_id: criterion.criterion_lineage_id,
        criterion_version: criterion.criterion_version,
        expected_parent_revision: review
            .workspace
            .objectives
            .iter()
            .find(|row| row.id == parent_id)
            .unwrap()
            .revision,
        reviewed_snapshot_id: review.snapshot.id,
        reviewed_snapshot_digest: review.snapshot.digest,
        evidence_refs: review.snapshot.evidence_refs,
        factual_summary: review.snapshot.factual_summary,
        assistance: Assistance::None,
    };
    let accepted = fixture.service.accept_progress(command.clone()).unwrap();
    (accepted.workspace, command, accepted.result)
}

fn rewarded_fixture() -> (Fixture, Workspace, AcceptProgressInput, SettlementResult) {
    let mut fixture = Fixture::new();
    let workspace = fixture.restore(SAMPLE).workspace;
    let criterion = workspace
        .progress
        .criteria
        .iter()
        .find(|row| row.parent_id == workspace.objectives[0].id)
        .unwrap()
        .clone();
    let workspace = fixture
        .service
        .lock_criterion(LockCriterionInput {
            expected_generation: workspace.generation,
            parent_id: criterion.parent_id.clone(),
            criterion_lineage_id: criterion.criterion_lineage_id.clone(),
            criterion_version: criterion.criterion_version,
        })
        .unwrap();
    let workspace = fixture
        .service
        .record_evidence(RecordEvidenceInput {
            expected_generation: workspace.generation,
            parent_id: criterion.parent_id,
            criterion_lineage_id: criterion.criterion_lineage_id,
            criterion_version: criterion.criterion_version,
            kind: EvidenceKind::UserAttestation,
            description: "I inspected the synthetic example and all declared sections are present."
                .into(),
            reference: None,
            observed_at: None,
            status: EvidenceStatus::Supports,
            assistance: Assistance::None,
            uncertainty: None,
            supersedes: None,
        })
        .unwrap();
    let (workspace, command, result) = accept_review(&mut fixture, workspace);
    assert_eq!(result.issued_bc, 4);
    assert_eq!(workspace.progress.wallet_bc, 4);
    (fixture, workspace, command, result)
}

fn corrected_and_reaccepted_fixture() -> (Fixture, Workspace, AcceptProgressInput, SettlementResult)
{
    let (mut fixture, workspace, original_command, original_result) = rewarded_fixture();
    let corrected = fixture
        .service
        .correct_acceptance(CorrectAcceptanceInput {
            expected_generation: workspace.generation,
            acceptance_id: original_result.acceptance_id.clone(),
            kind: CorrectionKind::IncorrectAcceptance,
            reason:
                "The original synthetic review omitted a detail; the factual status is withdrawn."
                    .into(),
        })
        .unwrap();
    assert_eq!(corrected.progress.wallet_bc, 4);
    let (reaccepted, _, new_result) = accept_review(&mut fixture, corrected);
    assert_eq!(new_result.issued_bc, 0);
    assert_eq!(reaccepted.progress.grants.len(), 1);
    assert_eq!(reaccepted.progress.ledger.len(), 1);
    assert_eq!(reaccepted.progress.acceptances.len(), 2);
    assert_eq!(reaccepted.progress.corrections.len(), 1);
    (fixture, reaccepted, original_command, original_result)
}

#[test]
fn populated_v2_upgrade_preserves_legacy_records_wal_selection_and_recoverable_backup() {
    let directory = tempfile::tempdir().unwrap();
    let database = directory.path().join("v2.sqlite3");
    let backups = directory.path().join("backups");
    let mut expected = seed_v2(&database);
    let writer = Connection::open(&database).unwrap();
    writer.pragma_update(None, "journal_mode", "WAL").unwrap();
    writer.pragma_update(None, "wal_autocheckpoint", 0).unwrap();
    expected.notes[0].text = "Synthetic pre-upgrade note committed in WAL.".into();
    expected.notes[0].revision += 1;
    writer
        .execute(
            "UPDATE notes SET text=?1,revision=?2 WHERE id=?3",
            params![
                expected.notes[0].text,
                expected.notes[0].revision,
                expected.notes[0].id
            ],
        )
        .unwrap();
    let service = WorkspaceService::open(&database, &backups).unwrap();
    let workspace = service.load_workspace().unwrap();
    assert_legacy_fields(&workspace, &expected);
    assert_eq!(workspace.generation, LEGACY_GENERATION);
    let observer = Connection::open(&database).unwrap();
    assert_eq!(
        observer
            .pragma_query_value(None, "user_version", |row| row.get::<_, u32>(0))
            .unwrap(),
        3
    );
    let duplicated_criterion: i64 = observer
        .query_row(
            "SELECT COUNT(*) FROM pragma_table_info('objectives') WHERE name='criterion'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(duplicated_criterion, 0);
    assert_eq!(
        count(&observer, "criterion_versions"),
        expected.objectives.len() as i64
    );
    assert_eq!(
        count(&observer, "reward_slots"),
        expected.objectives.len() as i64
    );
    for objective in &expected.objectives {
        let (description, state, version, allocation, locked): (String, String, u32, u32, bool) = observer.query_row(
            "SELECT c.description,c.state,c.criterion_version,s.allocation_bc,s.allocation_locked
             FROM criterion_versions c JOIN reward_slots s ON c.reward_slot_id=s.id WHERE c.parent_id=?1",
            [&objective.id], |row| Ok((row.get(0)?,row.get(1)?,row.get(2)?,row.get(3)?,row.get(4)?))
        ).unwrap();
        assert_eq!(description, objective.criterion);
        assert_eq!(state, "draft");
        assert_eq!(version, 1);
        assert_eq!(allocation, 4);
        assert!(!locked);
    }
    for table in [
        "evidence_records",
        "review_snapshots",
        "acceptances",
        "corrections",
        "reward_grants",
        "ledger_entries",
        "committed_commands",
    ] {
        assert_eq!(
            count(&observer, table),
            0,
            "Migration must not create {table}"
        );
    }
    let paths: Vec<_> = fs::read_dir(&backups)
        .unwrap()
        .map(|row| row.unwrap().path())
        .collect();
    assert_eq!(paths.len(), 1);
    let backup = Connection::open(&paths[0]).unwrap();
    assert_eq!(
        backup
            .pragma_query_value(None, "user_version", |row| row.get::<_, u32>(0))
            .unwrap(),
        2
    );
    assert_eq!(
        backup
            .query_row("PRAGMA integrity_check", [], |row| row.get::<_, String>(0))
            .unwrap(),
        "ok"
    );
    assert_eq!(
        backup
            .query_row("SELECT generation FROM workspace_state", [], |row| row
                .get::<_, String>(
                0
            ))
            .unwrap(),
        LEGACY_GENERATION
    );
    assert_eq!(
        backup
            .query_row(
                "SELECT text FROM notes WHERE id=?1",
                [&expected.notes[0].id],
                |row| row.get::<_, String>(0)
            )
            .unwrap(),
        expected.notes[0].text
    );
    drop(backup);
    let recovery_database = directory.path().join("recovery.sqlite3");
    fs::copy(&paths[0], &recovery_database).unwrap();
    let recovered = WorkspaceService::open(
        &recovery_database,
        &directory.path().join("recovery-backups"),
    )
    .unwrap();
    assert_eq!(recovered.load_workspace().unwrap(), workspace);
    assert_eq!(
        recovered.export_bytes().unwrap(),
        service.export_bytes().unwrap()
    );
    drop(recovered);
    drop(observer);
    drop(writer);
    drop(service);
    assert_eq!(
        WorkspaceService::open(&database, &backups)
            .unwrap()
            .load_workspace()
            .unwrap(),
        workspace
    );
}

#[test]
fn v2_upgrade_backup_schema_and_validation_failures_preserve_the_original_database() {
    for failure in ["backup", "schema", "record"] {
        let directory = tempfile::tempdir().unwrap();
        let database = directory.path().join("v2.sqlite3");
        let backups = directory.path().join("backups");
        let records = seed_v2(&database);
        let observer = Connection::open(&database).unwrap();
        match failure {
            "backup" => fs::write(&backups, b"Synthetic backup-directory blocker").unwrap(),
            "schema" => observer.execute_batch("CREATE TABLE criterion_versions(marker TEXT); INSERT INTO criterion_versions VALUES ('synthetic marker');").unwrap(),
            "record" => { observer.execute("UPDATE objectives SET criterion='   '", []).unwrap(); },
            _ => unreachable!(),
        }
        assert!(
            WorkspaceService::open(&database, &backups).is_err(),
            "{failure}"
        );
        assert_eq!(
            observer
                .pragma_query_value(None, "user_version", |row| row.get::<_, u32>(0))
                .unwrap(),
            2
        );
        assert_eq!(count(&observer, "projects"), records.projects.len() as i64);
        assert_eq!(count(&observer, "actions"), records.actions.len() as i64);
        assert_eq!(count(&observer, "notes"), records.notes.len() as i64);
        assert_eq!(
            observer
                .query_row("SELECT generation FROM workspace_state", [], |row| row
                    .get::<_, String>(
                    0
                ))
                .unwrap(),
            LEGACY_GENERATION
        );
        assert_eq!(
            observer
                .query_row(
                    "SELECT COUNT(*) FROM pragma_table_info('objectives') WHERE name='criterion'",
                    [],
                    |row| row.get::<_, i64>(0)
                )
                .unwrap(),
            1
        );
        assert_eq!(
            observer
                .query_row(
                    "SELECT COUNT(*) FROM sqlite_master WHERE name='reward_slots'",
                    [],
                    |row| row.get::<_, i64>(0)
                )
                .unwrap(),
            0
        );
        if failure == "schema" {
            assert_eq!(
                observer
                    .query_row("SELECT marker FROM criterion_versions", [], |row| row
                        .get::<_, String>(0))
                    .unwrap(),
                "synthetic marker"
            );
        }
        assert_eq!(
            observer
                .query_row("PRAGMA integrity_check", [], |row| row.get::<_, String>(0))
                .unwrap(),
            "ok"
        );
    }
}

#[test]
fn modern_corrected_history_roundtrips_exactly_and_retries_remain_idempotent_after_restore() {
    let (fixture, original, command, original_result) = corrected_and_reaccepted_fixture();
    let bytes = fixture.service.export_bytes().unwrap();
    let records: RecordSet = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(records.schema_version, 2);
    assert_eq!(records.progress.as_ref(), Some(&original.progress));
    assert!(
        !String::from_utf8(bytes.clone())
            .unwrap()
            .contains("workspace.sqlite3")
    );
    let mut clean = Fixture::new();
    let restored = clean.restore(&bytes);
    assert!(!restored.unchanged);
    assert_eq!(RecordSet::from(&restored.workspace), records);
    assert_eq!(clean.service.export_bytes().unwrap(), bytes);
    let replay = clean.restore(&bytes);
    assert!(replay.unchanged);
    assert!(!replay.backup_created);
    assert_eq!(replay.workspace, restored.workspace);
    let mut reordered = records.clone();
    reordered.projects.reverse();
    reordered.objectives.reverse();
    reordered.actions.reverse();
    reordered.notes.reverse();
    let progress = reordered.progress.as_mut().unwrap();
    macro_rules! reverse { ($($field:ident),*) => { $(progress.$field.reverse();)* }; }
    reverse!(
        parents,
        slots,
        criteria,
        evidence,
        validators,
        snapshots,
        acceptances,
        corrections,
        grants,
        ledger,
        audit,
        commands
    );
    let replay = clean.restore(&serde_json::to_vec(&reordered).unwrap());
    assert!(replay.unchanged);
    assert_eq!(replay.workspace, restored.workspace);
    let retry = clean.service.accept_progress(command.clone()).unwrap();
    assert_eq!(retry.result, original_result);
    assert_eq!(retry.workspace, restored.workspace);
    assert_eq!(clean.service.export_bytes().unwrap(), bytes);

    drop(clean.service);
    let mut reopened = WorkspaceService::open(&clean.database, &clean.backups).unwrap();
    let retry = reopened.accept_progress(command.clone()).unwrap();
    assert_eq!(retry.result, original_result);
    assert_eq!(retry.workspace, restored.workspace);
    let mut changed_payload = command;
    changed_payload.factual_summary = "Changed payload using the same synthetic command ID".into();
    assert!(reopened.accept_progress(changed_payload).is_err());
    assert_eq!(reopened.load_workspace().unwrap(), restored.workspace);
    assert_eq!(reopened.export_bytes().unwrap(), bytes);
}

#[test]
fn legacy_replacement_cannot_erase_earned_or_corrected_history() {
    let (mut fixture, original, _, _) = corrected_and_reaccepted_fixture();
    let original_bytes = fixture.service.export_bytes().unwrap();
    let original_backups = fixture.backup_paths().len();
    for bytes in [SAMPLE, EMPTY] {
        // A legacy file cannot carry the history needed for safe replacement.
        assert!(
            fixture
                .service
                .preview_import(bytes, "legacy.json".into())
                .is_err()
        );
        assert_eq!(fixture.service.load_workspace().unwrap(), original);
        assert_eq!(fixture.service.export_bytes().unwrap(), original_bytes);
        assert_eq!(fixture.backup_paths().len(), original_backups);
    }
}

#[test]
fn modern_replacement_creates_a_complete_recovery_backup_and_never_mints() {
    let (source, expected, _, _) = corrected_and_reaccepted_fixture();
    let incoming = source.service.export_bytes().unwrap();
    let (mut destination, before, _, _) = rewarded_fixture();
    let before_bytes = destination.service.export_bytes().unwrap();
    let previous_paths = destination.backup_paths();
    let restored = destination.restore(&incoming);
    assert!(restored.backup_created);
    assert_eq!(
        RecordSet::from(&restored.workspace),
        RecordSet::from(&expected)
    );
    assert_eq!(restored.workspace.progress.wallet_bc, 4);
    assert_eq!(restored.workspace.progress.grants, expected.progress.grants);
    assert_eq!(restored.workspace.progress.ledger, expected.progress.ledger);
    let new_paths: Vec<_> = destination
        .backup_paths()
        .into_iter()
        .filter(|path| !previous_paths.contains(path))
        .collect();
    assert_eq!(new_paths.len(), 1);
    let backup = fs::read(&new_paths[0]).unwrap();
    assert_eq!(backup, before_bytes);
    let recovered = destination.restore(&backup);
    assert_eq!(
        RecordSet::from(&recovered.workspace),
        RecordSet::from(&before)
    );
    assert_eq!(destination.service.export_bytes().unwrap(), before_bytes);
}

#[test]
fn late_modern_restore_failure_rolls_back_all_history_and_retains_a_complete_backup() {
    let (source, expected, _, _) = corrected_and_reaccepted_fixture();
    let incoming = source.service.export_bytes().unwrap();
    let (mut destination, before, _, _) = rewarded_fixture();
    let before_bytes = destination.service.export_bytes().unwrap();
    let previous_paths = destination.backup_paths();
    let preview = destination
        .service
        .preview_import(&incoming, "modern.json".into())
        .unwrap();
    let command = RestoreInput {
        expected_generation: preview.generation,
        token: preview.token,
    };
    let observer = Connection::open(&destination.database).unwrap();
    observer
        .execute_batch(
            "CREATE TRIGGER reject_restored_command BEFORE INSERT ON committed_commands
         BEGIN SELECT RAISE(ABORT,'synthetic late restore failure'); END;",
        )
        .unwrap();
    assert_eq!(
        destination
            .service
            .restore_import(command.clone())
            .unwrap_err()
            .code,
        "storage_error"
    );
    assert_eq!(destination.service.load_workspace().unwrap(), before);
    assert_eq!(destination.service.export_bytes().unwrap(), before_bytes);
    let paths: Vec<_> = destination
        .backup_paths()
        .into_iter()
        .filter(|path| !previous_paths.contains(path))
        .collect();
    assert_eq!(paths.len(), 1);
    assert_eq!(fs::read(&paths[0]).unwrap(), before_bytes);
    assert_eq!(
        observer
            .query_row("SELECT COUNT(*) FROM pragma_foreign_key_check", [], |row| {
                row.get::<_, i64>(0)
            })
            .unwrap(),
        0
    );
    observer
        .execute_batch("DROP TRIGGER reject_restored_command")
        .unwrap();
    let restored = destination.service.restore_import(command).unwrap();
    assert_eq!(
        RecordSet::from(&restored.workspace),
        RecordSet::from(&expected)
    );
}

#[test]
fn malformed_modern_history_cannot_replace_valid_saved_state() {
    let (mut fixture, original, _, _) = corrected_and_reaccepted_fixture();
    let bytes = fixture.service.export_bytes().unwrap();
    let original_backups = fixture.backup_paths().len();
    let valid: Value = serde_json::from_slice(&bytes).unwrap();
    let corruptions: Vec<fn(&mut Value)> = vec![
        |v| {
            v.as_object_mut().unwrap().remove("progress");
        },
        |v| {
            v["progress"] = Value::Null;
        },
        |v| {
            v["progress"]["unexpected_authority"] = json!(true);
        },
        |v| {
            v["progress"]["wallet_bc"] = json!(999);
        },
        |v| {
            v["progress"]["wallet_bc"] = json!(4.5);
        },
        |v| {
            v["progress"]["ledger"][0]["amount_bc"] = json!(3);
        },
        |v| {
            v["progress"]["ledger"][0]["grant_id"] = json!(uuid::Uuid::new_v4().to_string());
        },
        |v| {
            v["progress"]["grants"][0]["amount_bc"] = json!(5);
        },
        |v| {
            v["progress"]["grants"][0]["reward_slot_id"] = json!(uuid::Uuid::new_v4().to_string());
        },
        |v| {
            v["progress"]["grants"][0]["reward_policy_version"] = json!("unsupported-v99");
        },
        |v| {
            v["progress"]["criteria"][0]["description"] = json!("Tampered criterion definition");
        },
        |v| {
            v["progress"]["criteria"][0]["state"] = json!("trusted");
        },
        |v| {
            v["progress"]["slots"][0]["allocation_bc"] = json!(-1);
        },
        |v| {
            v["progress"]["slots"][0]["parent_id"] = json!(uuid::Uuid::new_v4().to_string());
        },
        |v| {
            v["progress"]["evidence"][0]["parent_id"] = json!(uuid::Uuid::new_v4().to_string());
        },
        |v| {
            v["progress"]["evidence"][0]["kind"] = json!("trusted_validator");
        },
        |v| {
            v["progress"]["snapshots"][0]["digest"] = json!("0".repeat(64));
        },
        |v| {
            v["progress"]["snapshots"][0]["applicable_evidence"] = json!([]);
        },
        |v| {
            v["progress"]["acceptances"][0]["actor"] = json!("external_authority");
        },
        |v| {
            v["progress"]["acceptances"][0]["snapshot_id"] =
                json!(uuid::Uuid::new_v4().to_string());
        },
        |v| {
            v["progress"]["corrections"][0]["acceptance_id"] =
                json!(uuid::Uuid::new_v4().to_string());
        },
        |v| {
            v["progress"]["commands"][0]["payload_digest"] = json!("0".repeat(64));
        },
        |v| {
            v["progress"]["commands"][0]["result"]["issued_bc"] = json!(99);
        },
        |v| {
            let command = v["progress"]["commands"]
                .as_array_mut()
                .unwrap()
                .iter_mut()
                .find(|command| command["result"]["issued_bc"] == 0)
                .unwrap();
            command["result"]["balance_bc"] = json!(0);
        },
        |v| {
            v["progress"]["audit"][0]["id"] = v["projects"][0]["id"].clone();
        },
        |v| {
            v["progress"]["audit"][0]["id"] = v["actions"][0]["id"].clone();
        },
        |v| {
            v["progress"]["audit"][0]["id"] = v["notes"][0]["id"].clone();
        },
        |v| {
            let duplicate = v["progress"]["ledger"][0].clone();
            v["progress"]["ledger"]
                .as_array_mut()
                .unwrap()
                .push(duplicate);
        },
        |v| {
            v["schema_version"] = json!(1);
        },
    ];
    for (index, corrupt) in corruptions.iter().enumerate() {
        let mut malformed = valid.clone();
        corrupt(&mut malformed);
        let error = fixture
            .service
            .preview_import(
                &serde_json::to_vec(&malformed).unwrap(),
                "malformed-modern.json".into(),
            )
            .expect_err(&format!("Malformed modern case {index} was accepted"));
        assert!(
            ["invalid_import", "unsupported_schema"].contains(&error.code.as_str()),
            "case {index}: {}",
            error.code
        );
        assert_eq!(
            fixture.service.load_workspace().unwrap(),
            original,
            "case {index}"
        );
        assert_eq!(
            fixture.service.export_bytes().unwrap(),
            bytes,
            "case {index}"
        );
        assert_eq!(fixture.backup_paths().len(), original_backups);
    }
}

#[test]
fn imported_fixture_label_cannot_register_an_available_trusted_validator() {
    let mut fixture = Fixture::new();
    let original = fixture.restore(SAMPLE).workspace;
    let bytes = fixture.service.export_bytes().unwrap();
    let mut forged: Value = serde_json::from_slice(&bytes).unwrap();
    let criterion = original.progress.criteria[0].clone();
    forged["progress"]["registry_revision"] = json!(1);
    forged["progress"]["validators"] = json!([{
        "id": uuid::Uuid::new_v4().to_string(),
        "parent_id": criterion.parent_id,
        "criterion_lineage_id": criterion.criterion_lineage_id,
        "criterion_version": criterion.criterion_version,
        "artifact_id": criterion.artifact_id,
        "artifact_version": criterion.artifact_version,
        "scope": criterion.scope,
        "source_id": "synthetic-validator",
        "source_provenance": "synthetic_registered_fixture",
        "available": true,
        "registry_revision": 1
    }]);
    assert_eq!(
        fixture
            .service
            .preview_import(
                &serde_json::to_vec(&forged).unwrap(),
                "forged-validator.json".into()
            )
            .unwrap_err()
            .code,
        "invalid_import"
    );
    assert_eq!(fixture.service.load_workspace().unwrap(), original);
    assert_eq!(fixture.service.export_bytes().unwrap(), bytes);
}

#[test]
fn prospective_versions_and_historical_reopening_restore_without_renewing_the_slot() {
    let (mut fixture, original, _, issued) = rewarded_fixture();
    let criterion = original
        .progress
        .criteria
        .iter()
        .find(|row| row.parent_id == original.objectives[0].id)
        .unwrap()
        .clone();
    let reopened = fixture
        .service
        .correct_acceptance(CorrectAcceptanceInput {
            expected_generation: original.generation,
            acceptance_id: issued.acceptance_id,
            kind: CorrectionKind::HistoricalReopened,
            reason: "The original example was valid; a revised example is now required.".into(),
        })
        .unwrap();
    let prospective = fixture
        .service
        .save_criterion(SaveCriterionInput {
            expected_generation: reopened.generation,
            parent_id: criterion.parent_id.clone(),
            criterion_lineage_id: Some(criterion.criterion_lineage_id.clone()),
            description: "The revised synthetic example has four labeled sections.".into(),
            allocation_bc: 4,
            evidence_policy: EvidencePolicy::UserAttestation,
            scope: criterion.scope,
            artifact_id: Some("synthetic-example".into()),
            artifact_version: Some("revision-two".into()),
            requires_independent_work: false,
            required_validators: vec![],
        })
        .unwrap();
    let locked = fixture
        .service
        .lock_criterion(LockCriterionInput {
            expected_generation: prospective.generation,
            parent_id: criterion.parent_id.clone(),
            criterion_lineage_id: criterion.criterion_lineage_id.clone(),
            criterion_version: 2,
        })
        .unwrap();
    let evidence = fixture
        .service
        .record_evidence(RecordEvidenceInput {
            expected_generation: locked.generation,
            parent_id: criterion.parent_id.clone(),
            criterion_lineage_id: criterion.criterion_lineage_id.clone(),
            criterion_version: 2,
            kind: EvidenceKind::UserAttestation,
            description: "The revised synthetic example now has all four labeled sections.".into(),
            reference: None,
            observed_at: None,
            status: EvidenceStatus::Supports,
            assistance: Assistance::None,
            uncertainty: None,
            supersedes: None,
        })
        .unwrap();
    let (accepted, command, result) = accept_review(&mut fixture, evidence);
    assert_eq!(result.issued_bc, 0);
    assert!(result.retained_reward);
    assert_eq!(accepted.progress.wallet_bc, 4);
    assert_eq!(accepted.progress.slots, original.progress.slots);
    assert_eq!(accepted.progress.grants, original.progress.grants);
    assert_eq!(accepted.progress.ledger, original.progress.ledger);
    let old_definition = accepted
        .progress
        .criteria
        .iter()
        .find(|row| row.id == criterion.id)
        .unwrap();
    assert_eq!(
        old_definition,
        original
            .progress
            .criteria
            .iter()
            .find(|row| row.id == criterion.id)
            .unwrap()
    );
    let bytes = fixture.service.export_bytes().unwrap();
    let mut destination = Fixture::new();
    let restored = destination.restore(&bytes).workspace;
    assert_eq!(RecordSet::from(&restored), RecordSet::from(&accepted));
    assert_eq!(
        destination.service.accept_progress(command).unwrap().result,
        result
    );
    assert_eq!(destination.service.export_bytes().unwrap(), bytes);
}

#[test]
fn ledger_sequence_and_historical_command_totals_cannot_be_forged_on_import() {
    let (mut fixture, first, _, _) = corrected_and_reaccepted_fixture();
    let criterion = first
        .progress
        .criteria
        .iter()
        .find(|row| row.parent_id == first.objectives[1].id)
        .unwrap()
        .clone();
    let second_locked = fixture
        .service
        .lock_criterion(LockCriterionInput {
            expected_generation: first.generation,
            parent_id: criterion.parent_id.clone(),
            criterion_lineage_id: criterion.criterion_lineage_id.clone(),
            criterion_version: criterion.criterion_version,
        })
        .unwrap();
    let evidenced = fixture
        .service
        .record_evidence(RecordEvidenceInput {
            expected_generation: second_locked.generation,
            parent_id: criterion.parent_id.clone(),
            criterion_lineage_id: criterion.criterion_lineage_id,
            criterion_version: criterion.criterion_version,
            kind: EvidenceKind::UserAttestation,
            description: "The second synthetic objective now satisfies its declared criterion."
                .into(),
            reference: None,
            observed_at: None,
            status: EvidenceStatus::Supports,
            assistance: Assistance::None,
            uncertainty: None,
            supersedes: None,
        })
        .unwrap();
    let (original, _, _) = accept_review_for_parent(&mut fixture, evidenced, &criterion.parent_id);
    assert_eq!(original.progress.wallet_bc, 8);
    assert_eq!(original.progress.ledger.len(), 2);
    let bytes = fixture.service.export_bytes().unwrap();
    let valid: Value = serde_json::from_slice(&bytes).unwrap();
    let backup_count = fixture.backup_paths().len();
    type Corruption = (&'static str, fn(&mut Value));
    let cases: Vec<Corruption> = vec![
        ("sequence gap", |v| {
            let entry = v["progress"]["ledger"]
                .as_array_mut()
                .unwrap()
                .iter_mut()
                .find(|row| row["sequence"] == 2)
                .unwrap();
            entry["sequence"] = json!(3);
        }),
        ("duplicate sequence", |v| {
            for entry in v["progress"]["ledger"].as_array_mut().unwrap() {
                entry["sequence"] = json!(1);
            }
        }),
        ("fractional sequence", |v| {
            v["progress"]["ledger"][0]["sequence"] = json!(1.5);
        }),
        ("zero sequence", |v| {
            v["progress"]["ledger"][0]["sequence"] = json!(0);
        }),
        ("missing sequence", |v| {
            v["progress"]["ledger"][0]
                .as_object_mut()
                .unwrap()
                .remove("sequence");
        }),
        ("nonexistent committed prefix", |v| {
            v["progress"]["commands"][0]["ledger_sequence"] = json!(3);
        }),
        ("fractional committed prefix", |v| {
            v["progress"]["commands"][0]["ledger_sequence"] = json!(1.5);
        }),
        ("zero committed prefix", |v| {
            v["progress"]["commands"][0]["ledger_sequence"] = json!(0);
        }),
        ("missing committed prefix", |v| {
            v["progress"]["commands"][0]
                .as_object_mut()
                .unwrap()
                .remove("ledger_sequence");
        }),
        ("retained settlement wrong prefix", |v| {
            let command = v["progress"]["commands"]
                .as_array_mut()
                .unwrap()
                .iter_mut()
                .find(|row| row["result"]["issued_bc"] == 0)
                .unwrap();
            command["ledger_sequence"] = json!(2);
        }),
        ("intermediate forged historical balance", |v| {
            let command = v["progress"]["commands"]
                .as_array_mut()
                .unwrap()
                .iter_mut()
                .find(|row| row["result"]["issued_bc"] == 0)
                .unwrap();
            command["result"]["balance_bc"] = json!(6);
        }),
        ("issuance moved to a later valid prefix", |v| {
            let command = v["progress"]["commands"]
                .as_array_mut()
                .unwrap()
                .iter_mut()
                .find(|row| row["result"]["issued_bc"] == 4 && row["ledger_sequence"] == 1)
                .unwrap();
            command["ledger_sequence"] = json!(2);
            command["result"]["balance_bc"] = json!(8);
        }),
    ];
    for (label, corrupt) in cases {
        let mut malformed = valid.clone();
        corrupt(&mut malformed);
        assert_eq!(
            fixture
                .service
                .preview_import(
                    &serde_json::to_vec(&malformed).unwrap(),
                    "forged-sequence.json".into()
                )
                .expect_err(label)
                .code,
            "invalid_import",
            "{label}"
        );
        assert_eq!(
            fixture.service.load_workspace().unwrap(),
            original,
            "{label}"
        );
        assert_eq!(fixture.service.export_bytes().unwrap(), bytes, "{label}");
        assert_eq!(fixture.backup_paths().len(), backup_count);
    }
    let mut clean = Fixture::new();
    let restored = clean.restore(&bytes).workspace;
    assert_eq!(restored.progress, original.progress);
    for command in &original.progress.commands {
        assert_eq!(
            clean
                .service
                .accept_progress(command.payload.clone())
                .unwrap()
                .result,
            command.result
        );
    }
    assert_eq!(clean.service.export_bytes().unwrap(), bytes);
}

#[test]
fn a_recomputed_command_digest_cannot_claim_a_future_parent_revision() {
    let (mut fixture, original, _, _) = corrected_and_reaccepted_fixture();
    let bytes = fixture.service.export_bytes().unwrap();
    let mut forged: RecordSet = serde_json::from_slice(&bytes).unwrap();
    let command = &mut forged.progress.as_mut().unwrap().commands[0];
    let current_revision = original
        .objectives
        .iter()
        .find(|row| row.id == command.payload.parent_id)
        .unwrap()
        .revision;
    command.payload.expected_parent_revision = current_revision + 1;
    command.payload_digest = format!(
        "{:x}",
        Sha256::digest(serde_json::to_vec(&command.payload).unwrap())
    );
    let original_backups = fixture.backup_paths().len();
    assert_eq!(
        fixture
            .service
            .preview_import(
                &serde_json::to_vec(&forged).unwrap(),
                "future-revision.json".into()
            )
            .unwrap_err()
            .code,
        "invalid_import"
    );
    assert_eq!(fixture.service.load_workspace().unwrap(), original);
    assert_eq!(fixture.service.export_bytes().unwrap(), bytes);
    assert_eq!(fixture.backup_paths().len(), original_backups);
}
