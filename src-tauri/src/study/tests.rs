use super::*;
use crate::domain::*;
use crate::service::WorkspaceService;
use serde_json::{Value, json};
use std::path::PathBuf;

const BASE: &str = "2030-01-01T10:00:00.000Z";
fn clock(seconds: i64) {
    TEST_CLOCK.with(|c| {
        *c.borrow_mut() = Some(
            (stamp(BASE).unwrap() + chrono::Duration::seconds(seconds))
                .to_rfc3339_opts(SecondsFormat::Millis, true),
        )
    });
}
fn time(seconds: i64) -> String {
    (stamp(BASE).unwrap() + chrono::Duration::seconds(seconds))
        .to_rfc3339_opts(SecondsFormat::Millis, true)
}
struct Fixture {
    _dir: tempfile::TempDir,
    db: PathBuf,
    backups: PathBuf,
    service: WorkspaceService,
}
impl Fixture {
    fn new() -> Self {
        clock(0);
        let dir = tempfile::tempdir().unwrap();
        let db = dir.path().join("synthetic.sqlite3");
        let backups = dir.path().join("backups");
        let mut service = WorkspaceService::open(&db, &backups).unwrap();
        let w = service.load_workspace().unwrap();
        let w = service
            .create_project(CreateProjectInput {
                expected_generation: w.generation,
                title: "Synthetic study project".into(),
            })
            .unwrap();
        let w = service
            .create_objective(CreateObjectiveInput {
                expected_generation: w.generation,
                project_id: w.projects[0].id.clone(),
                title: "Trace a synthetic failure".into(),
                criterion: "A reproducible example documents the cause".into(),
            })
            .unwrap();
        let w = service
            .save_work(SaveWorkInput {
                expected_generation: w.generation,
                objective_id: w.objectives[0].id.clone(),
                description: "Reproduce the example failure".into(),
                stop_condition: "Record the result and diagnosis".into(),
                note: None,
            })
            .unwrap();
        service
            .create_schedule_epoch(CreateScheduleEpochInput {
                expected_generation: w.generation,
                label: "Synthetic ordinary routine".into(),
                effective_from: time(-3600),
                timezone: "Etc/UTC".into(),
            })
            .unwrap();
        Self {
            _dir: dir,
            db,
            backups,
            service,
        }
    }
    fn ws(&self) -> Workspace {
        self.service.load_workspace().unwrap()
    }
    fn protocol(&mut self, phase: Phase) -> String {
        let w = self.ws();
        let w = self
            .service
            .configure_study(ConfigureStudyInput {
                expected_generation: w.generation,
                phase,
                epoch_id: w.study.epochs[0].id.clone(),
            })
            .unwrap();
        let id = w.study.manifests[0].id.clone();
        if phase == Phase::E01 {
            self.transition(&id, StudyStatus::Ready);
        }
        self.transition(&id, StudyStatus::Active);
        id
    }
    fn transition(&mut self, id: &str, status: StudyStatus) {
        let w = self.ws();
        self.service
            .transition_study(TransitionStudyInput {
                expected_generation: w.generation,
                manifest_id: id.into(),
                status,
                reason: "Synthetic owner decision after feasibility assessment".into(),
                acknowledgements: if status == StudyStatus::Active {
                    ACKS.iter().map(|s| s.to_string()).collect()
                } else {
                    vec![]
                },
            })
            .unwrap();
    }
    fn plan_input(&self, cue: i64, manifest: Option<&str>) -> PlanOpportunityInput {
        let w = self.ws();
        PlanOpportunityInput {
            expected_generation: w.generation,
            objective_id: w.objectives[0].id.clone(),
            epoch_id: w.study.epochs[0].id.clone(),
            manifest_id: manifest.map(str::to_owned),
            domain: "coding".into(),
            meaningful_start: "Reproduce the specified failure and record the result".into(),
            cue_at: time(cue),
            timezone: "Etc/UTC".into(),
            window_seconds: 900,
            eligible: true,
            unavailability: None,
            priority: Priority::Ordinary,
            available_minutes: 30,
            deadline_at: None,
            context: Some("Low energy remains eligible".into()),
        }
    }
    fn plan(&mut self, cue: i64, manifest: Option<&str>) -> String {
        let input = self.plan_input(cue, manifest);
        let before = self.ws();
        let w = self.service.plan_opportunity(input).unwrap();
        w.study
            .opportunities
            .iter()
            .find(|o| !before.study.opportunities.iter().any(|p| p.id == o.id))
            .unwrap()
            .id
            .clone()
    }
    fn input(&self, id: &str, report: Report, start: Option<i64>) -> RecordOpportunityOutcomeInput {
        RecordOpportunityOutcomeInput {
            expected_generation: self.ws().generation,
            opportunity_id: id.into(),
            report,
            started_at: start.map(time),
            precision: if start.is_some() {
                Precision::Exact
            } else {
                Precision::Unknown
            },
            uncertainty_seconds: None,
            note: Some("Synthetic factual report".into()),
            technical_failure: false,
            uncertainty_reason: if report == Report::Unknown {
                Some("No reliable observation".into())
            } else {
                None
            },
            correction_reason: None,
            priority_advanced: None,
            quality_note: None,
            acceptance_id: None,
        }
    }
    fn report(&mut self, id: &str, report: Report, start: Option<i64>) -> Workspace {
        self.service
            .record_opportunity_outcome(self.input(id, report, start))
            .unwrap()
    }
    fn restore(&mut self, bytes: &[u8]) -> Workspace {
        let p = self
            .service
            .preview_import(bytes, "synthetic.json".into())
            .unwrap();
        self.service
            .restore_import(RestoreInput {
                expected_generation: p.generation,
                token: p.token,
            })
            .unwrap()
            .workspace
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        TEST_CLOCK.with(|c| *c.borrow_mut() = None);
    }
}

#[test]
fn prospective_opportunity_snapshots_survive_work_edits_restart_and_do_not_mint() {
    let mut f = Fixture::new();
    let id = f.plan(300, None);
    let w = f.ws();
    let snapshot = w.study.opportunities[0].clone();
    let w = f
        .service
        .save_work(SaveWorkInput {
            expected_generation: w.generation,
            objective_id: w.objectives[0].id.clone(),
            description: "Inspect another code path".into(),
            stop_condition: "Document another diagnosis".into(),
            note: Some("Resume later".into()),
        })
        .unwrap();
    assert_eq!(w.study.opportunities[0], snapshot);
    assert!(w.progress.grants.is_empty());
    let w = f
        .service
        .update_objective(UpdateObjectiveInput {
            expected_generation: w.generation,
            id: w.objectives[0].id.clone(),
            title: w.objectives[0].title.clone(),
            criterion: "A changed prospective draft criterion".into(),
        })
        .unwrap();
    assert_eq!(w.study.opportunities[0], snapshot);
    assert_ne!(
        w.objectives[0].criterion,
        w.study.opportunities[0].criterion_snapshot
    );
    let reopened = WorkspaceService::open(&f.db, &f.backups)
        .unwrap()
        .load_workspace()
        .unwrap();
    assert_eq!(reopened.study.opportunities[0].id, id);
    assert_eq!(reopened.study, w.study);
}
#[test]
fn opportunity_must_precede_cue_and_requires_real_availability() {
    let mut f = Fixture::new();
    let before = f.ws();
    assert!(f.service.plan_opportunity(f.plan_input(0, None)).is_err());
    let mut i = f.plan_input(300, None);
    i.eligible = false;
    assert!(f.service.plan_opportunity(i.clone()).is_err());
    i.unavailability = Some("Planned rest".into());
    let w = f.service.plan_opportunity(i).unwrap();
    assert!(!w.study.opportunities[0].eligible);
    assert!(w.study.assignments.is_empty());
    assert_eq!(before.progress, w.progress);
}
#[test]
fn untouched_due_opportunity_is_missing_with_no_fabricated_latency() {
    let mut f = Fixture::new();
    f.plan(300, None);
    clock(1201);
    let w = f.ws();
    assert!(w.study.outcomes.is_empty());
    let v = serde_json::to_value(&w).unwrap();
    assert_eq!(v["study"]["opportunities"][0]["outcome_state"], "missing");
    assert!(
        v["study"]["opportunities"][0]
            .get("latency_seconds")
            .is_none()
    );
    assert!(w.progress.grants.is_empty());
}
#[test]
fn observed_start_exact_boundary_and_nonstart_are_distinct() {
    let mut f = Fixture::new();
    let first = f.plan(300, None);
    let second = f.plan(1500, None);
    clock(1200);
    let w = f.report(&first, Report::Started, Some(1200));
    let e = w.study.latest_outcome(&first).unwrap();
    assert_eq!(e.state, OutcomeState::ObservedStart);
    assert_eq!(e.latency_seconds, Some(900));
    assert_eq!(e.censored_at_seconds, None);
    clock(2400);
    let w = f.report(&second, Report::Nonstart, None);
    let e = w.study.latest_outcome(&second).unwrap();
    assert_eq!(e.state, OutcomeState::ObservedNonstart);
    assert_eq!(e.censored_at_seconds, Some(900));
    assert_eq!(e.latency_seconds, None);
}
#[test]
fn early_nonstart_and_future_report_are_rejected_without_writes() {
    let mut f = Fixture::new();
    let id = f.plan(300, None);
    clock(500);
    let before = f.ws();
    assert!(
        f.service
            .record_opportunity_outcome(f.input(&id, Report::Nonstart, None))
            .is_err()
    );
    assert!(
        f.service
            .record_opportunity_outcome(f.input(&id, Report::Started, Some(600)))
            .is_err()
    );
    let mut uncertain = f.input(&id, Report::Started, None);
    uncertain.uncertainty_reason = Some("Timing is not ascertainable yet".into());
    assert!(f.service.record_opportunity_outcome(uncertain).is_err());
    assert_eq!(f.ws(), before);
}
#[test]
fn late_start_is_censored_primary_nonstart_and_preserves_late_observation() {
    let mut f = Fixture::new();
    let id = f.plan(300, None);
    clock(1500);
    let w = f.report(&id, Report::Started, Some(1250));
    let e = w.study.latest_outcome(&id).unwrap();
    assert_eq!(e.state, OutcomeState::ObservedNonstart);
    assert!(e.late_start);
    assert_eq!(e.started_at, Some(time(1250)));
    assert_eq!(e.censored_at_seconds, Some(900));
    assert_eq!(e.latency_seconds, None);
}
#[test]
fn approximate_boundary_crossing_and_unknown_times_preserve_uncertainty() {
    let mut f = Fixture::new();
    let a = f.plan(300, None);
    let b = f.plan(1500, None);
    clock(1300);
    let mut input = f.input(&a, Report::Started, Some(1190));
    input.precision = Precision::Approximate;
    input.uncertainty_seconds = Some(30);
    input.uncertainty_reason = Some("Retrospective estimate crosses the window boundary".into());
    let w = f.service.record_opportunity_outcome(input).unwrap();
    let e = w.study.latest_outcome(&a).unwrap();
    assert_eq!(e.state, OutcomeState::Missing);
    assert_eq!(e.latency_seconds, None);
    assert_eq!(e.censored_at_seconds, None);
    clock(2500);
    let mut input = f.input(&b, Report::Started, None);
    input.uncertainty_reason = Some("Cannot recall the start time".into());
    let w = f.service.record_opportunity_outcome(input).unwrap();
    assert_eq!(
        w.study.latest_outcome(&b).unwrap().state,
        OutcomeState::Missing
    );
}
#[test]
fn outcome_corrections_are_append_only_and_require_explanation() {
    let mut f = Fixture::new();
    let id = f.plan(300, None);
    clock(1300);
    let w = f.report(&id, Report::Nonstart, None);
    let old = w.study.outcomes[0].clone();
    let input = f.input(&id, Report::Started, Some(400));
    assert!(f.service.record_opportunity_outcome(input.clone()).is_err());
    let mut input = input;
    input.correction_reason = Some("The written example confirms a start at 10:06:40".into());
    let w = f.service.record_opportunity_outcome(input).unwrap();
    assert_eq!(w.study.outcomes.len(), 2);
    assert!(w.study.outcomes.contains(&old));
    let latest = w.study.latest_outcome(&id).unwrap();
    assert_eq!(latest.state, OutcomeState::ObservedStart);
    assert_eq!(latest.supersedes, Some(old.id));
    assert!(w.progress.grants.is_empty());
}
#[test]
fn e00_is_explicit_and_never_randomized() {
    let mut f = Fixture::new();
    assert!(f.ws().study.manifests.is_empty());
    let id = f.protocol(Phase::E00);
    f.plan(300, Some(&id));
    let w = f.ws();
    assert_eq!(w.study.status(&id), StudyStatus::Active);
    assert!(w.study.manifests[0].allocation.is_empty());
    assert!(w.study.assignments.is_empty());
    assert_eq!(presentation(&w.study, &time(300)).condition, None);
}
#[test]
fn e01_activation_requires_readiness_and_all_acknowledgements() {
    let mut f = Fixture::new();
    let w = f.ws();
    let w = f
        .service
        .configure_study(ConfigureStudyInput {
            expected_generation: w.generation,
            phase: Phase::E01,
            epoch_id: w.study.epochs[0].id.clone(),
        })
        .unwrap();
    let id = w.study.manifests[0].id.clone();
    let input = TransitionStudyInput {
        expected_generation: w.generation,
        manifest_id: id.clone(),
        status: StudyStatus::Active,
        reason: "Synthetic decision".into(),
        acknowledgements: vec![],
    };
    assert!(f.service.transition_study(input).is_err());
    f.transition(&id, StudyStatus::Ready);
    let input = TransitionStudyInput {
        expected_generation: f.ws().generation,
        manifest_id: id.clone(),
        status: StudyStatus::Active,
        reason: "Synthetic decision".into(),
        acknowledgements: vec!["e00_assessed".into()],
    };
    assert!(f.service.transition_study(input).is_err());
    assert_eq!(f.ws().study.status(&id), StudyStatus::Ready);
}
#[test]
fn frozen_sequence_has_fifteen_balanced_blocks_and_hides_future_allocations() {
    let mut f = Fixture::new();
    let id = f.protocol(Phase::E01);
    f.plan(300, Some(&id));
    let w = f.ws();
    let m = &w.study.manifests[0];
    assert_eq!(m.allocation.len(), 60);
    for block in m.allocation.chunks(4) {
        assert_eq!(block.iter().filter(|v| **v == Condition::D).count(), 2);
    }
    assert_eq!(m.allocation, allocation(&m.seed, 15));
    let v = serde_json::to_value(&w).unwrap();
    assert!(v["study"]["manifests"][0].get("seed").is_none());
    assert!(v["study"]["manifests"][0].get("allocation").is_none());
    assert!(v["study"]["assignments"][0]["condition"].is_null());
    let export: Value = serde_json::from_slice(&f.service.export_bytes().unwrap()).unwrap();
    assert_eq!(
        export["study"]["manifests"][0]["allocation"]
            .as_array()
            .unwrap()
            .len(),
        60
    );
}
#[test]
fn p5t029_both_arms_have_identical_work_and_reward_authority_with_isolated_delivery() {
    let mut f = Fixture::new();
    let manifest = f.protocol(Phase::E01);
    let before = f.ws().progress;
    let mut ids = vec![];
    for n in 0..4 {
        ids.push(f.plan(300 + n * 1200, Some(&manifest)));
    }
    let w = f.ws();
    assert_eq!(w.progress, before);
    let mut conditions = vec![];
    for (n, id) in ids.iter().enumerate() {
        clock(240 + n as i64 * 1200);
        let w = f.ws();
        let a = w
            .study
            .assignments
            .iter()
            .find(|a| &a.opportunity_id == id)
            .unwrap();
        conditions.push(a.condition);
        let p = presentation(&w.study, &study_now());
        assert_eq!(p.condition, Some(a.condition));
        assert_eq!(p.world_allowed, a.condition == Condition::DPlusB);
        let input = RecordStudyExposureInput {
            expected_generation: w.generation,
            opportunity_id: Some(id.clone()),
            kind: ExposureKind::PreviewDelivered,
            note: None,
        };
        let result = f.service.record_study_exposure(input);
        assert_eq!(result.is_ok(), a.condition == Condition::DPlusB);
        clock(400 + n as i64 * 1200);
        assert!(!presentation(&f.ws().study, &study_now()).world_allowed);
    }
    assert!(conditions.contains(&Condition::D));
    assert!(conditions.contains(&Condition::DPlusB));
    assert_eq!(f.ws().progress, before);
}
#[test]
fn p5t030_all_assignments_include_unopened_technical_failure_and_missing_after_restore() {
    let mut f = Fixture::new();
    let m = f.protocol(Phase::E01);
    let a = f.plan(300, Some(&m));
    let b = f.plan(1500, Some(&m));
    let c = f.plan(2700, Some(&m));
    clock(4000);
    let mut input = f.input(&a, Report::Unknown, None);
    input.technical_failure = true;
    f.service.record_opportunity_outcome(input).unwrap();
    f.report(&b, Report::Nonstart, None);
    let before = f.ws();
    let bytes = f.service.export_bytes().unwrap();
    let mut other = Fixture::new();
    clock(4000);
    let w = other.restore(&bytes);
    assert_eq!(w.study, before.study);
    assert_eq!(w.study.assignments.len(), 3);
    assert!(w.study.latest_outcome(&a).unwrap().technical_failure);
    assert!(w.study.latest_outcome(&c).is_none());
    let v = serde_json::to_value(&w).unwrap();
    assert_eq!(
        v["study"]["opportunities"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|o| o["outcome_state"] == "missing")
            .count(),
        2
    );
}
#[test]
fn rest_does_not_consume_allocation_and_assignments_cannot_overlap() {
    let mut f = Fixture::new();
    let m = f.protocol(Phase::E01);
    let mut i = f.plan_input(300, Some(&m));
    i.eligible = false;
    i.unavailability = Some("Rest".into());
    f.service.plan_opportunity(i).unwrap();
    assert!(f.ws().study.assignments.is_empty());
    f.plan(300, Some(&m));
    let before = f.ws();
    assert!(
        f.service
            .plan_opportunity(f.plan_input(600, Some(&m)))
            .is_err()
    );
    assert_eq!(f.ws(), before);
}
#[test]
fn epoch_change_pauses_new_assignment_without_rerandomizing_history() {
    let mut f = Fixture::new();
    let m = f.protocol(Phase::E01);
    f.plan(300, Some(&m));
    let before = f.ws();
    clock(5);
    let w = f
        .service
        .create_schedule_epoch(CreateScheduleEpochInput {
            expected_generation: before.generation,
            label: "Changed synthetic routine".into(),
            effective_from: time(0),
            timezone: "Etc/UTC".into(),
        })
        .unwrap();
    assert_eq!(w.study.status(&m), StudyStatus::Paused);
    assert_eq!(w.study.assignments, before.study.assignments);
    assert_eq!(w.study.manifests, before.study.manifests);
    assert!(
        f.service
            .plan_opportunity(f.plan_input(1800, Some(&m)))
            .is_err()
    );
}
#[test]
fn complete_restore_preserves_ids_corrections_burden_reflections_and_backups() {
    let mut f = Fixture::new();
    let m = f.protocol(Phase::E00);
    let id = f.plan(300, Some(&m));
    clock(1500);
    f.report(&id, Report::Unknown, None);
    let mut input = f.input(&id, Report::Started, Some(400));
    input.correction_reason = Some("Recovered contemporaneous note".into());
    input.priority_advanced = Some(true);
    input.quality_note = Some("The reproduction is understandable".into());
    f.service.record_opportunity_outcome(input).unwrap();
    f.service
        .record_study_burden(RecordStudyBurdenInput {
            expected_generation: f.ws().generation,
            opportunity_id: Some(id),
            category: BurdenCategory::Logging,
            minutes: 1,
            note: Some("Exclusive estimate, excludes useful work".into()),
        })
        .unwrap();
    f.service
        .record_study_reflection(RecordStudyReflectionInput {
            expected_generation: f.ws().generation,
            manifest_id: Some(m),
            note: "Synthetic reflection; no human feasibility claim".into(),
        })
        .unwrap();
    let before = f.ws();
    let bytes = f.service.export_bytes().unwrap();
    let mut other = Fixture::new();
    let restored = other.restore(&bytes);
    assert_eq!(restored.study, before.study);
    assert_eq!(restored.progress, before.progress);
    assert!(std::fs::read_dir(&other.backups).unwrap().next().is_some());
    let unchanged = other
        .service
        .preview_import(&bytes, "same.json".into())
        .unwrap();
    assert!(
        other
            .service
            .restore_import(RestoreInput {
                expected_generation: unchanged.generation,
                token: unchanged.token
            })
            .unwrap()
            .unchanged
    );
}
#[test]
fn older_exports_cannot_erase_study_history() {
    let mut f = Fixture::new();
    let bytes = f.service.export_bytes().unwrap();
    let mut old: Value = serde_json::from_slice(&bytes).unwrap();
    old["schema_version"] = json!(2);
    old.as_object_mut().unwrap().remove("study");
    let old = serde_json::to_vec(&old).unwrap();
    f.plan(300, None);
    let before = f.ws();
    assert_eq!(
        f.service
            .preview_import(&old, "legacy.json".into())
            .unwrap_err()
            .code,
        "unsafe_legacy_import"
    );
    assert_eq!(f.ws(), before);
}
#[test]
fn legacy_v2_initializes_empty_study_state_in_compatible_workspace() {
    let f = Fixture::new();
    let mut value: Value = serde_json::from_slice(&f.service.export_bytes().unwrap()).unwrap();
    value["schema_version"] = json!(2);
    value.as_object_mut().unwrap().remove("study");
    let d = tempfile::tempdir().unwrap();
    let mut service =
        WorkspaceService::open(&d.path().join("db"), &d.path().join("backups")).unwrap();
    let p = service
        .preview_import(&serde_json::to_vec(&value).unwrap(), "legacy.json".into())
        .unwrap();
    assert_eq!(p.schema_version, 2);
    let w = service
        .restore_import(RestoreInput {
            expected_generation: p.generation,
            token: p.token,
        })
        .unwrap()
        .workspace;
    assert_eq!(w.study, StudyState::default());
}
#[test]
fn forged_manifest_assignment_outcome_or_exposure_imports_preserve_local_state() {
    let mut f = Fixture::new();
    let m = f.protocol(Phase::E01);
    let id = f.plan(300, Some(&m));
    clock(1500);
    f.report(&id, Report::Nonstart, None);
    let before = f.ws();
    let original: Value = serde_json::from_slice(&f.service.export_bytes().unwrap()).unwrap();
    let mutations: Vec<fn(&mut Value)> = vec![
        |v| v["study"]["manifests"][0]["allocation_hash"] = json!("forged"),
        |v| v["study"]["manifests"][0]["allocation"][0] = json!("invalid"),
        |v| v["study"]["assignments"][0]["position"] = json!(2),
        |v| v["study"]["assignments"] = json!([]),
        |v| v["study"]["protocol_events"] = json!([]),
        |v| v["study"]["opportunities"][0]["eligible"] = json!(false),
        |v| v["study"]["opportunities"][0]["action_snapshot"] = json!("Changed historical action"),
        |v| v["study"]["outcomes"][0]["latency_seconds"] = json!(900),
        |v| v["study"]["outcomes"][0]["state"] = json!("observed_start"),
        |v| {
            v["study"]["outcomes"][0]["acceptance_id"] =
                json!("00000000-0000-0000-0000-000000000001")
        },
    ];
    for mutate in mutations {
        let mut v = original.clone();
        mutate(&mut v);
        assert!(
            f.service
                .preview_import(&serde_json::to_vec(&v).unwrap(), "malformed.json".into())
                .is_err()
        );
        assert_eq!(f.ws(), before);
    }
}
#[test]
fn database_triggers_protect_immutable_definitions_and_outcome_rows() {
    let mut f = Fixture::new();
    let m = f.protocol(Phase::E01);
    let id = f.plan(300, Some(&m));
    clock(1500);
    f.report(&id, Report::Nonstart, None);
    let c = Connection::open(&f.db).unwrap();
    for table in [
        "schedule_epochs",
        "study_manifests",
        "study_protocol_events",
        "work_opportunities",
        "study_assignments",
        "opportunity_outcomes",
    ] {
        assert!(
            c.execute(&format!("UPDATE {table} SET payload=payload"), [])
                .is_err()
        );
    }
    assert_eq!(f.ws().study.assignments.len(), 1);
}
#[test]
fn assignment_insert_failure_rolls_back_opportunity_and_generation() {
    let mut f = Fixture::new();
    let m = f.protocol(Phase::E01);
    let c = Connection::open(&f.db).unwrap();
    c.execute_batch("CREATE TRIGGER synthetic_assignment_failure BEFORE INSERT ON study_assignments BEGIN SELECT RAISE(ABORT,'synthetic failure'); END;").unwrap();
    let before = f.ws();
    assert!(
        f.service
            .plan_opportunity(f.plan_input(300, Some(&m)))
            .is_err()
    );
    assert_eq!(f.ws(), before);
    assert!(f.ws().study.opportunities.is_empty());
}
#[test]
fn concurrent_stale_planning_cannot_skip_or_duplicate_assignments() {
    let mut f = Fixture::new();
    let m = f.protocol(Phase::E01);
    let mut other = WorkspaceService::open(&f.db, &f.backups).unwrap();
    let input = f.plan_input(300, Some(&m));
    f.service.plan_opportunity(input.clone()).unwrap();
    assert_eq!(other.plan_opportunity(input).unwrap_err().code, "conflict");
    assert_eq!(f.ws().study.assignments.len(), 1);
    assert_eq!(f.ws().study.assignments[0].position, 0);
}
#[test]
fn populated_v3_migration_is_additive_and_creates_coherent_backup() {
    let f = Fixture::new();
    let mut expected = f.ws();
    expected.study = StudyState::default();
    let c = Connection::open(&f.db).unwrap();
    c.pragma_update(None, "foreign_keys", "OFF").unwrap();
    for table in [
        "presentation_preferences",
        "study_reflections",
        "study_burdens",
        "study_exposures",
        "opportunity_outcomes",
        "study_assignments",
        "work_opportunities",
        "study_protocol_events",
        "study_manifests",
        "schedule_epochs",
    ] {
        c.execute(&format!("DROP TABLE {table}"), []).unwrap();
    }
    c.pragma_update(None, "user_version", 3).unwrap();
    drop(c);
    let reopened = WorkspaceService::open(&f.db, &f.backups).unwrap();
    let w = reopened.load_workspace().unwrap();
    assert_eq!(w, expected);
    let files: Vec<_> = std::fs::read_dir(&f.backups)
        .unwrap()
        .map(|p| p.unwrap().path())
        .collect();
    assert_eq!(files.len(), 1);
    let backup = Connection::open(&files[0]).unwrap();
    assert_eq!(
        backup
            .pragma_query_value(None, "user_version", |r| r.get::<_, u32>(0))
            .unwrap(),
        3
    );
    assert_eq!(
        backup
            .query_row("SELECT count(*) FROM objectives", [], |r| r
                .get::<_, u32>(0))
            .unwrap(),
        1
    );
}
#[test]
fn strict_study_inputs_reject_unknown_fields_missing_nullable_and_invalid_numbers() {
    let f = Fixture::new();
    let input = f.plan_input(300, None);
    let original = serde_json::to_value(input).unwrap();
    for mutation in [0, 1, 2] {
        let mut v = original.clone();
        match mutation {
            0 => {
                v["condition"] = json!("d_plus_b");
            }
            1 => {
                v.as_object_mut().unwrap().remove("deadline_at");
            }
            _ => v["available_minutes"] = json!(-1),
        }
        assert!(serde_json::from_value::<PlanOpportunityInput>(v).is_err());
    }
}

#[test]
fn construction_preference_survives_restart_restore_and_noop_keeps_generation() {
    let mut f = Fixture::new();
    let w = f
        .service
        .set_construction_enabled(SetConstructionEnabledInput {
            expected_generation: f.ws().generation,
            enabled: false,
        })
        .unwrap();
    assert!(!presentation(&w.study, &study_now()).enabled);
    assert!(!presentation(&w.study, &study_now()).world_allowed);
    let unchanged = f
        .service
        .set_construction_enabled(SetConstructionEnabledInput {
            expected_generation: w.generation.clone(),
            enabled: false,
        })
        .unwrap();
    assert_eq!(unchanged, w);
    let reopened = WorkspaceService::open(&f.db, &f.backups)
        .unwrap()
        .load_workspace()
        .unwrap();
    assert_eq!(reopened.study.preferences, w.study.preferences);
    let bytes = f.service.export_bytes().unwrap();
    let mut other = Fixture::new();
    let restored = other.restore(&bytes);
    assert_eq!(restored.study.preferences, w.study.preferences);
}

#[test]
fn malformed_exposure_timestamp_is_rejected_without_panicking_or_changing_state() {
    let mut f = Fixture::new();
    let m = f.protocol(Phase::E01);
    for n in 0..4 {
        f.plan(300 + n * 1200, Some(&m));
    }
    let w = f.ws();
    let a = w
        .study
        .assignments
        .iter()
        .find(|a| a.condition == Condition::DPlusB)
        .unwrap();
    let o = w
        .study
        .opportunities
        .iter()
        .find(|o| o.id == a.opportunity_id)
        .unwrap();
    TEST_CLOCK.with(|c| {
        *c.borrow_mut() = Some(
            (stamp(&o.cue_at).unwrap() - chrono::Duration::seconds(60))
                .to_rfc3339_opts(SecondsFormat::Millis, true),
        )
    });
    f.service
        .record_study_exposure(RecordStudyExposureInput {
            expected_generation: w.generation,
            opportunity_id: Some(o.id.clone()),
            kind: ExposureKind::PreviewDelivered,
            note: None,
        })
        .unwrap();
    let before = f.ws();
    let mut value: Value = serde_json::from_slice(&f.service.export_bytes().unwrap()).unwrap();
    value["study"]["exposures"][0]["created_at"] = json!("zzz");
    assert!(
        f.service
            .preview_import(
                &serde_json::to_vec(&value).unwrap(),
                "bad-exposure.json".into()
            )
            .is_err()
    );
    assert_eq!(f.ws(), before);
}

#[test]
fn disabling_or_pausing_at_the_same_millisecond_does_not_erase_delivered_history() {
    let mut f = Fixture::new();
    let m = f.protocol(Phase::E01);
    for n in 0..4 {
        f.plan(300 + n * 1200, Some(&m));
    }
    let w = f.ws();
    let a = w
        .study
        .assignments
        .iter()
        .find(|a| a.condition == Condition::DPlusB)
        .unwrap();
    let o = w
        .study
        .opportunities
        .iter()
        .find(|o| o.id == a.opportunity_id)
        .unwrap();
    TEST_CLOCK.with(|c| {
        *c.borrow_mut() = Some(
            (stamp(&o.cue_at).unwrap() - chrono::Duration::seconds(60))
                .to_rfc3339_opts(SecondsFormat::Millis, true),
        )
    });
    f.service
        .record_study_exposure(RecordStudyExposureInput {
            expected_generation: w.generation,
            opportunity_id: Some(o.id.clone()),
            kind: ExposureKind::PreviewDelivered,
            note: None,
        })
        .unwrap();
    f.service
        .set_construction_enabled(SetConstructionEnabledInput {
            expected_generation: f.ws().generation,
            enabled: false,
        })
        .unwrap();
    f.transition(&m, StudyStatus::Paused);
    let w = f.ws();
    assert_eq!(w.study.exposures.len(), 1);
    assert!(!presentation(&w.study, &study_now()).enabled);
    assert!(f.service.export_bytes().is_ok());
}

#[test]
fn historical_overlapping_active_protocols_are_rejected_even_when_both_now_paused() {
    let mut f = Fixture::new();
    let m = f.protocol(Phase::E01);
    clock(1500);
    f.transition(&m, StudyStatus::Paused);
    let before = f.ws();
    let mut r: RecordSet = serde_json::from_slice(&f.service.export_bytes().unwrap()).unwrap();
    let s = r.study.as_mut().unwrap();
    let mut cloned = s.manifests[0].clone();
    cloned.id = new_id();
    let id = cloned.id.clone();
    s.manifests.push(cloned);
    for (sequence, status, at) in [
        (1, StudyStatus::Ready, 100),
        (2, StudyStatus::Active, 200),
        (3, StudyStatus::Paused, 1400),
    ] {
        s.protocol_events.push(ProtocolEvent {
            id: new_id(),
            manifest_id: id.clone(),
            status,
            sequence,
            reason: "Synthetic overlap".into(),
            created_at: time(at),
            acknowledgements: if status == StudyStatus::Active {
                ACKS.iter().map(|s| s.to_string()).collect()
            } else {
                vec![]
            },
        });
    }
    assert!(
        f.service
            .preview_import(&serde_json::to_vec(&r).unwrap(), "overlap.json".into())
            .is_err()
    );
    assert_eq!(f.ws(), before);
}

#[test]
fn nonchronological_assignment_positions_and_corrections_are_rejected() {
    let mut f = Fixture::new();
    let m = f.protocol(Phase::E01);
    let a = f.plan(300, Some(&m));
    clock(1);
    let b = f.plan(1500, Some(&m));
    clock(1300);
    f.report(&a, Report::Nonstart, None);
    clock(1400);
    let mut input = f.input(&a, Report::Started, Some(400));
    input.correction_reason = Some("Synthetic correction".into());
    f.service.record_opportunity_outcome(input).unwrap();
    let original: RecordSet = serde_json::from_slice(&f.service.export_bytes().unwrap()).unwrap();
    let before = f.ws();
    let mut r = original.clone();
    let s = r.study.as_mut().unwrap();
    for assignment in &mut s.assignments {
        if assignment.opportunity_id == a {
            assignment.opportunity_id = b.clone();
            assignment.assigned_at = time(1);
        } else {
            assignment.opportunity_id = a.clone();
            assignment.assigned_at = time(0);
        }
    }
    assert!(
        f.service
            .preview_import(&serde_json::to_vec(&r).unwrap(), "positions.json".into())
            .is_err()
    );
    let mut r = original;
    let s = r.study.as_mut().unwrap();
    s.outcomes
        .iter_mut()
        .find(|e| e.sequence == 2)
        .unwrap()
        .observed_at = time(1250);
    assert!(
        f.service
            .preview_import(
                &serde_json::to_vec(&r).unwrap(),
                "correction-time.json".into()
            )
            .is_err()
    );
    assert_eq!(f.ws(), before);
}

#[test]
fn approximate_inside_window_has_no_fabricated_exact_latency_and_saturation_prefix_is_checked() {
    let mut f = Fixture::new();
    let m = f.protocol(Phase::E01);
    let id = f.plan(300, Some(&m));
    clock(1300);
    let mut input = f.input(&id, Report::Started, Some(600));
    input.precision = Precision::Approximate;
    input.uncertainty_seconds = Some(60);
    let w = f.service.record_opportunity_outcome(input).unwrap();
    assert_eq!(
        w.study.latest_outcome(&id).unwrap().state,
        OutcomeState::ObservedStart
    );
    assert_eq!(w.study.latest_outcome(&id).unwrap().latency_seconds, None);
    let mut r: RecordSet = serde_json::from_slice(&f.service.export_bytes().unwrap()).unwrap();
    r.study.as_mut().unwrap().assignments[0].ledger_sequence = 1;
    assert!(
        f.service
            .preview_import(&serde_json::to_vec(&r).unwrap(), "bad-prefix.json".into())
            .is_err()
    );
}

#[test]
fn retried_delivery_does_not_duplicate_exposure_or_claim_seen() {
    let mut f = Fixture::new();
    let id = f.plan(300, None);
    clock(300);
    let input = RecordStudyExposureInput {
        expected_generation: f.ws().generation,
        opportunity_id: Some(id),
        kind: ExposureKind::CueDelivered,
        note: None,
    };
    let w = f.service.record_study_exposure(input.clone()).unwrap();
    let mut retry = input;
    retry.expected_generation = w.generation.clone();
    let replay = f.service.record_study_exposure(retry).unwrap();
    assert_eq!(replay, w);
    assert_eq!(w.study.exposures.len(), 1);
    assert_eq!(w.study.exposures[0].kind, ExposureKind::CueDelivered);
    assert!(
        !w.study
            .exposures
            .iter()
            .any(|e| e.kind == ExposureKind::Seen)
    );
}
