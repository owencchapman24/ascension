use super::*;
use crate::domain::*;
use crate::progress::*;
use crate::service::WorkspaceService;
use crate::study::*;
use crate::world::*;
use chrono::{Duration as ChronoDuration, SecondsFormat, Utc};

fn day(value: &str) -> NaiveDate {
    NaiveDate::parse_from_str(value, "%Y-%m-%d").unwrap()
}

fn wait_for(service: &WorkspaceService) -> BackupStatus {
    wait_status(|| service.backup_status())
}

fn wait_status(status: impl Fn() -> BackupStatus) -> BackupStatus {
    let limit = Instant::now() + Duration::from_secs(20);
    loop {
        let status = status();
        if !matches!(status.state, BackupState::Checking | BackupState::Running) {
            return status;
        }
        assert!(Instant::now() < limit, "Recovery worker did not finish");
        thread::sleep(Duration::from_millis(5));
    }
}

fn project(service: &mut WorkspaceService) -> Workspace {
    service
        .create_project(CreateProjectInput {
            expected_generation: service.load_workspace().unwrap().generation,
            title: "Synthetic recovery project".into(),
        })
        .unwrap()
}

fn restore(service: &mut WorkspaceService, bytes: &[u8]) -> Workspace {
    let preview = service
        .preview_import(bytes, "synthetic.json".into())
        .unwrap();
    service
        .restore_import(RestoreInput {
            token: preview.token,
            expected_generation: preview.generation,
        })
        .unwrap()
        .workspace
}

#[test]
fn wal_snapshot_is_standalone_and_full_authority_checked_before_finalization() {
    let directory = tempfile::tempdir().unwrap();
    let db = directory.path().join("work.sqlite3");
    let snapshots = directory.path().join("manual-test-snapshots");
    let mut service = WorkspaceService::open(&db, &directory.path().join("automatic")).unwrap();
    let connection = Connection::open(&db).unwrap();
    connection
        .pragma_update(None, "wal_autocheckpoint", 0)
        .unwrap();
    let saved = project(&mut service);
    assert!(
        fs::metadata(db.with_extension("sqlite3-wal"))
            .unwrap()
            .len()
            > 0
    );
    create_snapshot(&db, &snapshots, day("2026-10-01")).unwrap();
    let path = snapshots.join(filename(day("2026-10-01")));
    let snapshot = Connection::open_with_flags(&path, OpenFlags::SQLITE_OPEN_READ_ONLY).unwrap();
    assert_eq!(read_workspace(&snapshot).unwrap(), saved);
    assert_eq!(
        snapshot
            .query_row("PRAGMA journal_mode", [], |r| r.get::<_, String>(0))
            .unwrap(),
        "delete"
    );
    assert_eq!(
        snapshot
            .query_row("PRAGMA quick_check", [], |r| r.get::<_, String>(0))
            .unwrap(),
        "ok"
    );
    assert_eq!(
        snapshot
            .query_row("SELECT COUNT(*) FROM pragma_foreign_key_check", [], |r| r
                .get::<_, u32>(
                0
            ))
            .unwrap(),
        0
    );
    assert!(!path.with_extension("sqlite3-wal").exists());
    let independent = directory.path().join("independent.sqlite3");
    fs::copy(&path, &independent).unwrap();
    let mut reopened =
        WorkspaceService::open(&independent, &directory.path().join("independent-backups"))
            .unwrap();
    assert_eq!(reopened.load_workspace().unwrap(), saved);
    project(&mut reopened);
    assert_eq!(
        wait_for(&reopened).state,
        BackupState::Ready,
        "A standalone recovered database can make its own new snapshot"
    );
}

#[test]
fn open_reads_and_noops_create_no_snapshot_and_changed_day_is_immutable() {
    let directory = tempfile::tempdir().unwrap();
    let db = directory.path().join("work.sqlite3");
    let snapshots = directory.path().join("backups");
    let mut service = WorkspaceService::open(&db, &snapshots).unwrap();
    assert_eq!(wait_for(&service).state, BackupState::Idle);
    let initial = service.load_workspace().unwrap();
    service
        .select_objective(SelectObjectiveInput {
            expected_generation: initial.generation,
            id: None,
        })
        .unwrap();
    assert_eq!(wait_for(&service).snapshot_count, 0);
    assert!(!snapshots.exists());
    let first = project(&mut service);
    let status = wait_for(&service);
    assert_eq!(status.state, BackupState::Ready);
    assert_eq!(status.snapshot_count, 1);
    assert_eq!(
        status.latest_successful_day,
        Some(Local::now().date_naive().to_string())
    );
    let path = snapshots.join(filename(Local::now().date_naive()));
    let bytes = fs::read(&path).unwrap();
    let second = service
        .update_project(UpdateProjectInput {
            expected_generation: first.generation,
            id: first.projects[0].id.clone(),
            title: "Synthetic changed later".into(),
        })
        .unwrap();
    service.retry_backup();
    assert_eq!(wait_for(&service).snapshot_count, 1);
    assert_eq!(fs::read(&path).unwrap(), bytes);
    assert_eq!(service.load_workspace().unwrap(), second);
    drop(service);
    let reopened = WorkspaceService::open(&db, &snapshots).unwrap();
    assert_eq!(wait_for(&reopened).snapshot_count, 1);
    assert_eq!(fs::read(path).unwrap(), bytes);
}

#[test]
fn retention_keeps_seven_used_days_and_newest_of_three_represented_months() {
    let directory = tempfile::tempdir().unwrap();
    let db = directory.path().join("work.sqlite3");
    let snapshots = directory.path().join("backups");
    let _service = WorkspaceService::open(&db, &directory.path().join("automatic")).unwrap();
    let dates = [
        "2026-01-09",
        "2026-06-30",
        "2026-08-04",
        "2026-08-27",
        "2026-09-01",
        "2026-09-19",
        "2026-10-01",
        "2026-10-03",
        "2026-10-04",
        "2026-10-08",
        "2026-10-11",
        "2026-10-15",
        "2026-10-20",
        "2026-10-21",
    ];
    for date in dates {
        create_snapshot(&db, &snapshots, day(date)).unwrap();
    }
    let unknown = [
        "before-migration-synthetic.sqlite3",
        "before-restore-synthetic.json",
        "personal-copy.sqlite3",
        ".daily-recovery-pending-interrupted.sqlite3.tmp",
        "daily-v1-2026-07-01.sqlite3",
    ];
    for name in unknown {
        fs::write(snapshots.join(name), b"untouched synthetic file").unwrap();
    }
    let (found, unverified) = inventory_snapshots(&snapshots).unwrap();
    assert!(unverified);
    assert_eq!(found.len(), dates.len());
    prune(&found).unwrap();
    let (retained, _) = inventory_snapshots(&snapshots).unwrap();
    let actual: Vec<_> = retained.iter().map(|s| s.day.to_string()).collect();
    assert_eq!(
        actual,
        [
            "2026-08-27",
            "2026-09-19",
            "2026-10-03",
            "2026-10-04",
            "2026-10-08",
            "2026-10-11",
            "2026-10-15",
            "2026-10-20",
            "2026-10-21"
        ]
    );
    assert!(!snapshots.join(filename(day("2026-10-02"))).exists());
    for name in unknown {
        assert_eq!(
            fs::read(snapshots.join(name)).unwrap(),
            b"untouched synthetic file"
        );
    }
}

#[test]
fn failed_snapshot_does_not_prune_and_partial_or_mimic_cannot_be_recovery() {
    let directory = tempfile::tempdir().unwrap();
    let db = directory.path().join("work.sqlite3");
    let snapshots = directory.path().join("backups");
    let _service = WorkspaceService::open(&db, &directory.path().join("automatic")).unwrap();
    for number in 1..=10 {
        create_snapshot(&db, &snapshots, day(&format!("2026-09-{number:02}"))).unwrap();
    }
    let blocked = snapshots.join(filename(day("2026-10-01")));
    fs::write(&blocked, b"interrupted or user-owned file").unwrap();
    assert!(create_snapshot(&db, &snapshots, day("2026-10-01")).is_err());
    assert_eq!(inventory_snapshots(&snapshots).unwrap().0.len(), 10);
    assert_eq!(
        fs::read(&blocked).unwrap(),
        b"interrupted or user-owned file"
    );
    let manager = BackupManager::new(&db, &snapshots);
    wait_status(|| manager.status());
    manager.request(day("2026-10-01"));
    assert_eq!(wait_status(|| manager.status()).state, BackupState::Warning);
    assert_eq!(
        inventory_snapshots(&snapshots).unwrap().0.len(),
        10,
        "A failed background attempt must not prune older valid points"
    );
    // A correctly named, coherent application database still lacks subsystem ownership.
    let mimic = snapshots.join(filename(day("2026-10-02")));
    Connection::open(&db)
        .unwrap()
        .backup(rusqlite::MAIN_DB, &mimic, None)
        .unwrap();
    assert!(inspect_snapshot(&mimic, day("2026-10-02")).is_err());
    assert_eq!(inventory_snapshots(&snapshots).unwrap().0.len(), 10);
}

#[test]
fn invalid_foreign_keys_schema_and_authority_never_finalize_or_prune() {
    let directory = tempfile::tempdir().unwrap();
    let db = directory.path().join("work.sqlite3");
    let snapshots = directory.path().join("backups");
    let mut service = WorkspaceService::open(&db, &directory.path().join("automatic")).unwrap();
    let saved = project(&mut service);
    wait_for(&service);
    create_snapshot(&db, &snapshots, day("2026-09-01")).unwrap();
    let connection = Connection::open(&db).unwrap();
    // Internally well-formed SQLite with a broken application authority identity.
    connection
        .execute(
            "UPDATE workspace_state SET generation='not-an-authority-id'",
            [],
        )
        .unwrap();
    assert!(create_snapshot(&db, &snapshots, day("2026-10-01")).is_err());
    connection
        .execute(
            "UPDATE workspace_state SET generation=?1",
            [&saved.generation],
        )
        .unwrap();
    connection
        .pragma_update(None, "user_version", DB_SCHEMA_VERSION + 1)
        .unwrap();
    assert!(create_snapshot(&db, &snapshots, day("2026-10-02")).is_err());
    connection
        .pragma_update(None, "user_version", DB_SCHEMA_VERSION)
        .unwrap();
    connection
        .pragma_update(None, "foreign_keys", "OFF")
        .unwrap();
    connection
        .execute(
            "UPDATE workspace_state SET selected_objective_id=?1",
            [new_id()],
        )
        .unwrap();
    assert!(create_snapshot(&db, &snapshots, day("2026-10-03")).is_err());
    connection
        .execute("UPDATE workspace_state SET selected_objective_id=NULL", [])
        .unwrap();
    let (valid, _) = inventory_snapshots(&snapshots).unwrap();
    assert_eq!(valid.len(), 1);
    assert_eq!(
        fs::read_dir(&snapshots).unwrap().count(),
        1,
        "Temporary failures are cleaned up, never finalized"
    );
}

#[test]
fn destination_failure_never_undoes_edit_and_retry_changes_no_application_data() {
    let directory = tempfile::tempdir().unwrap();
    let db = directory.path().join("work.sqlite3");
    let snapshots = directory.path().join("blocked-backups");
    fs::write(&snapshots, b"synthetic destination blocker").unwrap();
    let mut service = WorkspaceService::open(&db, &snapshots).unwrap();
    let saved = project(&mut service);
    let failure = wait_for(&service);
    assert_eq!(failure.state, BackupState::Warning);
    assert_eq!(failure.snapshot_count, 0);
    assert!(failure.warning.is_some());
    assert!(
        !serde_json::to_string(&failure)
            .unwrap()
            .contains(directory.path().to_str().unwrap())
    );
    assert_eq!(service.load_workspace().unwrap(), saved);
    fs::remove_file(&snapshots).unwrap();
    service.retry_backup();
    assert_eq!(wait_for(&service).state, BackupState::Ready);
    assert_eq!(service.load_workspace().unwrap(), saved);
    let connection = Connection::open(&db).unwrap();
    assert_eq!(read_workspace(&connection).unwrap(), saved);
}

#[test]
fn retry_does_not_claim_a_previously_verified_but_now_corrupt_file_is_available() {
    let directory = tempfile::tempdir().unwrap();
    let db = directory.path().join("work.sqlite3");
    let snapshots = directory.path().join("backups");
    let mut service = WorkspaceService::open(&db, &snapshots).unwrap();
    let saved = project(&mut service);
    assert_eq!(wait_for(&service).snapshot_count, 1);
    let path = snapshots.join(filename(Local::now().date_naive()));
    fs::write(&path, b"synthetic corruption").unwrap();
    service.retry_backup();
    let status = wait_for(&service);
    assert_eq!(status.state, BackupState::Warning);
    assert_eq!(status.latest_successful_day, None);
    assert_eq!(status.snapshot_count, 0);
    assert_eq!(service.load_workspace().unwrap(), saved);
    assert_eq!(fs::read(path).unwrap(), b"synthetic corruption");
}

#[test]
fn concurrent_snapshot_requests_finalize_at_most_one_file_per_local_day() {
    let directory = tempfile::tempdir().unwrap();
    let db = directory.path().join("work.sqlite3");
    let snapshots = directory.path().join("backups");
    let _service = WorkspaceService::open(&db, &directory.path().join("automatic")).unwrap();
    thread::scope(|scope| {
        let first = scope.spawn(|| create_snapshot(&db, &snapshots, day("2026-10-01")));
        let second = scope.spawn(|| create_snapshot(&db, &snapshots, day("2026-10-01")));
        first.join().unwrap().unwrap();
        second.join().unwrap().unwrap();
    });
    assert_eq!(inventory_snapshots(&snapshots).unwrap().0.len(), 1);
    assert_eq!(fs::read_dir(&snapshots).unwrap().count(), 1);
}

#[test]
#[cfg(windows)]
fn exclusively_locked_daily_destination_is_never_replaced_or_pruned() {
    use std::os::windows::fs::OpenOptionsExt;
    let directory = tempfile::tempdir().unwrap();
    let db = directory.path().join("work.sqlite3");
    let snapshots = directory.path().join("backups");
    let _service = WorkspaceService::open(&db, &directory.path().join("automatic")).unwrap();
    create_snapshot(&db, &snapshots, day("2026-10-01")).unwrap();
    let path = snapshots.join(filename(day("2026-10-01")));
    let before = fs::read(&path).unwrap();
    let lock = OpenOptions::new()
        .read(true)
        .write(true)
        .share_mode(0)
        .open(&path)
        .unwrap();
    assert!(create_snapshot(&db, &snapshots, day("2026-10-01")).is_err());
    assert!(latest_export(&snapshots).is_err());
    drop(lock);
    assert_eq!(fs::read(&path).unwrap(), before);
    assert_eq!(inventory_snapshots(&snapshots).unwrap().0.len(), 1);
}

#[test]
#[cfg(windows)]
fn recovery_does_not_follow_a_directory_junction() {
    let directory = tempfile::tempdir().unwrap();
    let db = directory.path().join("work.sqlite3");
    let target = directory.path().join("user-owned-synthetic-directory");
    let link = directory.path().join("backups-junction");
    fs::create_dir(&target).unwrap();
    fs::write(target.join("preserve.txt"), b"synthetic user-created file").unwrap();
    let created = std::process::Command::new("powershell.exe")
        .args(["-NoProfile", "-NonInteractive", "-Command", "New-Item -ItemType Junction -Path $env:ASCENSION_SYNTHETIC_LINK -Target $env:ASCENSION_SYNTHETIC_TARGET -ErrorAction Stop | Out-Null"])
        .env("ASCENSION_SYNTHETIC_LINK", &link)
        .env("ASCENSION_SYNTHETIC_TARGET", &target)
        .output().unwrap();
    assert!(
        created.status.success(),
        "Synthetic junction fixture must be created"
    );
    let _service = WorkspaceService::open(&db, &directory.path().join("automatic")).unwrap();
    assert!(create_snapshot(&db, &link, day("2026-10-01")).is_err());
    assert!(inventory_snapshots(&link).is_err());
    assert!(latest_export(&link).is_err());
    assert_eq!(fs::read_dir(&target).unwrap().count(), 1);
    assert_eq!(
        fs::read(target.join("preserve.txt")).unwrap(),
        b"synthetic user-created file"
    );
    // Remove the link itself; never recurse through its target.
    fs::remove_dir(&link).unwrap();
}

fn reward(service: &mut WorkspaceService, parent: &str) -> Workspace {
    let mut w = service.load_workspace().unwrap();
    let criterion = w
        .progress
        .criteria
        .iter()
        .find(|c| c.parent_id == parent)
        .unwrap()
        .clone();
    w = service
        .lock_criterion(LockCriterionInput {
            expected_generation: w.generation,
            parent_id: parent.into(),
            criterion_lineage_id: criterion.criterion_lineage_id.clone(),
            criterion_version: criterion.criterion_version,
        })
        .unwrap();
    w = service
        .record_evidence(RecordEvidenceInput {
            expected_generation: w.generation,
            parent_id: parent.into(),
            criterion_lineage_id: criterion.criterion_lineage_id.clone(),
            criterion_version: criterion.criterion_version,
            kind: EvidenceKind::UserAttestation,
            description: "Synthetic explicitly attested result".into(),
            reference: None,
            observed_at: None,
            status: EvidenceStatus::Supports,
            assistance: Assistance::None,
            uncertainty: None,
            supersedes: None,
        })
        .unwrap();
    let reviewed = service
        .review_progress(ReviewProgressInput {
            expected_generation: w.generation,
            parent_id: parent.into(),
            criterion_lineage_id: criterion.criterion_lineage_id.clone(),
            criterion_version: criterion.criterion_version,
            evidence_refs: w
                .progress
                .evidence
                .iter()
                .filter(|e| e.parent_id == parent)
                .map(|e| e.id.clone())
                .collect(),
            factual_summary: "Synthetic declared outcome review".into(),
            assistance: Assistance::None,
        })
        .unwrap();
    service
        .accept_progress(AcceptProgressInput {
            schema_version: 1,
            command_id: new_id(),
            expected_generation: reviewed.workspace.generation,
            parent_id: parent.into(),
            criterion_lineage_id: criterion.criterion_lineage_id,
            criterion_version: criterion.criterion_version,
            expected_parent_revision: reviewed
                .workspace
                .objectives
                .iter()
                .find(|o| o.id == parent)
                .unwrap()
                .revision,
            reviewed_snapshot_id: reviewed.snapshot.id,
            reviewed_snapshot_digest: reviewed.snapshot.digest,
            evidence_refs: reviewed.snapshot.evidence_refs,
            factual_summary: reviewed.snapshot.factual_summary,
            assistance: Assistance::None,
        })
        .unwrap()
        .workspace
}

fn world(service: &mut WorkspaceService, operation: WorldOperation) -> Workspace {
    let w = service.load_workspace().unwrap();
    service
        .world_command(WorldCommandInput {
            command_id: new_id(),
            schema_version: 1,
            content_version: CONTENT_VERSION.into(),
            expected_world_revision: w
                .world
                .chapter
                .as_ref()
                .map_or(0, |chapter| chapter.revision),
            chapter_id: w.world.chapter.map(|chapter| chapter.id),
            operation,
        })
        .unwrap()
        .workspace
}

#[test]
fn complete_recovery_preserves_corrected_rewards_study_and_world_without_reminting() {
    let directory = tempfile::tempdir().unwrap();
    let db = directory.path().join("source.sqlite3");
    let snapshots = directory.path().join("recovery");
    let mut source = WorkspaceService::open(&db, &directory.path().join("source-backups")).unwrap();
    let w = restore(
        &mut source,
        include_bytes!("../../contracts/fixtures/sample-v1.json"),
    );
    let parent = w.objectives[0].id.clone();
    let w = source
        .create_objective(CreateObjectiveInput {
            expected_generation: w.generation,
            project_id: w.projects[0].id.clone(),
            title: "Synthetic second bounded objective".into(),
            criterion: "Synthetic result exists".into(),
        })
        .unwrap();
    let second = w.selected_objective_id.unwrap();
    reward(&mut source, &parent);
    let mut w = reward(&mut source, &second);
    assert_eq!(w.progress.wallet_bc, 8);
    w = source
        .correct_acceptance(CorrectAcceptanceInput {
            expected_generation: w.generation,
            acceptance_id: w.progress.acceptances[0].id.clone(),
            kind: CorrectionKind::HistoricalReopened,
            reason: "Synthetic factual correction before recovery".into(),
        })
        .unwrap();
    w = source
        .create_schedule_epoch(CreateScheduleEpochInput {
            expected_generation: w.generation,
            label: "Synthetic recovery schedule".into(),
            effective_from: (Utc::now() - ChronoDuration::days(1))
                .to_rfc3339_opts(SecondsFormat::Millis, true),
            timezone: "Etc/UTC".into(),
        })
        .unwrap();
    w = source
        .configure_study(ConfigureStudyInput {
            expected_generation: w.generation,
            phase: Phase::E01,
            epoch_id: w.study.epochs[0].id.clone(),
        })
        .unwrap();
    source
        .plan_opportunity(PlanOpportunityInput {
            expected_generation: w.generation,
            objective_id: parent,
            epoch_id: w.study.epochs[0].id.clone(),
            manifest_id: None,
            domain: "Synthetic work".into(),
            meaningful_start: "Record first synthetic example".into(),
            cue_at: (Utc::now() + ChronoDuration::days(1))
                .to_rfc3339_opts(SecondsFormat::Millis, true),
            timezone: "Etc/UTC".into(),
            window_seconds: 900,
            eligible: true,
            unavailability: None,
            priority: Priority::Ordinary,
            available_minutes: 30,
            deadline_at: None,
            context: None,
        })
        .unwrap();
    world(&mut source, WorldOperation::Initialize);
    world(
        &mut source,
        WorldOperation::Queue {
            building_type: BuildingType::SurveyLodge,
            parcels: vec![Site::A1],
            reserve_available: true,
        },
    );
    world(&mut source, WorldOperation::CompleteConstruction);
    let before = world(
        &mut source,
        WorldOperation::Queue {
            building_type: BuildingType::Observatory,
            parcels: vec![Site::A2],
            reserve_available: true,
        },
    );
    assert_eq!(source.load_world().unwrap().bank.spent_bc, 4);
    assert_eq!(source.load_world().unwrap().bank.reserved_bc, 4);
    create_snapshot(&db, &snapshots, day("2026-10-01")).unwrap();
    let (bytes, _) = latest_export(&snapshots).unwrap();
    assert_eq!(bytes, source.export_bytes().unwrap());
    // Quick-check and foreign keys alone cannot establish accounting authority.
    // Alter only disposable copies, preserving syntactically valid SQLite rows.
    for (label, mutation) in [
        (
            "reward",
            "DROP TRIGGER ledger_immutable; UPDATE ledger_entries SET payload=json_set(payload,'$.amount_bc',3) WHERE sequence=1;",
        ),
        (
            "world",
            "DROP TRIGGER world_spending_immutable; UPDATE world_spending SET payload=json_set(payload,'$.allocations[0].amount_bc',3);",
        ),
    ] {
        let tampered = directory.path().join(format!("tampered-{label}.sqlite3"));
        fs::copy(snapshots.join(filename(day("2026-10-01"))), &tampered).unwrap();
        let connection = Connection::open(&tampered).unwrap();
        connection.execute_batch(mutation).unwrap();
        assert_eq!(
            connection
                .query_row("PRAGMA quick_check", [], |row| row.get::<_, String>(0))
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
        drop(connection);
        assert!(inspect_snapshot(&tampered, day("2026-10-01")).is_err());
        let rejected = directory.path().join(format!("rejected-{label}"));
        assert!(create_snapshot(&tampered, &rejected, day("2026-10-03")).is_err());
        assert_eq!(fs::read_dir(&rejected).unwrap().count(), 0);
    }
    // Recovery is available even if the original database is now unavailable.
    drop(source);
    fs::write(&db, b"synthetic unavailable primary database").unwrap();
    assert_eq!(latest_export(&snapshots).unwrap().0, bytes);
    let target_db = directory.path().join("recovered.sqlite3");
    let mut target = WorkspaceService::open(&target_db, &snapshots).unwrap();
    let empty = target.load_workspace().unwrap();
    let preview = target.preview_recovery().unwrap();
    assert_eq!(
        target.load_workspace().unwrap(),
        empty,
        "Preview is never destructive"
    );
    let result = target
        .restore_import(RestoreInput {
            token: preview.token,
            expected_generation: preview.generation,
        })
        .unwrap();
    assert!(result.backup_created);
    assert_eq!(target.export_bytes().unwrap(), bytes);
    assert_eq!(result.workspace.progress, before.progress);
    assert_eq!(result.workspace.study, before.study);
    assert_eq!(result.workspace.world, before.world);
    let original_acceptance = result.workspace.progress.commands[0].payload.clone();
    target.accept_progress(original_acceptance).unwrap();
    assert_eq!(
        target.export_bytes().unwrap(),
        bytes,
        "Corrected acceptance replay must not mint again"
    );
    drop(target);
    let reopened = WorkspaceService::open(&target_db, &snapshots).unwrap();
    assert_eq!(reopened.export_bytes().unwrap(), bytes);
    assert_eq!(reopened.load_world().unwrap().bank.spent_bc, 4);
    assert_eq!(reopened.load_world().unwrap().bank.reserved_bc, 4);
}
