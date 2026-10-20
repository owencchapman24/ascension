//! Repeatable, synthetic real-SQLite measurements. Run explicitly in release mode.
use ascension_lib::study::*;
use ascension_lib::{domain::*, progress::*, service::WorkspaceService};
use chrono::{DateTime, Duration, SecondsFormat, Utc};
use std::hint::black_box;
use std::time::Instant;
use uuid::Uuid;

fn restore(service: &mut WorkspaceService, bytes: &[u8]) -> Workspace {
    let preview = service
        .preview_import(bytes, "synthetic-benchmark.json".into())
        .unwrap();
    service
        .restore_import(RestoreInput {
            expected_generation: preview.generation,
            token: preview.token,
        })
        .unwrap()
        .workspace
}

fn timed<T>(size: usize, operation: &str, mut f: impl FnMut() -> T) {
    let mut samples = Vec::new();
    for _ in 0..3 {
        let start = Instant::now();
        black_box(f());
        samples.push(start.elapsed().as_secs_f64() * 1000.0);
    }
    samples.sort_by(f64::total_cmp);
    println!(
        "BENCH,{size},{operation},{:.3},{:.3}",
        samples[1], samples[2]
    );
    assert!(
        samples[2] < 60_000.0,
        "Severe (>60s) bounded operation regression"
    );
}

fn reward(
    service: &mut WorkspaceService,
    workspace: Workspace,
    parent: &str,
    correct: bool,
) -> Workspace {
    let criterion = workspace
        .progress
        .criteria
        .iter()
        .find(|c| c.parent_id == parent)
        .unwrap()
        .clone();
    let workspace = service
        .lock_criterion(LockCriterionInput {
            expected_generation: workspace.generation,
            parent_id: parent.into(),
            criterion_lineage_id: criterion.criterion_lineage_id.clone(),
            criterion_version: criterion.criterion_version,
        })
        .unwrap();
    let workspace = service
        .record_evidence(RecordEvidenceInput {
            expected_generation: workspace.generation,
            parent_id: parent.into(),
            criterion_lineage_id: criterion.criterion_lineage_id.clone(),
            criterion_version: criterion.criterion_version,
            kind: EvidenceKind::UserAttestation,
            description: "Inspected the synthetic result and recorded all required steps.".into(),
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
            expected_generation: workspace.generation,
            parent_id: parent.into(),
            criterion_lineage_id: criterion.criterion_lineage_id.clone(),
            criterion_version: criterion.criterion_version,
            evidence_refs: workspace
                .progress
                .evidence
                .iter()
                .filter(|e| e.parent_id == parent)
                .map(|e| e.id.clone())
                .collect(),
            factual_summary: "The synthetic result meets the declared criterion.".into(),
            assistance: Assistance::None,
        })
        .unwrap();
    let accepted = service
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
        .unwrap();
    if correct {
        service
            .correct_acceptance(CorrectAcceptanceInput {
                expected_generation: accepted.workspace.generation,
                acceptance_id: accepted.result.acceptance_id,
                kind: CorrectionKind::HistoricalReopened,
                reason: "The synthetic work was valid and is now reopened for additional detail."
                    .into(),
            })
            .unwrap()
    } else {
        accepted.workspace
    }
}

#[test]
#[ignore = "explicit release benchmark; prints median/worst timings, not a normal test gate"]
fn stage3b_authority_baseline() {
    println!("BENCH,objectives,operation,median_ms,worst_ms");
    for size in [10, 100, 500, 1_000, 1_500] {
        let directory = tempfile::tempdir().unwrap();
        let database = directory.path().join("synthetic.sqlite3");
        let backups = directory.path().join("backups");
        let mut service = WorkspaceService::open(&database, &backups).unwrap();
        let mut records: RecordSet =
            serde_json::from_slice(include_bytes!("../../contracts/fixtures/sample-v1.json"))
                .unwrap();
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
        for index in 0..size {
            let mut row = objective.clone();
            row.id = Uuid::new_v4().to_string();
            row.title = format!("Synthetic objective {index}");
            let mut next = action.clone();
            next.id = Uuid::new_v4().to_string();
            next.objective_id = row.id.clone();
            records.objectives.push(row);
            records.actions.push(next);
        }
        records.selected_objective_id = Some(records.objectives[0].id.clone());
        let mut workspace = restore(&mut service, &serde_json::to_vec(&records).unwrap());
        let parents: Vec<String> = workspace
            .objectives
            .iter()
            .take(8)
            .map(|o| o.id.clone())
            .collect();
        for (index, parent) in parents.iter().enumerate() {
            workspace = reward(&mut service, workspace, parent, index % 2 == 0);
        }
        let bytes = service.export_bytes().unwrap();
        println!(
            "FIXTURE,{size},bytes={},grants={},corrections={}",
            bytes.len(),
            workspace.progress.grants.len(),
            workspace.progress.corrections.len()
        );
        timed(size, "cold_repository_open", || {
            WorkspaceService::open(&database, &backups).unwrap()
        });
        timed(size, "load_workspace", || service.load_workspace().unwrap());
        let mut revision = 0;
        timed(size, "next_action_save", || {
            revision += 1;
            workspace = service
                .save_work(SaveWorkInput {
                    expected_generation: workspace.generation.clone(),
                    objective_id: parents[0].clone(),
                    description: format!("Inspect synthetic benchmark revision {revision}"),
                    stop_condition: "One result documented".into(),
                    note: Some("Resume at synthetic example".into()),
                })
                .unwrap();
        });
        let c = workspace
            .progress
            .criteria
            .iter()
            .find(|c| c.parent_id == parents[0])
            .unwrap()
            .clone();
        timed(size, "progress_review", || {
            let result = service
                .review_progress(ReviewProgressInput {
                    expected_generation: workspace.generation.clone(),
                    parent_id: c.parent_id.clone(),
                    criterion_lineage_id: c.criterion_lineage_id.clone(),
                    criterion_version: c.criterion_version,
                    evidence_refs: workspace
                        .progress
                        .evidence
                        .iter()
                        .filter(|e| e.parent_id == c.parent_id)
                        .map(|e| e.id.clone())
                        .collect(),
                    factual_summary: "Synthetic benchmark review of retained historical evidence"
                        .into(),
                    assistance: Assistance::None,
                })
                .unwrap();
            workspace = result.workspace;
        });
        timed(size, "ledger_h1_projection", || {
            workspace
                .progress
                .ledger
                .iter()
                .map(|e| e.amount_bc)
                .sum::<u32>()
                .min(64)
        });
        timed(size, "export", || service.export_bytes().unwrap());
        timed(size, "import_preview", || {
            service
                .preview_import(&bytes, "synthetic.json".into())
                .unwrap()
        });
        timed(size, "restore_into_empty", || {
            let destination = tempfile::tempdir().unwrap();
            let mut target = WorkspaceService::open(
                &destination.path().join("restore.sqlite3"),
                &destination.path().join("backups"),
            )
            .unwrap();
            restore(&mut target, &bytes)
        });
        // A repository-contained synthetic populated baseline for the installed
        // migration test. Never read or overwrite a normal application profile.
        if size == 10 && std::env::var_os("ASCENSION_CAPTURE_BASELINE").is_some() {
            let output = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .parent()
                .unwrap()
                .join(".artifacts/stage3b-baseline");
            std::fs::create_dir_all(&output).unwrap();
            let final_bytes = service.export_bytes().unwrap();
            drop(service);
            std::fs::copy(&database, output.join("ascension.sqlite3")).unwrap();
            std::fs::write(output.join("records-v2.json"), final_bytes).unwrap();
        }
    }
}

fn timestamp(value: DateTime<Utc>) -> String {
    value.to_rfc3339_opts(SecondsFormat::Millis, true)
}

fn ordinary_opportunity(generation: &str, objective: &str, epoch: &str) -> PlanOpportunityInput {
    PlanOpportunityInput {
        expected_generation: generation.into(),
        objective_id: objective.into(),
        epoch_id: epoch.into(),
        manifest_id: None,
        domain: "Synthetic workshop".into(),
        meaningful_start: "Inspect one synthetic assembly step and document its result.".into(),
        cue_at: timestamp(Utc::now() + Duration::hours(1)),
        timezone: "UTC".into(),
        window_seconds: 900,
        eligible: true,
        unavailability: None,
        priority: Priority::Ordinary,
        available_minutes: 30,
        deadline_at: None,
        context: None,
    }
}

// Fixture setup intentionally avoids N sequential whole-workspace mutations.
// The shape comes from a real public planning command. Only synthetic IDs,
// references and historical times are expanded; the ordinary import validator,
// backup route and real SQLite restore still validate every generated record.
fn opportunity_fixture(service: &mut WorkspaceService, size: usize) -> Workspace {
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
    for index in 0..10 {
        let mut parent = objective.clone();
        parent.id = Uuid::new_v4().to_string();
        parent.title = format!("Synthetic opportunity benchmark objective {index}");
        let mut next = action.clone();
        next.id = Uuid::new_v4().to_string();
        next.objective_id = parent.id.clone();
        records.objectives.push(parent);
        records.actions.push(next);
    }
    records.selected_objective_id = Some(records.objectives[0].id.clone());
    let mut workspace = restore(service, &serde_json::to_vec(&records).unwrap());
    let parents: Vec<String> = workspace
        .objectives
        .iter()
        .take(8)
        .map(|p| p.id.clone())
        .collect();
    for (index, parent) in parents.iter().enumerate() {
        workspace = reward(service, workspace, parent, index % 2 == 0);
    }

    let beginning = Utc::now() - Duration::days(30);
    workspace = service
        .create_schedule_epoch(CreateScheduleEpochInput {
            expected_generation: workspace.generation,
            label: "Synthetic historical benchmark epoch".into(),
            effective_from: timestamp(beginning - Duration::days(1)),
            timezone: "UTC".into(),
        })
        .unwrap();
    let epoch_id = workspace.study.epochs[0].id.clone();
    workspace = service
        .plan_opportunity(ordinary_opportunity(
            &workspace.generation,
            &parents[0],
            &epoch_id,
        ))
        .unwrap();
    let template = workspace.study.opportunities[0].clone();
    let mut records = RecordSet::from(&workspace);
    let progress = records.progress.as_ref().unwrap();
    let study = records.study.as_mut().unwrap();
    study.epochs[0].created_at = timestamp(beginning - Duration::days(1));
    study.opportunities.clear();
    for index in 0..size {
        let parent = &records.objectives[index % records.objectives.len()];
        let action = records
            .actions
            .iter()
            .find(|a| a.objective_id == parent.id)
            .unwrap();
        let criterion = progress
            .criteria
            .iter()
            .find(|c| c.parent_id == parent.id)
            .unwrap();
        let cue = beginning + Duration::minutes(index as i64 * 2);
        let mut opportunity = template.clone();
        opportunity.id = Uuid::new_v4().to_string();
        opportunity.project_id = parent.project_id.clone();
        opportunity.objective_id = parent.id.clone();
        opportunity.action_id = action.id.clone();
        opportunity.action_revision = action.revision;
        opportunity.action_snapshot = action.description.clone();
        opportunity.stop_snapshot = action.stop_condition.clone();
        opportunity.criterion_id = criterion.id.clone();
        opportunity.criterion_version = criterion.criterion_version;
        opportunity.criterion_snapshot = criterion.description.clone();
        opportunity.criterion_registry_revision = criterion.registry_revision;
        opportunity.created_at = timestamp(cue - Duration::minutes(20));
        opportunity.eligibility_decided_at = opportunity.created_at.clone();
        opportunity.cue_at = timestamp(cue);
        let report = match index % 4 {
            0 | 3 => Report::Started,
            1 => Report::Nonstart,
            _ => Report::Unknown,
        };
        let started = report == Report::Started;
        let late = index % 4 == 3;
        let mut observation = OutcomeEvent {
            id: Uuid::new_v4().to_string(),
            opportunity_id: opportunity.id.clone(),
            sequence: 1,
            report,
            started_at: started
                .then(|| timestamp(cue + Duration::seconds(if late { 1200 } else { 120 }))),
            precision: if started {
                Precision::Exact
            } else {
                Precision::Unknown
            },
            uncertainty_seconds: None,
            observed_at: timestamp(cue + Duration::seconds(1500)),
            source: "local_user_self_report".into(),
            note: Some(
                "Entirely synthetic observation for bounded engineering measurements.".into(),
            ),
            technical_failure: false,
            uncertainty_reason: (report == Report::Unknown)
                .then(|| "Synthetic missing observation; timing is unknown.".into()),
            correction_reason: None,
            supersedes: None,
            state: match (report, late) {
                (Report::Started, false) => OutcomeState::ObservedStart,
                (Report::Started | Report::Nonstart, _) => OutcomeState::ObservedNonstart,
                _ => OutcomeState::Missing,
            },
            latency_seconds: (started && !late).then_some(120),
            censored_at_seconds: (report == Report::Nonstart || late).then_some(900),
            late_start: late,
            priority_advanced: started.then_some(true),
            quality_note: Some(
                "Synthetic step checked against the existing bounded objective.".into(),
            ),
            acceptance_id: None,
        };
        study.outcomes.push(observation.clone());
        if index % 10 == 0 {
            observation.supersedes = Some(observation.id.clone());
            observation.id = Uuid::new_v4().to_string();
            observation.sequence = 2;
            observation.observed_at = timestamp(cue + Duration::seconds(1800));
            observation.correction_reason =
                Some("Synthetic factual-note clarification; original timing is preserved.".into());
            observation.note = Some(
                "Clarified the synthetic assembly result without rewriting its original report."
                    .into(),
            );
            study.outcomes.push(observation);
        }
        study.burdens.push(BurdenEvent {
            id: Uuid::new_v4().to_string(),
            opportunity_id: Some(opportunity.id.clone()),
            category: BurdenCategory::Logging,
            minutes: (index % 3 + 1) as u32,
            note: Some("Synthetic exclusive capture-time estimate.".into()),
            created_at: timestamp(cue + Duration::seconds(1800)),
            method: "user_estimate_exclusive_minutes".into(),
        });
        if index % 5 == 0 {
            study.exposures.push(ExposureEvent {
                id: Uuid::new_v4().to_string(),
                opportunity_id: Some(opportunity.id.clone()),
                kind: ExposureKind::CueDelivered,
                note: Some("Synthetic cue-delivery record.".into()),
                created_at: timestamp(cue),
                issued_bc: 0,
                ledger_sequence: 0,
                protocol_event_id: None,
                preference_sequence: 0,
                h1_saturated: false,
            });
        }
        study.opportunities.push(opportunity);
    }
    restore(service, &serde_json::to_vec(&records).unwrap())
}

fn record_synthetic_outcome(
    service: &mut WorkspaceService,
    workspace: &Workspace,
    id: &str,
) -> Workspace {
    service
        .record_opportunity_outcome(RecordOpportunityOutcomeInput {
            expected_generation: workspace.generation.clone(),
            opportunity_id: id.into(),
            report: Report::Unknown,
            started_at: None,
            precision: Precision::Unknown,
            uncertainty_seconds: None,
            note: Some("Synthetic retrospective correction for performance verification.".into()),
            technical_failure: false,
            uncertainty_reason: Some("Synthetic observation is left unknown.".into()),
            correction_reason: Some(
                "Synthetic measurement appends a correction, retaining prior observations.".into(),
            ),
            priority_advanced: None,
            quality_note: None,
            acceptance_id: None,
        })
        .unwrap()
}

#[test]
#[ignore = "explicit release benchmark; measures full validated opportunity history"]
fn stage3c_opportunity_scaling() {
    println!("OPPORTUNITY_BENCH,opportunities,operation,median_ms,worst_ms");
    for size in [10, 100, 500, 1_000, 2_000] {
        let directory = tempfile::tempdir().unwrap();
        let database = directory.path().join("opportunity-benchmark.sqlite3");
        let backups = directory.path().join("backups");
        let mut service = WorkspaceService::open(&database, &backups).unwrap();
        let mut workspace = opportunity_fixture(&mut service, size);
        let bytes = service.export_bytes().unwrap();
        let objective = workspace.objectives[0].id.clone();
        let epoch = workspace.study.epochs[0].id.clone();
        let historical = workspace.study.opportunities[0].id.clone();
        println!(
            "OPPORTUNITY_FIXTURE,{size},bytes={},objectives={},evidence={},grants={},acceptance_corrections={},outcomes={},outcome_corrections={},burdens={},exposures={}",
            bytes.len(),
            workspace.objectives.len(),
            workspace.progress.evidence.len(),
            workspace.progress.grants.len(),
            workspace.progress.corrections.len(),
            workspace.study.outcomes.len(),
            workspace
                .study
                .outcomes
                .iter()
                .filter(|e| e.supersedes.is_some())
                .count(),
            workspace.study.burdens.len(),
            workspace.study.exposures.len()
        );
        timed(size, "opportunity_cold_repository_open", || {
            WorkspaceService::open(&database, &backups).unwrap()
        });
        timed(size, "opportunity_load_workspace", || {
            service.load_workspace().unwrap()
        });
        timed(size, "opportunity_serialize_frontend_view", || {
            serde_json::to_vec(&workspace).unwrap()
        });
        let mut revision = 0;
        timed(size, "opportunity_next_action_save", || {
            revision += 1;
            workspace = service
                .save_work(SaveWorkInput {
                    expected_generation: workspace.generation.clone(),
                    objective_id: objective.clone(),
                    description: format!("Inspect synthetic benchmark action revision {revision}"),
                    stop_condition: "One synthetic result documented".into(),
                    note: None,
                })
                .unwrap();
        });
        timed(size, "opportunity_create", || {
            workspace = service
                .plan_opportunity(ordinary_opportunity(
                    &workspace.generation,
                    &objective,
                    &epoch,
                ))
                .unwrap();
        });
        timed(size, "opportunity_record_outcome", || {
            workspace = record_synthetic_outcome(&mut service, &workspace, &historical);
        });
        let criterion = workspace
            .progress
            .criteria
            .iter()
            .find(|c| c.parent_id == objective)
            .unwrap()
            .clone();
        timed(size, "opportunity_progress_review", || {
            let result = service
                .review_progress(ReviewProgressInput {
                    expected_generation: workspace.generation.clone(),
                    parent_id: objective.clone(),
                    criterion_lineage_id: criterion.criterion_lineage_id.clone(),
                    criterion_version: criterion.criterion_version,
                    evidence_refs: workspace
                        .progress
                        .evidence
                        .iter()
                        .filter(|e| e.parent_id == objective)
                        .map(|e| e.id.clone())
                        .collect(),
                    factual_summary:
                        "Synthetic performance review of registered historical evidence".into(),
                    assistance: Assistance::None,
                })
                .unwrap();
            workspace = result.workspace;
        });
        timed(size, "opportunity_ledger_h1_projection", || {
            workspace
                .progress
                .ledger
                .iter()
                .map(|e| e.amount_bc)
                .sum::<u32>()
                .min(64)
        });
        timed(size, "opportunity_export", || {
            service.export_bytes().unwrap()
        });
        timed(size, "opportunity_import_validation", || {
            service
                .preview_import(&bytes, "synthetic-opportunities.json".into())
                .unwrap()
        });
        timed(size, "opportunity_restore_into_empty", || {
            let destination = tempfile::tempdir().unwrap();
            let mut target = WorkspaceService::open(
                &destination.path().join("restore.sqlite3"),
                &destination.path().join("backups"),
            )
            .unwrap();
            restore(&mut target, &bytes)
        });
        assert_eq!(
            workspace.progress.wallet_bc, 32,
            "Observation benchmark must not issue any rewards"
        );
        assert_eq!(workspace.progress.grants.len(), 8);
    }
}

#[test]
fn bounded_opportunity_history_avoids_severe_operation_regressions() {
    let directory = tempfile::tempdir().unwrap();
    let database = directory.path().join("bounded-opportunities.sqlite3");
    let backups = directory.path().join("backups");
    let mut service = WorkspaceService::open(&database, &backups).unwrap();
    let workspace = opportunity_fixture(&mut service, 1_000);
    let started = Instant::now();
    let reopened = WorkspaceService::open(&database, &backups).unwrap();
    let loaded = reopened.load_workspace().unwrap();
    assert_eq!(loaded.study.opportunities.len(), 1_000);
    assert!(
        started.elapsed().as_secs() < 30,
        "Severe cold-open/load regression for a bounded 1,000-opportunity history"
    );
    let view = serde_json::to_value(&loaded).unwrap();
    let opportunities = view["study"]["opportunities"].as_array().unwrap();
    assert_eq!(
        opportunities
            .iter()
            .filter(|o| o["outcome_state"] == "observed_start")
            .count(),
        250
    );
    assert_eq!(
        opportunities
            .iter()
            .filter(|o| o["outcome_state"] == "observed_nonstart")
            .count(),
        500
    );
    assert_eq!(
        opportunities
            .iter()
            .filter(|o| o["outcome_state"] == "missing")
            .count(),
        250
    );
    let corrected_id = workspace
        .study
        .opportunities
        .iter()
        .find(|o| {
            workspace
                .study
                .latest_outcome(&o.id)
                .is_some_and(|e| e.state == OutcomeState::ObservedStart)
        })
        .unwrap()
        .id
        .clone();
    let started = Instant::now();
    let after = record_synthetic_outcome(&mut service, &workspace, &corrected_id);
    assert_eq!(after.progress, workspace.progress);
    let corrected_view = serde_json::to_value(&after).unwrap();
    let corrected = corrected_view["study"]["opportunities"]
        .as_array()
        .unwrap()
        .iter()
        .find(|o| o["id"] == corrected_id)
        .unwrap();
    assert_eq!(corrected["outcome_state"], "missing");
    assert!(
        started.elapsed().as_secs() < 30,
        "Severe outcome-recording regression for a bounded 1,000-opportunity history"
    );
}
