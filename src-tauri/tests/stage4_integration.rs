//! Synthetic cross-stage migration, authority, and release measurements.
use ascension_lib::{domain::*, progress::*, service::WorkspaceService, study::*, world::*};
use chrono::{Duration, SecondsFormat, Utc};
use rusqlite::Connection;
use std::{fs, path::Path, time::Instant};
use uuid::Uuid;

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

fn reward(service: &mut WorkspaceService, mut w: Workspace, parent: &str) -> Workspace {
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
            description: "Synthetic result meets the declared acceptance criterion.".into(),
            reference: None,
            observed_at: None,
            status: EvidenceStatus::Supports,
            assistance: Assistance::None,
            uncertainty: None,
            supersedes: None,
        })
        .unwrap();
    let review = service
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
            factual_summary: "Reviewed synthetic outcome and its criterion.".into(),
            assistance: Assistance::None,
        })
        .unwrap();
    service
        .accept_progress(AcceptProgressInput {
            schema_version: 1,
            command_id: Uuid::new_v4().to_string(),
            expected_generation: review.workspace.generation,
            parent_id: parent.into(),
            criterion_lineage_id: criterion.criterion_lineage_id,
            criterion_version: criterion.criterion_version,
            expected_parent_revision: review
                .workspace
                .objectives
                .iter()
                .find(|o| o.id == parent)
                .unwrap()
                .revision,
            reviewed_snapshot_id: review.snapshot.id,
            reviewed_snapshot_digest: review.snapshot.digest,
            evidence_refs: review.snapshot.evidence_refs,
            factual_summary: review.snapshot.factual_summary,
            assistance: Assistance::None,
        })
        .unwrap()
        .workspace
}

fn fixture(service: &mut WorkspaceService, count: usize, opportunities: usize) -> Workspace {
    let mut records: RecordSet =
        serde_json::from_slice(include_bytes!("../../contracts/fixtures/sample-v1.json")).unwrap();
    let objective = records.objectives[0].clone();
    let action = records
        .actions
        .iter()
        .find(|a| a.objective_id == objective.id)
        .unwrap()
        .clone();
    records.objectives.clear();
    records.actions.clear();
    records.notes.clear();
    for index in 0..count {
        let mut row = objective.clone();
        row.id = Uuid::new_v4().to_string();
        row.title = format!("Synthetic River Reach objective {index}");
        let mut next = action.clone();
        next.id = Uuid::new_v4().to_string();
        next.objective_id = row.id.clone();
        records.objectives.push(row);
        records.actions.push(next);
    }
    records.selected_objective_id = Some(records.objectives[0].id.clone());
    let mut w = restore(service, &serde_json::to_vec(&records).unwrap());
    let parents: Vec<_> = w.objectives.iter().take(6).map(|o| o.id.clone()).collect();
    for parent in &parents {
        w = reward(service, w, parent);
    }
    w = service
        .create_schedule_epoch(CreateScheduleEpochInput {
            expected_generation: w.generation,
            label: "Synthetic migration routine".into(),
            effective_from: (Utc::now() - Duration::days(1))
                .to_rfc3339_opts(SecondsFormat::Millis, true),
            timezone: "Etc/UTC".into(),
        })
        .unwrap();
    // A frozen configured E01 manifest is preserved without activating a study.
    w = service
        .configure_study(ConfigureStudyInput {
            expected_generation: w.generation,
            phase: Phase::E01,
            epoch_id: w.study.epochs[0].id.clone(),
        })
        .unwrap();
    for index in 0..opportunities {
        w = service
            .plan_opportunity(PlanOpportunityInput {
                expected_generation: w.generation,
                objective_id: parents[0].clone(),
                epoch_id: w.study.epochs[0].id.clone(),
                manifest_id: None,
                domain: "synthetic engineering".into(),
                meaningful_start: "Record the first synthetic example result".into(),
                cue_at: (Utc::now() + Duration::days(1) + Duration::minutes(index as i64 * 30))
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
    w
}

fn as_v4_database(path: &Path) {
    let c = Connection::open(path).unwrap();
    c.execute_batch("PRAGMA foreign_keys=OFF; DROP TABLE world_commands; DROP TABLE world_spending; DROP TABLE world_chapter; PRAGMA user_version=4;").unwrap();
}

#[test]
fn populated_v4_upgrade_preserves_reward_and_frozen_study_graph_with_coherent_backup() {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("synthetic.sqlite3");
    let backups = dir.path().join("backups");
    let mut service = WorkspaceService::open(&db, &backups).unwrap();
    let before = fixture(&mut service, 6, 2);
    let complete = service.export_bytes().unwrap();
    drop(service);
    as_v4_database(&db);
    let service = WorkspaceService::open(&db, &backups).unwrap();
    assert_eq!(service.load_workspace().unwrap(), before);
    assert_eq!(service.export_bytes().unwrap(), complete);
    let c = Connection::open(&db).unwrap();
    assert_eq!(
        c.pragma_query_value(None, "user_version", |r| r.get::<_, u32>(0))
            .unwrap(),
        5
    );
    assert_eq!(
        c.query_row("SELECT count(*) FROM pragma_foreign_key_check", [], |r| r
            .get::<_, u32>(
            0
        ))
        .unwrap(),
        0
    );
    let migration = fs::read_dir(&backups)
        .unwrap()
        .map(|p| p.unwrap().path())
        .find(|p| p.extension().is_some_and(|e| e == "sqlite3"))
        .unwrap();
    let backup = Connection::open(migration).unwrap();
    assert_eq!(
        backup
            .pragma_query_value(None, "user_version", |r| r.get::<_, u32>(0))
            .unwrap(),
        4
    );
    assert_eq!(
        backup
            .query_row("SELECT SUM(amount_bc) FROM ledger_entries", [], |r| r
                .get::<_, u32>(0))
            .unwrap(),
        24
    );
    assert_eq!(
        backup
            .query_row("SELECT count(*) FROM work_opportunities", [], |r| r
                .get::<_, u32>(0))
            .unwrap(),
        2
    );
    assert_eq!(
        backup
            .query_row("PRAGMA quick_check", [], |r| r.get::<_, String>(0))
            .unwrap(),
        "ok"
    );
}

#[test]
fn v4_upgrade_backup_failure_preserves_original_database_and_retries_safely() {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("synthetic.sqlite3");
    let backups = dir.path().join("backups");
    let mut service = WorkspaceService::open(&db, &backups).unwrap();
    let before = fixture(&mut service, 6, 1);
    drop(service);
    as_v4_database(&db);
    let blocked = dir.path().join("backup-is-a-file");
    fs::write(&blocked, b"synthetic blocker").unwrap();
    assert_eq!(
        WorkspaceService::open(&db, &blocked).err().unwrap().code,
        "backup_failed"
    );
    let c = Connection::open(&db).unwrap();
    assert_eq!(
        c.pragma_query_value(None, "user_version", |r| r.get::<_, u32>(0))
            .unwrap(),
        4
    );
    assert_eq!(
        c.query_row(
            "SELECT count(*) FROM sqlite_master WHERE name='world_chapter'",
            [],
            |r| r.get::<_, u32>(0)
        )
        .unwrap(),
        0
    );
    drop(c);
    assert_eq!(
        WorkspaceService::open(&db, &backups)
            .unwrap()
            .load_workspace()
            .unwrap(),
        before
    );
}

#[test]
fn v4_upgrade_ddl_failure_rolls_back_new_tables_and_preserves_authority() {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("synthetic.sqlite3");
    let backups = dir.path().join("backups");
    let mut service = WorkspaceService::open(&db, &backups).unwrap();
    let before = fixture(&mut service, 6, 1);
    drop(service);
    as_v4_database(&db);
    let c = Connection::open(&db).unwrap();
    c.execute_batch("CREATE TABLE world_spending(synthetic_marker TEXT); INSERT INTO world_spending VALUES ('retain');").unwrap();
    drop(c);
    assert!(WorkspaceService::open(&db, &backups).is_err());
    let c = Connection::open(&db).unwrap();
    assert_eq!(
        c.pragma_query_value(None, "user_version", |r| r.get::<_, u32>(0))
            .unwrap(),
        4
    );
    assert_eq!(
        c.query_row(
            "SELECT count(*) FROM sqlite_master WHERE name='world_chapter'",
            [],
            |r| r.get::<_, u32>(0)
        )
        .unwrap(),
        0
    );
    assert_eq!(
        c.query_row("SELECT synthetic_marker FROM world_spending", [], |r| {
            r.get::<_, String>(0)
        })
        .unwrap(),
        "retain"
    );
    assert_eq!(
        c.query_row("SELECT SUM(amount_bc) FROM ledger_entries", [], |r| r
            .get::<_, u32>(0))
            .unwrap(),
        24
    );
    c.execute_batch("DROP TABLE world_spending;").unwrap();
    drop(c);
    assert_eq!(
        WorkspaceService::open(&db, &backups)
            .unwrap()
            .load_workspace()
            .unwrap(),
        before
    );
}

fn command(service: &mut WorkspaceService, operation: WorldOperation) -> Workspace {
    let w = service.load_workspace().unwrap();
    let chapter = w.world.chapter.as_ref();
    service
        .world_command(WorldCommandInput {
            command_id: Uuid::new_v4().to_string(),
            schema_version: 1,
            content_version: CONTENT_VERSION.into(),
            expected_world_revision: chapter.map_or(0, |c| c.revision),
            chapter_id: chapter.map(|c| c.id.clone()),
            operation,
        })
        .unwrap()
        .workspace
}

fn measure<T>(size: usize, name: &str, f: impl FnOnce() -> T) -> T {
    let start = Instant::now();
    let result = f();
    let elapsed = start.elapsed().as_secs_f64() * 1000.;
    println!("WORLD_BENCH,{size},{name},{elapsed:.3}");
    assert!(elapsed < 60_000., "Severe bounded operation regression");
    result
}

#[test]
#[ignore = "explicit synthetic release benchmark; prints single-operation timings"]
fn river_reach_service_performance_with_work_study_and_world_history() {
    for size in [10, 100, 500, 1000] {
        let dir = tempfile::tempdir().unwrap();
        let db = dir.path().join("synthetic.sqlite3");
        let backups = dir.path().join("backups");
        let mut service = WorkspaceService::open(&db, &backups).unwrap();
        fixture(&mut service, size, 10);
        measure(size, "work_load", || service.load_workspace().unwrap());
        measure(size, "world_open", || service.load_world().unwrap());
        measure(size, "initialize", || {
            command(&mut service, WorldOperation::Initialize)
        });
        let placements = [
            (BuildingType::Foundry, vec![Site::A1, Site::A2]),
            (BuildingType::Foundry, vec![Site::B1, Site::B2]),
            (BuildingType::GrandArchive, vec![Site::A3, Site::B3]),
        ];
        for (kind, parcels) in placements {
            measure(size, "queue", || {
                command(
                    &mut service,
                    WorldOperation::Queue {
                        building_type: kind,
                        parcels,
                        reserve_available: false,
                    },
                )
            });
            measure(size, "reserve", || {
                command(&mut service, WorldOperation::Reserve)
            });
            measure(size, "complete_construction", || {
                command(&mut service, WorldOperation::CompleteConstruction)
            });
        }
        let w = service.load_workspace().unwrap();
        let chapter = w.world.chapter.as_ref().unwrap();
        let mut moved: Vec<_> = chapter
            .layout
            .iter()
            .map(|p| PlacementInput {
                id: Some(p.id.clone()),
                building_type: p.building_type,
                parcels: p.parcels.clone(),
            })
            .collect();
        let foundries: Vec<_> = moved
            .iter()
            .enumerate()
            .filter(|(_, p)| p.building_type == BuildingType::Foundry)
            .map(|(i, _)| i)
            .collect();
        let first = moved[foundries[0]].parcels.clone();
        moved[foundries[0]].parcels = moved[foundries[1]].parcels.clone();
        moved[foundries[1]].parcels = first;
        measure(size, "placement_preview", || {
            service
                .preview_world(WorldPreviewInput {
                    chapter_id: chapter.id.clone(),
                    content_version: CONTENT_VERSION.into(),
                    placements: moved.clone(),
                })
                .unwrap()
        });
        measure(size, "reconfigure", || {
            command(
                &mut service,
                WorldOperation::Reconfigure { placements: moved },
            )
        });
        measure(size, "undo", || command(&mut service, WorldOperation::Undo));
        command(
            &mut service,
            WorldOperation::SelectRoute {
                route: Route::StoneCauseway,
            },
        );
        measure(size, "complete_chapter", || {
            command(&mut service, WorldOperation::CompleteChapter)
        });
        let bytes = measure(size, "export", || service.export_bytes().unwrap());
        println!(
            "WORLD_FIXTURE,{size},bytes={},commands={}",
            bytes.len(),
            service.load_workspace().unwrap().world.commands.len()
        );
        measure(size, "import_preview", || {
            service
                .preview_import(&bytes, "synthetic.json".into())
                .unwrap()
        });
        let mut target = WorkspaceService::open(
            &dir.path().join("restore.sqlite3"),
            &dir.path().join("restore-backups"),
        )
        .unwrap();
        measure(size, "restore_empty", || restore(&mut target, &bytes));
        assert_eq!(target.export_bytes().unwrap(), bytes);
        drop(service);
        measure(size, "cold_repository_open", || {
            WorkspaceService::open(&db, &backups).unwrap()
        });
    }
}

#[test]
#[ignore = "explicit synthetic release benchmark for retained world command history"]
fn river_reach_retained_history_performance() {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("synthetic.sqlite3");
    let backups = dir.path().join("backups");
    let mut service = WorkspaceService::open(&db, &backups).unwrap();
    fixture(&mut service, 10, 10);
    command(&mut service, WorldOperation::Initialize);
    let mut decisions = 1;
    for size in [10, 100, 500, 1000] {
        while decisions < size {
            command(
                &mut service,
                WorldOperation::SelectRoute {
                    route: if decisions % 2 == 0 {
                        Route::StoneCauseway
                    } else {
                        Route::RiverCompact
                    },
                },
            );
            decisions += 1;
        }
        measure(size, "retained_history_work_load", || {
            service.load_workspace().unwrap()
        });
        measure(size, "retained_history_world_open", || {
            service.load_world().unwrap()
        });
        let bytes = measure(size, "retained_history_export", || {
            service.export_bytes().unwrap()
        });
        println!("WORLD_HISTORY,{size},bytes={}", bytes.len());
        measure(size, "retained_history_import_preview", || {
            service
                .preview_import(&bytes, "synthetic-history.json".into())
                .unwrap()
        });
    }
}
