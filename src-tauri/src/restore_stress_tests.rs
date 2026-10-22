//! Bounded synthetic restore repetitions against real SQLite, including held WAL readers.
use crate::{
    backup::{self, BackupPhase, BackupState, test_gate},
    daily::*,
    domain::*,
    progress::*,
    service::WorkspaceService,
    study::*,
    world::*,
};
use chrono::{Duration as ChronoDuration, SecondsFormat, Utc};
use rusqlite::{Connection, OpenFlags};
use std::{
    fs,
    path::Path,
    sync::mpsc,
    thread,
    time::{Duration, Instant},
};

struct Fixture {
    complete: Vec<u8>,
    reserved: Vec<u8>,
    snapshot_name: String,
    snapshot: Vec<u8>,
}

fn import(service: &mut WorkspaceService, bytes: &[u8]) -> crate::domain::RestoreResult {
    let preview = service
        .preview_import(bytes, "synthetic-stress.json".into())
        .unwrap();
    service
        .restore_import(RestoreInput {
            token: preview.token,
            expected_generation: preview.generation,
        })
        .unwrap()
}

fn wait_for_protection(service: &WorkspaceService, expected: BackupState) {
    let deadline = Instant::now() + Duration::from_secs(20);
    loop {
        let status = service.backup_status();
        if status.state == expected {
            return;
        }
        assert!(
            Instant::now() < deadline,
            "backup did not reach {expected:?}; state={:?}",
            status.state
        );
        thread::sleep(Duration::from_millis(5));
    }
}

fn assert_sqlite(database: &Path) {
    let c = Connection::open_with_flags(database, OpenFlags::SQLITE_OPEN_READ_ONLY).unwrap();
    assert_eq!(
        c.query_row("PRAGMA quick_check", [], |r| r.get::<_, String>(0))
            .unwrap(),
        "ok"
    );
    assert_eq!(
        c.query_row("SELECT COUNT(*) FROM pragma_foreign_key_check", [], |r| r
            .get::<_, u32>(
            0
        ))
        .unwrap(),
        0
    );
}

fn diagnostics(directory: &Path) -> Vec<serde_json::Value> {
    fs::read_to_string(directory.join("restore-diagnostics-v1.jsonl"))
        .unwrap()
        .lines()
        .skip(1)
        .map(|line| serde_json::from_str(line).unwrap())
        .collect()
}

fn reward(service: &mut WorkspaceService, parent: &str) {
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
            description: "Synthetic declared result exists.".into(),
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
            factual_summary: "Synthetic criterion and evidence reviewed.".into(),
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
        .unwrap();
}

fn world(service: &mut WorkspaceService, operation: WorldOperation) {
    let w = service.load_workspace().unwrap();
    service
        .world_command(WorldCommandInput {
            command_id: new_id(),
            schema_version: 1,
            content_version: CONTENT_VERSION.into(),
            expected_world_revision: w.world.chapter.as_ref().map_or(0, |c| c.revision),
            chapter_id: w.world.chapter.map(|c| c.id),
            operation,
        })
        .unwrap();
}

fn transition(service: &mut WorkspaceService, manifest: &str, status: StudyStatus) {
    service
        .transition_study(TransitionStudyInput {
            expected_generation: service.load_workspace().unwrap().generation,
            manifest_id: manifest.into(),
            status,
            reason: "Synthetic isolated feasibility decision.".into(),
            acknowledgements: if status == StudyStatus::Active {
                [
                    "e00_assessed",
                    "frozen_protocol",
                    "schedule_epoch",
                    "assignment_and_window",
                    "matched_support",
                    "as_assigned_analysis",
                    "voluntary_stop",
                ]
                .into_iter()
                .map(str::to_owned)
                .collect()
            } else {
                vec![]
            },
        })
        .unwrap();
}

fn fixture() -> Fixture {
    let directory = tempfile::tempdir().unwrap();
    let database = directory.path().join("source.sqlite3");
    let backups = directory.path().join("backups");
    let mut service = WorkspaceService::open(&database, &backups).unwrap();
    let w = service
        .create_project(CreateProjectInput {
            expected_generation: service.load_workspace().unwrap().generation,
            title: "Synthetic restore stress project".into(),
        })
        .unwrap();
    let project = w.projects[0].id.clone();
    let mut parents = Vec::new();
    for index in 0..6 {
        let w = service
            .create_objective(CreateObjectiveInput {
                expected_generation: service.load_workspace().unwrap().generation,
                project_id: project.clone(),
                title: format!("Synthetic bounded objective {index}"),
                criterion: "One synthetic example is recorded and reviewed.".into(),
            })
            .unwrap();
        let parent = w.selected_objective_id.unwrap();
        service
            .save_work(SaveWorkInput {
                expected_generation: w.generation,
                objective_id: parent.clone(),
                description: "Record the next synthetic example.".into(),
                stop_condition: "One example is recorded.".into(),
                note: Some("Resume at the next synthetic row.".into()),
            })
            .unwrap();
        reward(&mut service, &parent);
        parents.push(parent);
    }
    let w = service.load_workspace().unwrap();
    service
        .correct_acceptance(CorrectAcceptanceInput {
            expected_generation: w.generation,
            acceptance_id: w.progress.acceptances[0].id.clone(),
            kind: CorrectionKind::HistoricalReopened,
            reason: "Synthetic correction retains original issuance.".into(),
        })
        .unwrap();
    let w = service.load_workspace().unwrap();
    let action = w
        .actions
        .iter()
        .find(|a| a.objective_id == parents[0])
        .unwrap();
    let input = RecordCheckpointInput {
        expected_generation: w.generation.clone(),
        command_id: new_id(),
        objective_id: parents[0].clone(),
        note: "Recorded the synthetic example.".into(),
        reference: Some("synthetic-example.txt".into()),
        description: action.description.clone(),
        stop_condition: action.stop_condition.clone(),
        resumption_note: Some("Resume with the next synthetic example.".into()),
        supersedes: None,
        correction_reason: None,
    };
    let first = service.record_checkpoint(input.clone()).unwrap();
    service
        .record_checkpoint(RecordCheckpointInput {
            expected_generation: first.workspace.generation,
            command_id: new_id(),
            note: "Corrected the synthetic factual account.".into(),
            supersedes: Some(first.checkpoint.id),
            correction_reason: Some("Synthetic transcription correction.".into()),
            ..input
        })
        .unwrap();
    let w = service.load_workspace().unwrap();
    service
        .reorder_objectives(ReorderObjectivesInput {
            expected_generation: w.generation,
            objective_ids: parents.iter().rev().cloned().collect(),
        })
        .unwrap();
    for (parent, status) in [
        (&parents[4], ObjectiveStatus::Paused),
        (&parents[5], ObjectiveStatus::Archived),
    ] {
        service
            .set_objective_status(SetObjectiveStatusInput {
                expected_generation: service.load_workspace().unwrap().generation,
                objective_id: parent.clone(),
                status,
            })
            .unwrap();
    }
    let w = service
        .create_schedule_epoch(CreateScheduleEpochInput {
            expected_generation: service.load_workspace().unwrap().generation,
            label: "Synthetic frozen schedule".into(),
            effective_from: (Utc::now() - ChronoDuration::days(1))
                .to_rfc3339_opts(SecondsFormat::Millis, true),
            timezone: "Etc/UTC".into(),
        })
        .unwrap();
    let epoch = w.study.epochs[0].id.clone();
    let w = service
        .configure_study(ConfigureStudyInput {
            expected_generation: w.generation,
            phase: Phase::E01,
            epoch_id: epoch.clone(),
        })
        .unwrap();
    let manifest = w.study.manifests[0].id.clone();
    transition(&mut service, &manifest, StudyStatus::Ready);
    transition(&mut service, &manifest, StudyStatus::Active);
    for index in 0..4 {
        service
            .plan_opportunity(PlanOpportunityInput {
                expected_generation: service.load_workspace().unwrap().generation,
                objective_id: parents[0].clone(),
                epoch_id: epoch.clone(),
                manifest_id: Some(manifest.clone()),
                domain: "Synthetic engineering".into(),
                meaningful_start: "Write the first synthetic example label.".into(),
                cue_at: (Utc::now()
                    + ChronoDuration::days(1)
                    + ChronoDuration::minutes(index * 30))
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
    }
    transition(&mut service, &manifest, StudyStatus::Paused);
    world(&mut service, WorldOperation::Initialize);
    let layout = [
        (BuildingType::SurveyLodge, Site::A1),
        (BuildingType::SurveyLodge, Site::A2),
        (BuildingType::Observatory, Site::A3),
        (BuildingType::Observatory, Site::B1),
        (BuildingType::CaravanHouse, Site::B2),
        (BuildingType::CaravanHouse, Site::B3),
    ];
    let mut reserved = Vec::new();
    for (index, (building_type, site)) in layout.into_iter().enumerate() {
        world(
            &mut service,
            WorldOperation::Queue {
                building_type,
                parcels: vec![site],
                reserve_available: true,
            },
        );
        if index == 1 {
            reserved = service.export_bytes().unwrap();
        }
        world(&mut service, WorldOperation::CompleteConstruction);
    }
    world(
        &mut service,
        WorldOperation::SelectRoute {
            route: Route::BeaconMesh,
        },
    );
    world(&mut service, WorldOperation::CompleteChapter);
    let complete = service.export_bytes().unwrap();
    let w = service.load_workspace().unwrap();
    assert_eq!(w.progress.wallet_bc, 24);
    assert_eq!(w.progress.grants.len(), 6);
    assert_eq!(w.study.assignments.len(), 4);
    assert_eq!(w.daily.checkpoints.len(), 2);
    assert!(w.world.chapter.unwrap().completion.is_some());
    drop(service);
    let point = fs::read_dir(&backups)
        .unwrap()
        .map(|e| e.unwrap().path())
        .find(|p| {
            p.file_name()
                .unwrap()
                .to_string_lossy()
                .starts_with("daily-v1-")
        })
        .unwrap();
    Fixture {
        complete,
        reserved,
        snapshot_name: point.file_name().unwrap().to_string_lossy().into_owned(),
        snapshot: fs::read(point).unwrap(),
    }
}

#[test]
fn twenty_five_fresh_complete_restore_restart_and_recovery_cycles_preserve_all_authority() {
    let data = fixture();
    for iteration in 0..25 {
        let directory = tempfile::tempdir().unwrap();
        let database = directory.path().join("workspace.sqlite3");
        let backups = directory.path().join("backups");
        crate::restore_diagnostics::with_test_sink(directory.path(), || {
            let mut service = WorkspaceService::open(&database, &backups).unwrap();
            assert!(import(&mut service, &data.complete).backup_created);
            assert_eq!(
                service.export_bytes().unwrap(),
                data.complete,
                "fresh import {iteration}"
            );
            drop(service);
            let mut service = WorkspaceService::open(&database, &backups).unwrap();
            assert_eq!(service.export_bytes().unwrap(), data.complete);
            let generation = service.load_workspace().unwrap().generation;
            assert!(import(&mut service, &data.complete).unchanged);
            assert_eq!(service.load_workspace().unwrap().generation, generation);
            import(&mut service, &data.reserved);
            assert_eq!(service.load_world().unwrap().bank.reserved_bc, 4);
            assert_eq!(service.load_world().unwrap().bank.spent_bc, 4);
            drop(service);
            let mut service = WorkspaceService::open(&database, &backups).unwrap();
            assert_eq!(service.export_bytes().unwrap(), data.reserved);
            let preview = service.preview_recovery().unwrap();
            service
                .restore_import(RestoreInput {
                    token: preview.token,
                    expected_generation: preview.generation,
                })
                .unwrap();
            assert_eq!(
                service.export_bytes().unwrap(),
                data.complete,
                "snapshot recovery {iteration}"
            );
            let w = service.load_workspace().unwrap();
            let corrected = &w.progress.corrections[0].acceptance_id;
            let replay = w
                .progress
                .commands
                .iter()
                .find(|c| &c.result.acceptance_id == corrected)
                .unwrap()
                .payload
                .clone();
            service.accept_progress(replay).unwrap();
            service
                .record_checkpoint(w.daily.commands[0].input.clone())
                .unwrap();
            assert_eq!(
                service.export_bytes().unwrap(),
                data.complete,
                "no remint or checkpoint duplication {iteration}"
            );
            drop(service);
            assert_sqlite(&database);
            let service = WorkspaceService::open(&database, &backups).unwrap();
            assert_eq!(service.export_bytes().unwrap(), data.complete);
        });
        let entries = diagnostics(directory.path());
        assert_eq!(entries.len(), 4);
        assert_eq!(
            entries.iter().filter(|e| e["outcome"] == "success").count(),
            3
        );
        assert_eq!(
            entries
                .iter()
                .filter(|e| e["outcome"] == "unchanged")
                .count(),
            1
        );
        assert!(
            entries
                .iter()
                .all(|e| e["transaction_outcome"] == "committed")
        );
    }
    eprintln!(
        "RESTORE_STRESS {{\"scenario\":\"fresh_restart_recovery\",\"cycles\":25,\"replacement_successes\":75,\"noop_successes\":25,\"unexpected_failures\":0}}"
    );
}

#[test]
fn five_failed_backup_and_sql_restore_attempts_preserve_state_then_retry_exactly() {
    let data = fixture();
    for iteration in 0..5 {
        let directory = tempfile::tempdir().unwrap();
        let database = directory.path().join("workspace.sqlite3");
        let backups = directory.path().join("backups");
        crate::restore_diagnostics::with_test_sink(directory.path(), || {
            fs::write(&backups, b"Synthetic unavailable backup destination").unwrap();
            let mut service = WorkspaceService::open(&database, &backups).unwrap();
            service
                .create_project(CreateProjectInput {
                    expected_generation: service.load_workspace().unwrap().generation,
                    title: "Synthetic preserved local project".into(),
                })
                .unwrap();
            wait_for_protection(&service, BackupState::Warning);
            let before = service.load_workspace().unwrap();
            let before_bytes = service.export_bytes().unwrap();
            let preview = service
                .preview_import(&data.complete, "synthetic-stress.json".into())
                .unwrap();
            let input = RestoreInput {
                token: preview.token,
                expected_generation: preview.generation,
            };
            assert_eq!(
                service.restore_import(input.clone()).unwrap_err().code,
                "backup_failed"
            );
            assert_eq!(service.load_workspace().unwrap(), before);
            assert_eq!(service.export_bytes().unwrap(), before_bytes);
            fs::remove_file(&backups).unwrap();
            fs::create_dir(&backups).unwrap();
            service.restore_import(input).unwrap();
            assert_eq!(service.export_bytes().unwrap(), data.complete);
            let w = service.load_workspace().unwrap();
            service.retry_backup();
            wait_for_protection(&service, BackupState::Ready);
            assert_eq!(service.load_workspace().unwrap(), w);
            let connection = Connection::open(&database).unwrap();
            connection.execute_batch("CREATE TRIGGER synthetic_restore_failure BEFORE INSERT ON work_checkpoints WHEN NEW.sequence=2 BEGIN SELECT RAISE(ABORT,'synthetic restore constraint'); END;").unwrap();
            let preview = service
                .preview_import(&data.reserved, "synthetic-reservation.json".into())
                .unwrap();
            let input = RestoreInput {
                token: preview.token,
                expected_generation: preview.generation,
            };
            assert_eq!(
                service.restore_import(input.clone()).unwrap_err().code,
                "storage_error"
            );
            assert_eq!(service.load_workspace().unwrap(), w, "rollback {iteration}");
            assert_eq!(service.export_bytes().unwrap(), data.complete);
            assert!(
                fs::read_dir(&backups)
                    .unwrap()
                    .map(|e| e.unwrap().path())
                    .filter(|p| p.extension().is_some_and(|e| e == "json"))
                    .any(|p| fs::read(p).unwrap() == data.complete)
            );
            connection
                .execute_batch("DROP TRIGGER synthetic_restore_failure;")
                .unwrap();
            drop(connection);
            service.restore_import(input).unwrap();
            assert_eq!(service.export_bytes().unwrap(), data.reserved);
            drop(service);
            assert_sqlite(&database);
            assert_eq!(
                WorkspaceService::open(&database, &backups)
                    .unwrap()
                    .export_bytes()
                    .unwrap(),
                data.reserved
            );
        });
        let entries = diagnostics(directory.path());
        assert_eq!(entries.len(), 4);
        assert_eq!(entries[0]["outcome"], "error");
        assert_eq!(entries[0]["command_error_code"], "backup_failed");
        assert_eq!(entries[0]["transaction_outcome"], "rolled_back");
        assert_eq!(entries[0]["displaced_backup_created"], false);
        assert!(!entries[0]["filesystem_category"].is_null());
        assert_eq!(entries[1]["outcome"], "success");
        assert_eq!(entries[2]["outcome"], "error");
        assert_eq!(entries[2]["operation"], "work_checkpoints");
        assert_eq!(entries[2]["sqlite_primary_code"], 19);
        assert_eq!(entries[2]["sqlite_extended_code"], 1811);
        assert_eq!(entries[2]["transaction_outcome"], "rolled_back");
        assert_eq!(entries[2]["displaced_backup_created"], true);
        assert_eq!(entries[3]["outcome"], "success");
        eprintln!(
            "RESTORE_STRESS_CATEGORY {}",
            serde_json::json!({
                "iteration": iteration, "filesystem_category": entries[0]["filesystem_category"],
                "filesystem_os_code": entries[0]["os_error_code"], "sql_primary": entries[2]["sqlite_primary_code"],
                "sql_extended": entries[2]["sqlite_extended_code"], "preserved_state": true,
            })
        );
    }
    eprintln!(
        "RESTORE_STRESS {{\"scenario\":\"failure_retry\",\"cycles\":5,\"expected_backup_failures\":5,\"expected_sql_failures\":5,\"retry_successes\":10,\"unexpected_failures\":0}}"
    );
}

fn concurrent_restore(
    service: WorkspaceService,
    complete: Vec<u8>,
    gate: test_gate::Gate,
    diagnostic_directory: &Path,
    phase: BackupPhase,
) -> WorkspaceService {
    let (sender, receiver) = mpsc::channel();
    let path = diagnostic_directory.to_path_buf();
    let handle = thread::spawn(move || {
        let mut service = service;
        let preview = service
            .preview_import(&complete, "synthetic-overlap.json".into())
            .unwrap();
        let result = crate::restore_diagnostics::with_test_sink(&path, || {
            service.restore_import(RestoreInput {
                token: preview.token,
                expected_generation: preview.generation,
            })
        });
        sender.send(result.map(|r| r.backup_created)).unwrap();
        service
    });
    let result = receiver.recv_timeout(Duration::from_secs(10));
    // Always release before assertions/join, including an unexpected timeout.
    gate.release();
    let service = handle.join().unwrap();
    assert!(
        result
            .expect("restore must finish while the worker is held")
            .unwrap()
    );
    let entries = diagnostics(diagnostic_directory);
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0]["outcome"], "success");
    assert_eq!(entries[0]["transaction_outcome"], "committed");
    assert_eq!(
        entries[0]["backup_phase_start"],
        serde_json::to_value(phase).unwrap()
    );
    assert_eq!(
        entries[0]["backup_phase_end"],
        serde_json::to_value(phase).unwrap()
    );
    service
}

#[test]
fn five_restores_complete_during_pinned_inventory_and_five_during_pinned_snapshot() {
    let data = fixture();
    for iteration in 0..5 {
        for point in [
            test_gate::Point::Inventory,
            test_gate::Point::SnapshotPinned,
        ] {
            let directory = tempfile::tempdir().unwrap();
            let database = directory.path().join("workspace.sqlite3");
            let backups = directory.path().join("backups");
            if point == test_gate::Point::Inventory {
                fs::create_dir(&backups).unwrap();
                fs::write(backups.join(&data.snapshot_name), &data.snapshot).unwrap();
            }
            let registration = test_gate::install(&database, point);
            let mut service = WorkspaceService::open(&database, &backups).unwrap();
            // Release the held worker before the service joins it during unwinding.
            let gate = registration;
            if point == test_gate::Point::SnapshotPinned {
                service
                    .create_project(CreateProjectInput {
                        expected_generation: service.load_workspace().unwrap().generation,
                        title: "Synthetic before pinned WAL snapshot".into(),
                    })
                    .unwrap();
            }
            let reached = gate.wait_until_reached(Duration::from_secs(10));
            if !reached {
                gate.release();
            }
            assert!(reached, "worker gate {iteration}");
            let before = service.export_bytes().unwrap();
            let phase = if point == test_gate::Point::Inventory {
                BackupPhase::Inventory
            } else {
                BackupPhase::Snapshot
            };
            let service = concurrent_restore(
                service,
                data.complete.clone(),
                gate,
                directory.path(),
                phase,
            );
            assert_eq!(service.export_bytes().unwrap(), data.complete);
            wait_for_protection(&service, BackupState::Ready);
            if point == test_gate::Point::SnapshotPinned {
                assert_eq!(
                    backup::latest_export(&backups).unwrap().0,
                    before,
                    "snapshot remains the pinned pre-restore state"
                );
            } else {
                assert_eq!(
                    fs::read(backups.join(&data.snapshot_name)).unwrap(),
                    data.snapshot
                );
            }
            drop(service);
            assert_sqlite(&database);
            assert_eq!(
                WorkspaceService::open(&database, &backups)
                    .unwrap()
                    .export_bytes()
                    .unwrap(),
                data.complete
            );
        }
    }
    eprintln!(
        "RESTORE_STRESS {{\"scenario\":\"worker_overlap\",\"inventory_successes\":5,\"pinned_snapshot_successes\":5,\"unexpected_failures\":0}}"
    );
}
