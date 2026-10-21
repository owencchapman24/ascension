use super::*;
use crate::domain::*;
use crate::progress::*;
use crate::service::WorkspaceService;
use crate::study::{ConfigureStudyInput, CreateScheduleEpochInput, TransitionStudyInput};
use serde_json::{Value, json};
use std::path::PathBuf;
use std::sync::{Arc, Barrier};

struct Fixture {
    _directory: tempfile::TempDir,
    database: PathBuf,
    backups: PathBuf,
    service: WorkspaceService,
}
impl Fixture {
    fn new() -> Self {
        let directory = tempfile::tempdir().unwrap();
        let database = directory.path().join("synthetic.sqlite3");
        let backups = directory.path().join("backups");
        let mut service = WorkspaceService::open(&database, &backups).unwrap();
        let w = service.load_workspace().unwrap();
        service
            .create_project(CreateProjectInput {
                expected_generation: w.generation,
                title: "Synthetic river work".into(),
            })
            .unwrap();
        let mut f = Self {
            _directory: directory,
            database,
            backups,
            service,
        };
        f.run(WorldOperation::Initialize);
        f
    }
    fn ws(&self) -> Workspace {
        self.service.load_workspace().unwrap()
    }
    fn view(&self) -> WorldView {
        self.service.load_world().unwrap()
    }
    fn input(&self, operation: WorldOperation) -> WorldCommandInput {
        let w = self.ws();
        WorldCommandInput {
            command_id: new_id(),
            schema_version: 1,
            content_version: CONTENT_VERSION.into(),
            expected_world_revision: w.world.chapter.as_ref().map_or(0, |c| c.revision),
            chapter_id: w.world.chapter.as_ref().map(|c| c.id.clone()),
            operation,
        }
    }
    fn run(&mut self, operation: WorldOperation) -> WorldCommandResult {
        let input = self.input(operation);
        self.service.world_command(input).unwrap().result
    }
    fn reject(&mut self, operation: WorldOperation) -> CommandError {
        let before = self.service.export_bytes().unwrap();
        let input = self.input(operation);
        let error = self.service.world_command(input).unwrap_err();
        assert_eq!(self.service.export_bytes().unwrap(), before);
        error
    }
    fn earn(&mut self, amount: u32) -> String {
        let w = self.ws();
        let w = self
            .service
            .create_objective(CreateObjectiveInput {
                expected_generation: w.generation,
                project_id: w.projects[0].id.clone(),
                title: format!("Synthetic bounded outcome {}", w.objectives.len() + 1),
                criterion: "Record a synthetic observation and its limitation.".into(),
            })
            .unwrap();
        let id = w.selected_objective_id.clone().unwrap();
        let mut criterion = w
            .progress
            .criteria
            .iter()
            .find(|c| c.parent_id == id)
            .unwrap()
            .clone();
        if amount != 4 {
            let w = self
                .service
                .save_criterion(SaveCriterionInput {
                    expected_generation: w.generation,
                    parent_id: id.clone(),
                    criterion_lineage_id: Some(criterion.criterion_lineage_id.clone()),
                    description: criterion.description.clone(),
                    allocation_bc: amount,
                    evidence_policy: criterion.evidence_policy.clone(),
                    scope: criterion.scope.clone(),
                    artifact_id: criterion.artifact_id.clone(),
                    artifact_version: criterion.artifact_version.clone(),
                    requires_independent_work: false,
                    required_validators: vec![],
                })
                .unwrap();
            criterion = w
                .progress
                .criteria
                .iter()
                .find(|c| c.parent_id == id)
                .unwrap()
                .clone();
        }
        let w = self
            .service
            .lock_criterion(LockCriterionInput {
                expected_generation: self.ws().generation,
                parent_id: id.clone(),
                criterion_lineage_id: criterion.criterion_lineage_id.clone(),
                criterion_version: criterion.criterion_version,
            })
            .unwrap();
        let w = self
            .service
            .record_evidence(RecordEvidenceInput {
                expected_generation: w.generation,
                parent_id: id.clone(),
                criterion_lineage_id: criterion.criterion_lineage_id.clone(),
                criterion_version: criterion.criterion_version,
                kind: EvidenceKind::UserAttestation,
                description: "The isolated synthetic observation and limitation were recorded."
                    .into(),
                reference: None,
                observed_at: None,
                status: EvidenceStatus::Supports,
                assistance: Assistance::None,
                uncertainty: None,
                supersedes: None,
            })
            .unwrap();
        let evidence = w
            .progress
            .evidence
            .iter()
            .find(|e| e.parent_id == id)
            .unwrap()
            .id
            .clone();
        let r = self
            .service
            .review_progress(ReviewProgressInput {
                expected_generation: w.generation,
                parent_id: id.clone(),
                criterion_lineage_id: criterion.criterion_lineage_id.clone(),
                criterion_version: criterion.criterion_version,
                evidence_refs: vec![evidence],
                factual_summary: "The synthetic bounded outcome is documented.".into(),
                assistance: Assistance::None,
            })
            .unwrap();
        let s = r.snapshot;
        self.service
            .accept_progress(AcceptProgressInput {
                schema_version: 1,
                command_id: new_id(),
                expected_generation: r.workspace.generation,
                parent_id: id.clone(),
                criterion_lineage_id: criterion.criterion_lineage_id,
                criterion_version: criterion.criterion_version,
                expected_parent_revision: r
                    .workspace
                    .objectives
                    .iter()
                    .find(|o| o.id == id)
                    .unwrap()
                    .revision,
                reviewed_snapshot_id: s.id,
                reviewed_snapshot_digest: s.digest,
                evidence_refs: s.evidence_refs,
                factual_summary: s.factual_summary,
                assistance: Assistance::None,
            })
            .unwrap()
            .result
            .acceptance_id
    }
    fn fund(&mut self, kind: BuildingType, sites: &[Site]) {
        self.run(WorldOperation::Queue {
            building_type: kind,
            parcels: sites.to_vec(),
            reserve_available: true,
        });
        self.run(WorldOperation::CompleteConstruction);
    }
    fn full(&mut self) {
        for _ in 0..6 {
            self.earn(4);
        }
        self.fund(BuildingType::Foundry, &[Site::A1, Site::B1]);
        self.fund(BuildingType::Foundry, &[Site::A2, Site::B2]);
        self.fund(BuildingType::GrandArchive, &[Site::A3, Site::B3]);
    }
    fn reopen(&mut self) {
        self.service = WorkspaceService::open(&self.database, &self.backups).unwrap();
    }
    fn restore(&mut self, bytes: &[u8]) {
        let p = self
            .service
            .preview_import(bytes, "synthetic.json".into())
            .unwrap();
        self.service
            .restore_import(RestoreInput {
                expected_generation: p.generation,
                token: p.token,
            })
            .unwrap();
    }
}
fn p(kind: BuildingType, sites: &[Site]) -> PlacementInput {
    PlacementInput {
        id: None,
        building_type: kind,
        parcels: sites.to_vec(),
    }
}
fn route_layout(route: Route) -> Vec<PlacementInput> {
    match route {
        Route::StoneCauseway => vec![
            p(BuildingType::Foundry, &[Site::A1, Site::B1]),
            p(BuildingType::Foundry, &[Site::A2, Site::B2]),
            p(BuildingType::GrandArchive, &[Site::A3, Site::B3]),
        ],
        Route::RiverCompact => vec![
            p(BuildingType::Embassy, &[Site::A1, Site::B1]),
            p(BuildingType::Embassy, &[Site::A2, Site::B2]),
            p(BuildingType::GrandArchive, &[Site::A3, Site::B3]),
        ],
        Route::BeaconMesh => vec![
            p(BuildingType::SurveyLodge, &[Site::A1]),
            p(BuildingType::SurveyLodge, &[Site::A2]),
            p(BuildingType::Observatory, &[Site::A3]),
            p(BuildingType::Observatory, &[Site::B1]),
            p(BuildingType::CaravanHouse, &[Site::B2]),
            p(BuildingType::CaravanHouse, &[Site::B3]),
        ],
    }
}

#[test]
fn optional_initialization_is_zero_spend_and_persists_identity() {
    let mut f = Fixture::new();
    let before = f.ws();
    assert_eq!(before.progress.wallet_bc, 0);
    let c = before.world.chapter.as_ref().unwrap();
    assert_eq!(c.revision, 1);
    assert_eq!(c.funded_site_count, 0);
    assert!(c.layout.is_empty());
    assert_eq!(f.view().buildings.len(), 6);
    assert_eq!(f.view().parcels.len(), 6);
    assert_eq!(f.view().routes.len(), 3);
    f.reopen();
    assert_eq!(f.ws(), before);
    f.reject(WorldOperation::Initialize);
}

#[test]
fn reserve_partial_cancel_repeat_and_replace_conserve_bank() {
    let mut f = Fixture::new();
    f.earn(4);
    let issuance = f.ws().progress.clone();
    f.run(WorldOperation::Queue {
        building_type: BuildingType::Foundry,
        parcels: vec![Site::A1, Site::A2],
        reserve_available: true,
    });
    let canceled_queue_command = f.ws().world.commands.last().unwrap().payload.clone();
    assert_eq!(
        f.view().bank,
        BankProjection {
            issued_bc: 4,
            spent_bc: 0,
            reserved_bc: 4,
            available_bc: 0
        }
    );
    assert_eq!(f.view().capabilities, Capabilities::default());
    assert_eq!(f.view().chapter.unwrap().funded_site_count, 0);
    f.reject(WorldOperation::CompleteConstruction);
    f.run(WorldOperation::ReplaceQueue {
        building_type: BuildingType::SurveyLodge,
        parcels: vec![Site::B3],
        reserve_available: true,
    });
    assert_eq!(f.view().bank.reserved_bc, 4);
    let cancel = f.input(WorldOperation::CancelQueue);
    let original = f.service.world_command(cancel.clone()).unwrap().result;
    assert_eq!(f.view().bank.available_bc, 4);
    assert_eq!(f.service.world_command(cancel).unwrap().result, original);
    f.reject(WorldOperation::CancelQueue);
    f.service.world_command(canceled_queue_command).unwrap();
    assert!(f.view().chapter.unwrap().queue.is_none());
    assert_eq!(f.view().bank.available_bc, 4);
    assert_eq!(f.ws().progress, issuance);
}

#[test]
fn invalid_replacement_rolls_back_original_reservation_and_queue() {
    let mut f = Fixture::new();
    f.earn(4);
    f.run(WorldOperation::Queue {
        building_type: BuildingType::Foundry,
        parcels: vec![Site::A1, Site::A2],
        reserve_available: true,
    });
    f.reject(WorldOperation::ReplaceQueue {
        building_type: BuildingType::GrandArchive,
        parcels: vec![Site::A1, Site::B3],
        reserve_available: true,
    });
    assert_eq!(f.view().bank.reserved_bc, 4);
}

#[test]
fn partial_reservation_reopens_restores_and_completes_with_later_issuance() {
    let mut f = Fixture::new();
    f.earn(4);
    f.run(WorldOperation::Queue {
        building_type: BuildingType::Embassy,
        parcels: vec![Site::A1, Site::B1],
        reserve_available: true,
    });
    let bytes = f.service.export_bytes().unwrap();
    f.reopen();
    assert_eq!(f.view().bank.reserved_bc, 4);
    let mut restored = Fixture::new();
    restored.restore(&bytes);
    assert_eq!(restored.service.export_bytes().unwrap(), bytes);
    restored.earn(4);
    restored.run(WorldOperation::Reserve);
    assert_eq!(restored.view().bank.reserved_bc, 8);
    restored.run(WorldOperation::CompleteConstruction);
    assert_eq!(
        restored.view().bank,
        BankProjection {
            issued_bc: 8,
            spent_bc: 8,
            reserved_bc: 0,
            available_bc: 0
        }
    );
    assert_eq!(restored.view().capabilities.diplomacy, 5);
    assert_eq!(restored.view().chapter.unwrap().funded_site_count, 2);
}

#[test]
fn non_multiple_reservations_are_honest_and_completion_uses_available_credit() {
    let mut f = Fixture::new();
    f.earn(3);
    f.run(WorldOperation::Queue {
        building_type: BuildingType::Observatory,
        parcels: vec![Site::A2],
        reserve_available: true,
    });
    assert_eq!(f.view().bank.reserved_bc, 3);
    f.reject(WorldOperation::CompleteConstruction);
    f.earn(1);
    f.run(WorldOperation::CompleteConstruction);
    assert_eq!(f.view().bank.spent_bc, 4);
    assert_eq!(
        f.ws().world.spending[0]
            .allocations
            .iter()
            .map(|a| a.amount_bc)
            .collect::<Vec<_>>(),
        vec![3, 1]
    );
}

#[test]
fn exact_replay_survives_restart_restore_and_payload_changes_conflict() {
    let mut f = Fixture::new();
    f.earn(4);
    f.run(WorldOperation::Queue {
        building_type: BuildingType::SurveyLodge,
        parcels: vec![Site::A1],
        reserve_available: true,
    });
    let complete = f.input(WorldOperation::CompleteConstruction);
    let first = f.service.world_command(complete.clone()).unwrap().result;
    let bytes = f.service.export_bytes().unwrap();
    f.reopen();
    assert_eq!(
        f.service.world_command(complete.clone()).unwrap().result,
        first
    );
    assert_eq!(f.service.export_bytes().unwrap(), bytes);
    let mut target = Fixture::new();
    target.restore(&bytes);
    assert_eq!(
        target
            .service
            .world_command(complete.clone())
            .unwrap()
            .result,
        first
    );
    let mut changed = complete;
    changed.operation = WorldOperation::CancelQueue;
    assert_eq!(
        target.service.world_command(changed).unwrap_err().code,
        "idempotency_conflict"
    );
    assert_eq!(target.view().bank.spent_bc, 4);
    assert_eq!(target.ws().world.spending.len(), 1);
}

#[test]
fn concurrent_exact_completion_constructs_once_and_returns_original_result() {
    let mut f = Fixture::new();
    f.earn(4);
    f.run(WorldOperation::Queue {
        building_type: BuildingType::SurveyLodge,
        parcels: vec![Site::A1],
        reserve_available: true,
    });
    let input = f.input(WorldOperation::CompleteConstruction);
    let barrier = Arc::new(Barrier::new(2));
    let threads: Vec<_> = (0..2)
        .map(|_| {
            let database = f.database.clone();
            let backups = f.backups.clone();
            let input = input.clone();
            let barrier = barrier.clone();
            std::thread::spawn(move || {
                let mut service = WorkspaceService::open(&database, &backups).unwrap();
                barrier.wait();
                service.world_command(input).map(|r| r.result)
            })
        })
        .collect();
    let results: Vec<_> = threads
        .into_iter()
        .map(|t| t.join().unwrap().unwrap())
        .collect();
    assert_eq!(results[0], results[1]);
    assert_eq!(f.ws().world.spending.len(), 1);
    assert_eq!(f.view().bank.spent_bc, 4);
}

#[test]
fn concurrent_different_reservations_cannot_reserve_same_credit_twice() {
    let mut f = Fixture::new();
    f.earn(4);
    f.run(WorldOperation::Queue {
        building_type: BuildingType::Foundry,
        parcels: vec![Site::A1, Site::B1],
        reserve_available: false,
    });
    let barrier = Arc::new(Barrier::new(2));
    let input = f.input(WorldOperation::Reserve);
    let threads: Vec<_> = (0..2)
        .map(|_| {
            let database = f.database.clone();
            let backups = f.backups.clone();
            let mut input = input.clone();
            input.command_id = new_id();
            let barrier = barrier.clone();
            std::thread::spawn(move || {
                let mut service = WorkspaceService::open(&database, &backups).unwrap();
                barrier.wait();
                service.world_command(input).map(|r| r.result)
            })
        })
        .collect();
    let results: Vec<_> = threads.into_iter().map(|t| t.join().unwrap()).collect();
    assert_eq!(results.iter().filter(|r| r.is_ok()).count(), 1);
    assert_eq!(
        results.iter().find_map(|r| r.as_ref().err()).unwrap().code,
        "conflict"
    );
    assert_eq!(f.view().bank.reserved_bc, 4);
    assert_eq!(f.view().bank.available_bc, 0);
}

#[test]
fn stale_world_decision_rejects_but_work_changes_do_not_change_world_revision() {
    let mut f = Fixture::new();
    let stale = f.input(WorldOperation::SelectRoute {
        route: Route::BeaconMesh,
    });
    f.run(WorldOperation::SelectRoute {
        route: Route::StoneCauseway,
    });
    assert_eq!(f.service.world_command(stale).unwrap_err().code, "conflict");
    let decision = f.input(WorldOperation::SelectRoute {
        route: Route::RiverCompact,
    });
    f.earn(4);
    f.service.world_command(decision).unwrap();
    assert_eq!(
        f.view().chapter.unwrap().selected_route,
        Some(Route::RiverCompact)
    );
}

#[test]
fn transactional_faults_at_spend_layout_command_and_generation_leave_no_partial_state() {
    for (name, sql) in [
        (
            "fail_spend",
            "CREATE TRIGGER fail_spend BEFORE INSERT ON world_spending BEGIN SELECT RAISE(ABORT,'injected'); END;",
        ),
        (
            "fail_layout",
            "CREATE TRIGGER fail_layout BEFORE UPDATE ON world_chapter BEGIN SELECT RAISE(ABORT,'injected'); END;",
        ),
        (
            "fail_command",
            "CREATE TRIGGER fail_command BEFORE INSERT ON world_commands BEGIN SELECT RAISE(ABORT,'injected'); END;",
        ),
        (
            "fail_generation",
            "CREATE TRIGGER fail_generation BEFORE UPDATE ON workspace_state BEGIN SELECT RAISE(ABORT,'injected'); END;",
        ),
    ] {
        let mut f = Fixture::new();
        f.earn(4);
        f.run(WorldOperation::Queue {
            building_type: BuildingType::SurveyLodge,
            parcels: vec![Site::A1],
            reserve_available: true,
        });
        let before = f.service.export_bytes().unwrap();
        let input = f.input(WorldOperation::CompleteConstruction);
        let c = Connection::open(&f.database).unwrap();
        c.execute_batch(sql).unwrap();
        assert_eq!(
            f.service.world_command(input.clone()).unwrap_err().code,
            "storage_error"
        );
        assert_eq!(f.service.export_bytes().unwrap(), before);
        c.execute_batch(&format!("DROP TRIGGER {name}")).unwrap();
        f.reopen();
        assert_eq!(f.service.export_bytes().unwrap(), before);
        f.service.world_command(input).unwrap();
        assert_eq!(f.view().bank.spent_bc, 4);
    }
}

#[test]
fn dropped_uncommitted_transaction_recovers_queue_and_reservation() {
    let mut f = Fixture::new();
    f.earn(4);
    f.run(WorldOperation::Queue {
        building_type: BuildingType::SurveyLodge,
        parcels: vec![Site::B1],
        reserve_available: true,
    });
    let before = f.service.export_bytes().unwrap();
    let input = f.input(WorldOperation::CompleteConstruction);
    {
        let mut c = Connection::open(&f.database).unwrap();
        c.pragma_update(None, "foreign_keys", true).unwrap();
        let transaction = c
            .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)
            .unwrap();
        execute(&transaction, input.clone()).unwrap();
        assert_eq!(super::read(&transaction).unwrap().spending.len(), 1);
    }
    f.reopen();
    assert_eq!(f.service.export_bytes().unwrap(), before);
    f.service.world_command(input).unwrap();
    assert_eq!(f.view().bank.spent_bc, 4);
}

#[test]
fn funded_capacity_is_reusable_move_replace_remove_and_undo_never_refund() {
    let mut f = Fixture::new();
    f.earn(4);
    f.earn(4);
    f.fund(BuildingType::Foundry, &[Site::A1, Site::A2]);
    let issuance = f.ws().progress.clone();
    let original = f.view().chapter.unwrap().layout;
    f.run(WorldOperation::Reconfigure {
        placements: vec![
            p(BuildingType::SurveyLodge, &[Site::B1]),
            p(BuildingType::Observatory, &[Site::B2]),
        ],
    });
    assert_eq!(
        f.view().capabilities,
        Capabilities {
            engineering: 2,
            knowledge: 3,
            diplomacy: 1
        }
    );
    f.reopen();
    f.run(WorldOperation::Undo);
    assert_eq!(f.view().chapter.unwrap().layout, original);
    f.reject(WorldOperation::Undo);
    let building = &original[0];
    f.run(WorldOperation::Reconfigure {
        placements: vec![PlacementInput {
            id: Some(building.id.clone()),
            building_type: building.building_type,
            parcels: vec![Site::A3, Site::B3],
        }],
    });
    assert_eq!(f.view().chapter.unwrap().layout[0].id, building.id);
    f.run(WorldOperation::Reconfigure { placements: vec![] });
    assert_eq!(f.view().bank.available_bc, 0);
    assert_eq!(f.view().chapter.unwrap().funded_site_count, 2);
    f.fund(BuildingType::Embassy, &[Site::A2, Site::B2]);
    assert_eq!(f.view().bank.spent_bc, 8);
    assert_eq!(f.ws().world.spending.len(), 1);
    assert_eq!(f.ws().progress, issuance);
}

#[test]
fn three_one_parcel_buildings_cannot_replace_two_funded_sites() {
    let mut f = Fixture::new();
    f.earn(4);
    f.earn(4);
    f.fund(BuildingType::Foundry, &[Site::A1, Site::A2]);
    f.reject(WorldOperation::Reconfigure {
        placements: vec![
            p(BuildingType::SurveyLodge, &[Site::A1]),
            p(BuildingType::Observatory, &[Site::A2]),
            p(BuildingType::CaravanHouse, &[Site::A3]),
        ],
    });
    assert_eq!(f.view().bank.spent_bc, 8);
}

#[test]
fn unused_capacity_only_charges_deficit_and_seventh_site_is_impossible() {
    let mut f = Fixture::new();
    for _ in 0..3 {
        f.earn(4);
    }
    f.fund(BuildingType::Foundry, &[Site::A1, Site::A2]);
    f.run(WorldOperation::Reconfigure {
        placements: vec![p(BuildingType::SurveyLodge, &[Site::A1])],
    });
    f.fund(BuildingType::GrandArchive, &[Site::B2, Site::B3]);
    assert_eq!(f.view().bank.spent_bc, 12);
    assert_eq!(f.ws().world.spending[1].amount_bc, 4);
    assert_eq!(f.view().chapter.unwrap().funded_site_count, 3);
    let mut full = Fixture::new();
    full.full();
    full.earn(4);
    full.reject(WorldOperation::Queue {
        building_type: BuildingType::CaravanHouse,
        parcels: vec![Site::A1],
        reserve_available: true,
    });
    assert_eq!(full.view().bank.spent_bc, 24);
    assert_eq!(full.view().bank.available_bc, 4);
}

#[test]
fn reconfiguration_and_undo_cannot_invalidate_active_blueprint() {
    let mut f = Fixture::new();
    f.earn(4);
    f.fund(BuildingType::SurveyLodge, &[Site::A1]);
    f.run(WorldOperation::Reconfigure {
        placements: vec![p(BuildingType::Observatory, &[Site::B1])],
    });
    f.run(WorldOperation::Queue {
        building_type: BuildingType::Foundry,
        parcels: vec![Site::A2, Site::B2],
        reserve_available: false,
    });
    f.reject(WorldOperation::Undo);
    f.reject(WorldOperation::Reconfigure { placements: vec![] });
}

#[test]
fn all_routes_complete_legally_and_original_chronicle_survives_sandbox_restore() {
    for route in [Route::StoneCauseway, Route::RiverCompact, Route::BeaconMesh] {
        let mut f = Fixture::new();
        f.full();
        let authority = f.ws().progress.clone();
        f.run(WorldOperation::Reconfigure {
            placements: route_layout(route),
        });
        f.run(WorldOperation::SelectRoute { route });
        let input = f.input(WorldOperation::CompleteChapter);
        f.service.world_command(input.clone()).unwrap();
        let ending = f.view().chapter.unwrap().completion.unwrap();
        assert_eq!(ending.route, route);
        assert_eq!(ending.spent_bc, 24);
        assert!(satisfies(&ending.capabilities, route));
        f.run(WorldOperation::Reconfigure {
            placements: route_layout(if route == Route::BeaconMesh {
                Route::StoneCauseway
            } else {
                Route::BeaconMesh
            }),
        });
        assert_eq!(f.view().chapter.unwrap().completion, Some(ending.clone()));
        assert_eq!(f.view().chapter.unwrap().sandbox_alternatives, 1);
        f.run(WorldOperation::Undo);
        f.service.world_command(input).unwrap();
        f.reject(WorldOperation::CompleteChapter);
        f.reject(WorldOperation::SelectRoute {
            route: Route::BeaconMesh,
        });
        f.reject(WorldOperation::Queue {
            building_type: BuildingType::Foundry,
            parcels: vec![Site::A1, Site::B1],
            reserve_available: true,
        });
        assert_eq!(f.ws().progress, authority);
        let bytes = f.service.export_bytes().unwrap();
        let mut target = Fixture::new();
        target.restore(&bytes);
        target.reopen();
        assert_eq!(target.service.export_bytes().unwrap(), bytes);
        assert_eq!(target.view().chapter.unwrap().completion, Some(ending));
    }
}

#[test]
fn route_preview_or_missing_capability_never_commits_chapter() {
    let mut f = Fixture::new();
    f.full();
    f.run(WorldOperation::SelectRoute {
        route: Route::BeaconMesh,
    });
    f.reject(WorldOperation::CompleteChapter);
    assert!(f.view().chapter.unwrap().completion.is_none());
    let preview = f
        .service
        .preview_world(WorldPreviewInput {
            chapter_id: f.view().chapter.unwrap().id,
            content_version: CONTENT_VERSION.into(),
            placements: route_layout(Route::BeaconMesh),
        })
        .unwrap();
    assert!(preview.legal);
    assert!(
        preview
            .routes
            .iter()
            .find(|r| r.route == Route::BeaconMesh)
            .unwrap()
            .satisfied
    );
    assert!(f.view().chapter.unwrap().completion.is_none());
}

#[test]
fn exhaustive_729_numerical_combinations_match_frozen_balance_counts() {
    let definitions = building_definitions();
    let mut eligible = 0;
    let mut counts = [0, 0, 0];
    for code in 0..729 {
        let mut remainder = code;
        let mut footprint = 0;
        let mut c = Capabilities::default();
        for b in &definitions {
            let copies = remainder % 3;
            remainder /= 3;
            footprint += copies * b.footprint;
            c.engineering += copies * b.capabilities.engineering;
            c.knowledge += copies * b.capabilities.knowledge;
            c.diplomacy += copies * b.capabilities.diplomacy;
        }
        if footprint > 6 {
            continue;
        }
        eligible += 1;
        for (index, route) in [Route::StoneCauseway, Route::RiverCompact, Route::BeaconMesh]
            .into_iter()
            .enumerate()
        {
            if satisfies(&c, route) {
                counts[index] += 1;
                assert_eq!(footprint, 6);
            }
        }
    }
    assert_eq!(counts, [3, 3, 1]);
    assert!(eligible > 7);
    println!(
        "River Reach: 729 combinations; {eligible} within six sites; route counts {counts:?}. Spatial tests are separate."
    );
}

#[test]
fn every_specialist_pair_checks_orthogonal_adjacency_and_no_diagonals() {
    let sites = parcels();
    let mut legal = 0;
    for a in 0..6 {
        for b in a + 1..6 {
            let layout = vec![BuildingPlacement {
                id: new_id(),
                building_type: BuildingType::Foundry,
                parcels: vec![sites[a].id, sites[b].id],
            }];
            let expected = sites[a].row.abs_diff(sites[b].row)
                + sites[a].column.abs_diff(sites[b].column)
                == 1;
            assert_eq!(validate_layout(&layout, 2).is_ok(), expected);
            legal += usize::from(expected);
        }
    }
    assert_eq!(legal, 7);
}

#[test]
fn overlap_copy_limits_wrong_footprint_and_foreign_identity_reject() {
    let mut f = Fixture::new();
    f.full();
    f.reject(WorldOperation::Reconfigure {
        placements: vec![
            p(BuildingType::SurveyLodge, &[Site::A1]),
            p(BuildingType::Observatory, &[Site::A1]),
        ],
    });
    f.reject(WorldOperation::Reconfigure {
        placements: vec![
            p(BuildingType::SurveyLodge, &[Site::A1]),
            p(BuildingType::SurveyLodge, &[Site::A2]),
            p(BuildingType::SurveyLodge, &[Site::A3]),
        ],
    });
    f.reject(WorldOperation::Reconfigure {
        placements: vec![p(BuildingType::Foundry, &[Site::A1])],
    });
    f.reject(WorldOperation::Reconfigure {
        placements: vec![PlacementInput {
            id: Some(new_id()),
            building_type: BuildingType::Foundry,
            parcels: vec![Site::A1, Site::B1],
        }],
    });
    let id = f.view().chapter.unwrap().layout[0].id.clone();
    f.reject(WorldOperation::Reconfigure {
        placements: vec![PlacementInput {
            id: Some(id),
            building_type: BuildingType::CaravanHouse,
            parcels: vec![Site::A1],
        }],
    });
}

#[test]
fn preview_distinguishes_legal_extra_funding_and_invalid_geometry_route_numbers() {
    let mut f = Fixture::new();
    let c = f.view().chapter.unwrap();
    let preview = f
        .service
        .preview_world(WorldPreviewInput {
            chapter_id: c.id.clone(),
            content_version: CONTENT_VERSION.into(),
            placements: vec![p(BuildingType::Foundry, &[Site::A1, Site::A2])],
        })
        .unwrap();
    assert!(preview.placement_legal);
    assert!(!preview.legal);
    assert_eq!(preview.additional_sites_needed, 2);
    let invalid = vec![
        p(BuildingType::Foundry, &[Site::A1, Site::B2]),
        p(BuildingType::Foundry, &[Site::A2, Site::B3]),
        p(BuildingType::GrandArchive, &[Site::A3, Site::B1]),
    ];
    let preview = f
        .service
        .preview_world(WorldPreviewInput {
            chapter_id: c.id,
            content_version: CONTENT_VERSION.into(),
            placements: invalid.clone(),
        })
        .unwrap();
    assert_eq!(preview.capabilities.engineering, 10);
    assert!(!preview.placement_legal);
    assert!(preview.routes.iter().all(|r| !r.satisfied));
    f.reject(WorldOperation::Reconfigure {
        placements: invalid,
    });
}

#[test]
fn fifo_provenance_and_factual_correction_retain_capacity_and_issuance() {
    let mut f = Fixture::new();
    let first = f.earn(4);
    f.earn(4);
    f.fund(BuildingType::Foundry, &[Site::A1, Site::B1]);
    let before = f.ws();
    let mut ledger = before.progress.ledger.clone();
    ledger.sort_by_key(|l| l.sequence);
    assert_eq!(
        before.world.spending[0]
            .allocations
            .iter()
            .map(|a| &a.issuance_entry_id)
            .collect::<Vec<_>>(),
        ledger.iter().map(|l| &l.id).collect::<Vec<_>>()
    );
    f.service
        .correct_acceptance(CorrectAcceptanceInput {
            expected_generation: before.generation,
            acceptance_id: first,
            kind: CorrectionKind::IncorrectAcceptance,
            reason: "Synthetic factual correction; fictional capacity remains.".into(),
        })
        .unwrap();
    assert_eq!(f.ws().world, before.world);
    assert_eq!(f.ws().progress.wallet_bc, 8);
    assert_eq!(f.view().retained_credit_bc, 4);
    assert_eq!(f.view().bank.available_bc, 0);
    let bytes = f.service.export_bytes().unwrap();
    let mut target = Fixture::new();
    target.restore(&bytes);
    assert_eq!(target.view().retained_credit_bc, 4);
    assert_eq!(target.ws().world, f.ws().world);
}

#[test]
fn every_active_study_phase_withholds_h2_without_changing_h1_or_protocol_history() {
    for phase in [Phase::E00, Phase::E01] {
        let mut f = Fixture::new();
        let init = f.ws().world.commands[0].payload.clone();
        let w = f.ws();
        let w = f
            .service
            .create_schedule_epoch(CreateScheduleEpochInput {
                expected_generation: w.generation,
                label: "Synthetic isolated epoch".into(),
                effective_from: "2026-01-01T00:00:00.000Z".into(),
                timezone: "UTC".into(),
            })
            .unwrap();
        let w = f
            .service
            .configure_study(ConfigureStudyInput {
                expected_generation: w.generation,
                phase,
                epoch_id: w.study.epochs[0].id.clone(),
            })
            .unwrap();
        let manifest = w.study.manifests[0].id.clone();
        if phase == Phase::E01 {
            f.service
                .transition_study(TransitionStudyInput {
                    expected_generation: w.generation,
                    manifest_id: manifest.clone(),
                    status: StudyStatus::Ready,
                    reason: "Synthetic feasibility decision only".into(),
                    acknowledgements: vec![],
                })
                .unwrap();
        }
        f.service
            .transition_study(TransitionStudyInput {
                expected_generation: f.ws().generation,
                manifest_id: manifest.clone(),
                status: StudyStatus::Active,
                reason: "Synthetic owner feasibility decision in isolated test".into(),
                acknowledgements: [
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
                .collect(),
            })
            .unwrap();
        let before = f.ws();
        assert_eq!(f.service.load_world().unwrap_err().code, "study_active");
        let input = f.input(WorldOperation::SelectRoute {
            route: Route::BeaconMesh,
        });
        assert_eq!(
            f.service.world_command(input).unwrap_err().code,
            "study_active"
        );
        assert_eq!(
            f.service
                .preview_world(WorldPreviewInput {
                    chapter_id: before.world.chapter.as_ref().unwrap().id.clone(),
                    content_version: CONTENT_VERSION.into(),
                    placements: vec![]
                })
                .unwrap_err()
                .code,
            "study_active"
        );
        assert_eq!(f.ws(), before);
        f.service.world_command(init).unwrap();
        assert_eq!(f.ws(), before);
        f.service
            .transition_study(TransitionStudyInput {
                expected_generation: before.generation,
                manifest_id: manifest,
                status: StudyStatus::Paused,
                reason: "Explicit synthetic pause before optional gameplay".into(),
                acknowledgements: vec![],
            })
            .unwrap();
        f.run(WorldOperation::SelectRoute {
            route: Route::BeaconMesh,
        });
        assert_eq!(f.ws().progress, before.progress);
        assert_eq!(f.ws().study.assignments, before.study.assignments);
    }
}

#[test]
fn legacy_formats_cannot_erase_world_history_and_current_restore_replays_exactly() {
    let mut f = Fixture::new();
    f.earn(4);
    f.fund(BuildingType::SurveyLodge, &[Site::A1]);
    let bytes = f.service.export_bytes().unwrap();
    let mut old: Value = serde_json::from_slice(&bytes).unwrap();
    old["schema_version"] = json!(3);
    old.as_object_mut().unwrap().remove("world");
    old.as_object_mut().unwrap().remove("daily");
    assert_eq!(
        f.service
            .preview_import(
                &serde_json::to_vec(&old).unwrap(),
                "synthetic-old.json".into()
            )
            .unwrap_err()
            .code,
        "unsafe_legacy_import"
    );
    assert_eq!(f.service.export_bytes().unwrap(), bytes);
    f.restore(&bytes);
    assert_eq!(f.service.export_bytes().unwrap(), bytes);
}

#[test]
fn forged_economics_layout_command_undo_and_chronicle_imports_fail_without_state_loss() {
    let mut f = Fixture::new();
    f.full();
    f.run(WorldOperation::SelectRoute {
        route: Route::StoneCauseway,
    });
    f.run(WorldOperation::CompleteChapter);
    let bytes = f.service.export_bytes().unwrap();
    let source: Value = serde_json::from_slice(&bytes).unwrap();
    type Forgery = Box<dyn Fn(&mut Value)>;
    let mutations: Vec<Forgery> = vec![
        Box::new(|v| v["world"]["chapter"]["funded_site_count"] = json!(7)),
        Box::new(|v| v["world"]["chapter"]["revision"] = json!(999)),
        Box::new(|v| v["world"]["chapter"]["content_version"] = json!("cheaper-v2")),
        Box::new(|v| v["world"]["spending"][0]["amount_bc"] = json!(4)),
        Box::new(|v| v["world"]["spending"][0]["allocations"][0]["amount_bc"] = json!(8)),
        Box::new(|v| v["world"]["spending"][0]["ledger_sequence"] = json!(0)),
        Box::new(|v| v["world"]["chapter"]["layout"][0]["parcels"] = json!(["a1", "b3"])),
        Box::new(|v| v["world"]["chapter"]["layout"][0]["parcels"] = json!(["a7", "b3"])),
        Box::new(|v| v["world"]["commands"][1]["payload_digest"] = json!("0000")),
        Box::new(|v| v["world"]["commands"][1]["result"]["available_bc"] = json!(100)),
        Box::new(|v| v["world"]["commands"][1]["ledger_sequence"] = json!(0)),
        Box::new(|v| {
            v["world"]["chapter"]["previous_layout"] = v["world"]["chapter"]["layout"].clone()
        }),
        Box::new(|v| v["world"]["chapter"]["completion"]["route"] = json!("beacon_mesh")),
        Box::new(|v| {
            v["world"]["chapter"]["completion"]["capabilities"]["engineering"] = json!(100)
        }),
        Box::new(|v| v["world"]["chapter"]["completion"]["narrative"] = json!("Forged ending")),
        Box::new(|v| v["world"]["chapter"]["completion"] = Value::Null),
        Box::new(|v| v["progress"]["wallet_bc"] = json!(999)),
        Box::new(|v| {
            v["world"]["commands"].as_array_mut().unwrap().remove(1);
        }),
        Box::new(|v| {
            let row = v["world"]["spending"][0].clone();
            v["world"]["spending"].as_array_mut().unwrap().push(row);
        }),
    ];
    for mutate in mutations {
        let mut bad = source.clone();
        mutate(&mut bad);
        assert!(
            f.service
                .preview_import(
                    &serde_json::to_vec(&bad).unwrap(),
                    "forged-synthetic.json".into()
                )
                .is_err()
        );
        assert_eq!(f.service.export_bytes().unwrap(), bytes);
    }
}

#[test]
fn recomputed_hash_cannot_make_impossible_historical_command_valid() {
    let mut f = Fixture::new();
    f.earn(4);
    f.fund(BuildingType::SurveyLodge, &[Site::A1]);
    let bytes = f.service.export_bytes().unwrap();
    let mut records: RecordSet = serde_json::from_slice(&bytes).unwrap();
    let command = &mut records.world.as_mut().unwrap().commands[1];
    command.payload.operation = WorldOperation::Undo;
    command.payload_digest = hash(&command.payload).unwrap();
    assert!(
        f.service
            .preview_import(
                &serde_json::to_vec(&records).unwrap(),
                "synthetic-forged.json".into()
            )
            .is_err()
    );
    assert_eq!(f.service.export_bytes().unwrap(), bytes);
}

#[test]
fn sqlite_independent_guards_reject_budget_capacity_and_ending_corruption() {
    let mut f = Fixture::new();
    f.full();
    f.run(WorldOperation::SelectRoute {
        route: Route::StoneCauseway,
    });
    f.run(WorldOperation::CompleteChapter);
    let c = Connection::open(&f.database).unwrap();
    c.pragma_update(None, "foreign_keys", true).unwrap();
    assert!(
        c.execute(
            "UPDATE world_chapter SET payload=json_set(payload,'$.funded_site_count',7)",
            []
        )
        .is_err()
    );
    assert!(
        c.execute(
            "UPDATE world_chapter SET payload=json_set(payload,'$.completion',NULL)",
            []
        )
        .is_err()
    );
    assert!(
        c.execute("UPDATE world_spending SET payload=payload", [])
            .is_err()
    );
    assert!(
        c.execute("UPDATE world_commands SET payload=payload", [])
            .is_err()
    );
    let mut spend = f.ws().world.spending[0].clone();
    spend.id = new_id();
    spend.command_id = new_id();
    spend.sequence = 4;
    assert!(insert(&c, "world_spending", &spend.id, &spend).is_err());
    assert_eq!(f.view().bank.spent_bc, 24);
}

#[test]
fn strict_world_dtos_reject_privileges_prices_invalid_enums_and_missing_nullable() {
    let f = Fixture::new();
    let valid = serde_json::to_value(f.input(WorldOperation::Queue {
        building_type: BuildingType::Foundry,
        parcels: vec![Site::A1, Site::A2],
        reserve_available: true,
    }))
    .unwrap();
    for (field, value) in [
        ("cost_bc", json!(0)),
        ("actor", json!("validator")),
        ("expected_world_revision", json!(-1)),
        ("schema_version", json!(1.5)),
    ] {
        let mut bad = valid.clone();
        bad[field] = value;
        assert!(serde_json::from_value::<WorldCommandInput>(bad).is_err());
    }
    let mut bad = valid.clone();
    bad.as_object_mut().unwrap().remove("chapter_id");
    assert!(serde_json::from_value::<WorldCommandInput>(bad).is_err());
    let mut bad = valid;
    bad["operation"]["building_type"] = json!("super_foundry");
    assert!(serde_json::from_value::<WorldCommandInput>(bad).is_err());
    assert!(
        serde_json::from_value::<PlacementInput>(
            json!({"building_type":"foundry","parcels":["a1","a2"]})
        )
        .is_err()
    );
}

#[test]
fn restored_historical_full_snapshot_cannot_repeat_existing_spend_identity() {
    let mut f = Fixture::new();
    f.earn(4);
    f.run(WorldOperation::Queue {
        building_type: BuildingType::SurveyLodge,
        parcels: vec![Site::A1],
        reserve_available: true,
    });
    let command = f.input(WorldOperation::CompleteConstruction);
    f.service.world_command(command.clone()).unwrap();
    let old = f.service.export_bytes().unwrap();
    f.earn(4);
    let mut target = Fixture::new();
    target.restore(&old);
    target.service.world_command(command).unwrap();
    assert_eq!(target.ws().world.spending.len(), 1);
    assert_eq!(target.view().bank.available_bc, 0);
}

#[test]
fn zero_issuance_cannot_reserve_construct_or_create_free_capacity() {
    let mut f = Fixture::new();
    f.run(WorldOperation::Queue {
        building_type: BuildingType::SurveyLodge,
        parcels: vec![Site::A1],
        reserve_available: true,
    });
    assert_eq!(f.view().bank.reserved_bc, 0);
    f.reject(WorldOperation::Reserve);
    f.reject(WorldOperation::CompleteConstruction);
    f.run(WorldOperation::CancelQueue);
    f.reject(WorldOperation::Reconfigure {
        placements: vec![p(BuildingType::SurveyLodge, &[Site::A1])],
    });
    assert_eq!(f.view().bank.spent_bc, 0);
    assert_eq!(f.view().chapter.unwrap().funded_site_count, 0);
}

#[test]
fn concurrent_distinct_completions_cannot_commit_the_same_blueprint_twice() {
    let mut f = Fixture::new();
    f.earn(4);
    f.run(WorldOperation::Queue {
        building_type: BuildingType::Observatory,
        parcels: vec![Site::B3],
        reserve_available: true,
    });
    let input = f.input(WorldOperation::CompleteConstruction);
    let barrier = Arc::new(Barrier::new(2));
    let threads: Vec<_> = (0..2)
        .map(|_| {
            let database = f.database.clone();
            let backups = f.backups.clone();
            let mut input = input.clone();
            input.command_id = new_id();
            let barrier = barrier.clone();
            std::thread::spawn(move || {
                let mut service = WorkspaceService::open(&database, &backups).unwrap();
                barrier.wait();
                service.world_command(input).map(|r| r.result)
            })
        })
        .collect();
    let results: Vec<_> = threads.into_iter().map(|t| t.join().unwrap()).collect();
    assert_eq!(results.iter().filter(|r| r.is_ok()).count(), 1);
    assert_eq!(
        results.iter().find_map(|r| r.as_ref().err()).unwrap().code,
        "conflict"
    );
    assert_eq!(f.ws().world.spending.len(), 1);
    assert_eq!(f.view().chapter.unwrap().layout.len(), 1);
    assert_eq!(f.view().bank.spent_bc, 4);
}

#[test]
fn world_lock_contention_is_bounded_and_preserves_every_authority_record() {
    let mut f = Fixture::new();
    f.earn(4);
    f.run(WorldOperation::Queue {
        building_type: BuildingType::SurveyLodge,
        parcels: vec![Site::A1],
        reserve_available: true,
    });
    let input = f.input(WorldOperation::CompleteConstruction);
    let before = f.ws();
    let c = Connection::open(&f.database).unwrap();
    c.execute_batch("BEGIN IMMEDIATE").unwrap();
    let started = std::time::Instant::now();
    assert_eq!(f.service.world_command(input).unwrap_err().code, "busy");
    assert!(started.elapsed() < std::time::Duration::from_secs(8));
    c.execute_batch("ROLLBACK").unwrap();
    assert_eq!(f.ws(), before);
}

#[test]
fn failed_restore_backup_and_world_insertion_preserve_displaced_world_and_work() {
    let mut source = Fixture::new();
    source.earn(4);
    source.fund(BuildingType::Observatory, &[Site::B3]);
    let incoming = source.service.export_bytes().unwrap();
    let mut target = Fixture::new();
    target.earn(4);
    target.run(WorldOperation::Queue {
        building_type: BuildingType::Foundry,
        parcels: vec![Site::A1, Site::B1],
        reserve_available: true,
    });
    let before = target.service.export_bytes().unwrap();
    drop(target.service);
    if target.backups.exists() {
        std::fs::rename(
            &target.backups,
            target._directory.path().join("prior-backups"),
        )
        .unwrap();
    }
    target.service = WorkspaceService::open(&target.database, &target.backups).unwrap();
    std::fs::write(&target.backups, b"synthetic backup path blocker").unwrap();
    let preview = target
        .service
        .preview_import(&incoming, "synthetic.json".into())
        .unwrap();
    assert_eq!(
        target
            .service
            .restore_import(RestoreInput {
                token: preview.token,
                expected_generation: preview.generation
            })
            .unwrap_err()
            .code,
        "backup_failed"
    );
    assert_eq!(target.service.export_bytes().unwrap(), before);
    std::fs::remove_file(&target.backups).unwrap();
    for (name, sql) in [
        (
            "restore_world_command_failure",
            "CREATE TRIGGER restore_world_command_failure BEFORE INSERT ON world_commands BEGIN SELECT RAISE(ABORT,'injected'); END;",
        ),
        (
            "restore_generation_failure",
            "CREATE TRIGGER restore_generation_failure BEFORE UPDATE ON workspace_state BEGIN SELECT RAISE(ABORT,'injected'); END;",
        ),
    ] {
        let c = Connection::open(&target.database).unwrap();
        c.execute_batch(sql).unwrap();
        let preview = target
            .service
            .preview_import(&incoming, "synthetic.json".into())
            .unwrap();
        assert!(
            target
                .service
                .restore_import(RestoreInput {
                    token: preview.token,
                    expected_generation: preview.generation
                })
                .is_err()
        );
        assert_eq!(target.service.export_bytes().unwrap(), before);
        c.execute_batch(&format!("DROP TRIGGER {name}")).unwrap();
        target.reopen();
        assert_eq!(target.service.export_bytes().unwrap(), before);
    }
    let backups: Vec<_> = std::fs::read_dir(&target.backups)
        .unwrap()
        .map(|e| e.unwrap().path())
        .filter(|p| {
            p.file_name()
                .unwrap()
                .to_string_lossy()
                .starts_with("before-restore-")
        })
        .map(|p| std::fs::read(p).unwrap())
        .collect();
    assert_eq!(backups.len(), 2);
    assert!(backups.iter().all(|bytes| bytes == &before));
    target.restore(&incoming);
    assert_eq!(target.service.export_bytes().unwrap(), incoming);
}

#[test]
fn modern_and_legacy_world_presence_rules_are_strict() {
    let mut f = Fixture::new();
    let bytes = f.service.export_bytes().unwrap();
    let source: Value = serde_json::from_slice(&bytes).unwrap();
    let mut missing = source.clone();
    missing.as_object_mut().unwrap().remove("world");
    assert!(
        f.service
            .preview_import(
                &serde_json::to_vec(&missing).unwrap(),
                "synthetic.json".into()
            )
            .is_err()
    );
    let mut extra = source;
    extra["schema_version"] = json!(3);
    assert!(
        f.service
            .preview_import(
                &serde_json::to_vec(&extra).unwrap(),
                "synthetic.json".into()
            )
            .is_err()
    );
    assert_eq!(f.service.export_bytes().unwrap(), bytes);
}
