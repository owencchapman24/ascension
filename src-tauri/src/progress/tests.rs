use super::*;
use crate::domain::*;
use crate::service::WorkspaceService;
use serde_json::{Value, json};
use std::path::PathBuf;
use std::sync::{Arc, Barrier};
use std::time::{Duration, Instant};

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
        let backups = directory.path().join("recovery");
        let mut service = WorkspaceService::open(&database, &backups).unwrap();
        let ws = service.load_workspace().unwrap();
        let ws = service
            .create_project(CreateProjectInput {
                expected_generation: ws.generation,
                title: "Synthetic experiments".into(),
            })
            .unwrap();
        service.create_objective(CreateObjectiveInput { expected_generation:ws.generation,project_id:ws.projects[0].id.clone(),title:"Investigate a bounded hypothesis".into(),criterion:"Document the method, observations, and conclusion, including a negative finding.".into() }).unwrap();
        Self {
            _directory: directory,
            database,
            backups,
            service,
        }
    }
    fn ws(&self) -> Workspace {
        self.service.load_workspace().unwrap()
    }
    fn criterion(&self) -> Criterion {
        let ws = self.ws();
        let parent = &ws.progress.parents[0];
        current(&ws.progress, &parent.parent_id, &parent.primary_lineage_id)
            .unwrap()
            .clone()
    }
    fn save_input(&self) -> SaveCriterionInput {
        let c = self.criterion();
        let ws = self.ws();
        let slot = ws
            .progress
            .slots
            .iter()
            .find(|s| s.id == c.reward_slot_id)
            .unwrap();
        SaveCriterionInput {
            expected_generation: ws.generation,
            parent_id: c.parent_id,
            criterion_lineage_id: Some(c.criterion_lineage_id),
            description: c.description,
            allocation_bc: slot.allocation_bc,
            evidence_policy: c.evidence_policy,
            scope: c.scope,
            artifact_id: c.artifact_id,
            artifact_version: c.artifact_version,
            requires_independent_work: c.requires_independent_work,
            required_validators: c.required_validators,
        }
    }
    fn lock(&mut self) -> Criterion {
        let c = self.criterion();
        self.lock_other(&c);
        self.criterion()
    }
    fn lock_other(&mut self, c: &Criterion) {
        self.service
            .lock_criterion(LockCriterionInput {
                expected_generation: self.ws().generation,
                parent_id: c.parent_id.clone(),
                criterion_lineage_id: c.criterion_lineage_id.clone(),
                criterion_version: c.criterion_version,
            })
            .unwrap();
    }
    fn evidence_input(&self, c: &Criterion) -> RecordEvidenceInput {
        RecordEvidenceInput {
            expected_generation: self.ws().generation,
            parent_id: c.parent_id.clone(),
            criterion_lineage_id: c.criterion_lineage_id.clone(),
            criterion_version: c.criterion_version,
            kind: EvidenceKind::UserAttestation,
            description:
                "The synthetic hypothesis did not hold; the method and observations are documented."
                    .into(),
            reference: None,
            observed_at: None,
            status: EvidenceStatus::Supports,
            assistance: Assistance::None,
            uncertainty: None,
            supersedes: None,
        }
    }
    fn evidence(&mut self, c: &Criterion) -> String {
        let input = self.evidence_input(c);
        self.record(input)
    }
    fn record(&mut self, input: RecordEvidenceInput) -> String {
        let before = self
            .ws()
            .progress
            .evidence
            .iter()
            .map(|e| e.id.clone())
            .collect::<HashSet<_>>();
        let ws = self.service.record_evidence(input).unwrap();
        ws.progress
            .evidence
            .iter()
            .find(|e| !before.contains(&e.id))
            .unwrap()
            .id
            .clone()
    }
    fn review_input(&self, c: &Criterion, ids: Vec<String>) -> ReviewProgressInput {
        ReviewProgressInput {
            expected_generation: self.ws().generation,
            parent_id: c.parent_id.clone(),
            criterion_lineage_id: c.criterion_lineage_id.clone(),
            criterion_version: c.criterion_version,
            evidence_refs: ids,
            factual_summary:
                "The declared investigation was completed and its negative result documented."
                    .into(),
            assistance: Assistance::None,
        }
    }
    fn review(&mut self, c: &Criterion, ids: Vec<String>) -> ReviewSnapshot {
        let input = self.review_input(c, ids);
        self.service.review_progress(input).unwrap().snapshot
    }
    fn accept_input(&self, s: &ReviewSnapshot) -> AcceptProgressInput {
        let ws = self.ws();
        AcceptProgressInput {
            schema_version: 1,
            command_id: new_id(),
            expected_generation: ws.generation,
            parent_id: s.parent_id.clone(),
            criterion_lineage_id: s.criterion_lineage_id.clone(),
            criterion_version: s.criterion_version,
            expected_parent_revision: ws
                .objectives
                .iter()
                .find(|o| o.id == s.parent_id)
                .unwrap()
                .revision,
            reviewed_snapshot_id: s.id.clone(),
            reviewed_snapshot_digest: s.digest.clone(),
            evidence_refs: s.evidence_refs.clone(),
            factual_summary: s.factual_summary.clone(),
            assistance: s.assistance.clone(),
        }
    }
    fn accepted(&mut self) -> (AcceptProgressInput, SettlementResult) {
        let c = self.lock();
        let e = self.evidence(&c);
        let s = self.review(&c, vec![e]);
        assert!(s.eligible);
        let input = self.accept_input(&s);
        let result = self.service.accept_progress(input.clone()).unwrap().result;
        (input, result)
    }
    fn correct(&mut self, id: &str, kind: CorrectionKind) {
        self.service
            .correct_acceptance(CorrectAcceptanceInput {
                expected_generation: self.ws().generation,
                acceptance_id: id.into(),
                kind,
                reason: "Synthetic correction discovered during review.".into(),
            })
            .unwrap();
    }
    fn reopen(&mut self) {
        self.service = WorkspaceService::open(&self.database, &self.backups).unwrap();
    }
}

#[test]
fn prospective_lock_never_rewards_drafts_or_historical_activity() {
    let mut f = Fixture::new();
    let draft = f.criterion();
    let old = f.evidence(&draft);
    assert!(
        f.service
            .review_progress(f.review_input(&draft, vec![old.clone()]))
            .is_err()
    );
    let locked = f.lock();
    let review = f.review(&locked, vec![old]);
    assert!(!review.eligible);
    assert!(!review.missing.is_empty());
    let before = f.ws();
    assert_eq!(
        f.service
            .accept_progress(f.accept_input(&review))
            .unwrap_err()
            .code,
        "evidence_ineligible"
    );
    assert_eq!(f.ws(), before);
    let mut historical = f.evidence_input(&locked);
    historical.observed_at = Some("2020-01-01T00:00:00.000Z".into());
    let old = f.record(historical);
    assert!(!f.review(&locked, vec![old]).eligible);
    let fresh = f.evidence(&locked);
    let review = f.review(&locked, vec![fresh]);
    assert!(review.eligible);
    assert_eq!(
        f.service
            .accept_progress(f.accept_input(&review))
            .unwrap()
            .result
            .issued_bc,
        4
    );
}

#[test]
fn committed_retry_precedes_staleness_survives_restart_and_changed_payload_conflicts() {
    let mut f = Fixture::new();
    let (input, result) = f.accepted();
    let ws = f.ws();
    f.service
        .update_project(UpdateProjectInput {
            expected_generation: ws.generation,
            id: ws.projects[0].id.clone(),
            title: "Renamed synthetic project".into(),
        })
        .unwrap();
    f.reopen();
    let before = f.ws();
    assert_eq!(
        f.service.accept_progress(input.clone()).unwrap().result,
        result
    );
    assert_eq!(f.ws(), before);
    let mut changed = input.clone();
    changed.factual_summary = "A different claim must not replay.".into();
    assert_eq!(
        f.service.accept_progress(changed).unwrap_err().code,
        "idempotency_conflict"
    );
    let mut new_command = input;
    new_command.command_id = new_id();
    new_command.expected_generation = f.ws().generation;
    let repeated = f.service.accept_progress(new_command).unwrap().result;
    assert_eq!(repeated.issued_bc, 0);
    assert_eq!(repeated.balance_bc, 4);
    assert_eq!(f.ws().progress.grants.len(), 1);
    assert_eq!(f.ws().progress.acceptances.len(), 1);
}

#[test]
fn criterion_versions_preserve_slot_and_require_prospective_work_and_explicit_reopening() {
    let mut f = Fixture::new();
    let (_, result) = f.accepted();
    let old = f.criterion();
    let mut edit = f.save_input();
    edit.description = "Document a second prospective investigation, including limitations.".into();
    edit.artifact_id = Some("synthetic-report".into());
    edit.artifact_version = Some("2".into());
    f.service.save_criterion(edit).unwrap();
    let new = f.lock();
    assert_eq!(new.criterion_version, 2);
    assert_eq!(new.reward_slot_id, old.reward_slot_id);
    assert_eq!(
        f.ws()
            .progress
            .criteria
            .iter()
            .find(|c| c.id == old.id)
            .unwrap(),
        &old
    );
    let old_evidence = f.ws().progress.evidence[0].id.clone();
    assert!(
        f.service
            .review_progress(f.review_input(&new, vec![old_evidence]))
            .is_err()
    );
    let e = f.evidence(&new);
    let s = f.review(&new, vec![e.clone()]);
    assert!(f.service.accept_progress(f.accept_input(&s)).is_err());
    f.correct(&result.acceptance_id, CorrectionKind::HistoricalReopened);
    let s = f.review(&new, vec![e]);
    let result = f
        .service
        .accept_progress(f.accept_input(&s))
        .unwrap()
        .result;
    assert_eq!(result.issued_bc, 0);
    assert_eq!(result.balance_bc, 4);
    assert!(
        f.ws()
            .progress
            .acceptances
            .iter()
            .any(|a| a.status == AcceptanceStatus::Historical && a.criterion_version == 1)
    );
}

#[test]
fn ordinary_edits_preserve_all_dependent_history_and_locked_text_is_not_editable_in_place() {
    let mut f = Fixture::new();
    f.accepted();
    let original = f.ws();
    let parent = original.objectives[0].clone();
    let before_progress = original.progress.clone();
    f.service
        .update_objective(UpdateObjectiveInput {
            expected_generation: original.generation,
            id: parent.id.clone(),
            title: "A clearer bounded title".into(),
            criterion: parent.criterion.clone(),
        })
        .unwrap();
    for index in 0..12 {
        f.service
            .save_work(SaveWorkInput {
                expected_generation: f.ws().generation,
                objective_id: parent.id.clone(),
                description: format!("Synthetic next action {index}"),
                stop_condition: "One specified observation is recorded.".into(),
                note: if index % 2 == 0 {
                    Some("Resume from the current observation.".into())
                } else {
                    None
                },
            })
            .unwrap();
    }
    f.service
        .select_objective(SelectObjectiveInput {
            expected_generation: f.ws().generation,
            id: None,
        })
        .unwrap();
    assert_eq!(f.ws().progress, before_progress);
    assert_eq!(f.ws().actions.len(), 1);
    let before = f.ws();
    assert!(
        f.service
            .update_objective(UpdateObjectiveInput {
                expected_generation: before.generation.clone(),
                id: parent.id,
                title: "Attempted change".into(),
                criterion: "A lower retrospective standard.".into()
            })
            .is_err()
    );
    assert_eq!(f.ws(), before);
    f.reopen();
    assert_eq!(f.ws(), before);
}

#[test]
fn split_allocations_are_bounded_for_every_integer_partition_and_cannot_be_renewed() {
    for first in 1..=3 {
        let mut f = Fixture::new();
        let mut input = f.save_input();
        input.allocation_bc = first;
        f.service.save_criterion(input).unwrap();
        let mut second = f.save_input();
        second.criterion_lineage_id = None;
        second.allocation_bc = 4 - first;
        second.description = "Independently document the bounded validation result.".into();
        f.service.save_criterion(second).unwrap();
        let criteria = f.ws().progress.criteria.clone();
        for c in criteria {
            f.lock_other(&c);
            let c = f
                .ws()
                .progress
                .criteria
                .iter()
                .find(|v| v.id == c.id)
                .unwrap()
                .clone();
            let e = f.evidence(&c);
            let s = f.review(&c, vec![e]);
            f.service.accept_progress(f.accept_input(&s)).unwrap();
        }
        let before = f.ws();
        assert_eq!(before.progress.wallet_bc, 4);
        assert_eq!(before.progress.grants.len(), 2);
        let mut excess = f.save_input();
        excess.criterion_lineage_id = None;
        excess.allocation_bc = 1;
        assert!(f.service.save_criterion(excess).is_err());
        let mut changed = f.save_input();
        changed.allocation_bc = 4;
        assert!(f.service.save_criterion(changed).is_err());
        assert_eq!(f.ws(), before);
    }
}

#[test]
fn unknown_duplicate_and_cross_domain_evidence_references_leave_no_mutation() {
    let mut f = Fixture::new();
    let c = f.lock();
    let e = f.evidence(&c);
    for ids in [vec![new_id()], vec![e.clone(), e.clone()]] {
        let before = f.ws();
        assert!(f.service.review_progress(f.review_input(&c, ids)).is_err());
        assert_eq!(f.ws(), before);
    }
    let ws = f.ws();
    let ws = f
        .service
        .create_objective(CreateObjectiveInput {
            expected_generation: ws.generation,
            project_id: ws.projects[0].id.clone(),
            title: "Practice one physical movement".into(),
            criterion: "Complete the declared physical movement independently.".into(),
        })
        .unwrap();
    let other = ws
        .progress
        .criteria
        .iter()
        .find(|v| v.parent_id != c.parent_id)
        .unwrap()
        .clone();
    f.lock_other(&other);
    let before = f.ws();
    assert!(
        f.service
            .review_progress(f.review_input(&other, vec![e]))
            .is_err()
    );
    assert_eq!(f.ws(), before);
}

#[test]
fn referenced_evidence_and_unavailable_validation_keep_honest_source_authority() {
    let mut f = Fixture::new();
    let mut edit = f.save_input();
    edit.evidence_policy = EvidencePolicy::ReferencedEvidence;
    f.service.save_criterion(edit).unwrap();
    let c = f.lock();
    let mut input = f.evidence_input(&c);
    input.kind = EvidenceKind::ImportedMetadata;
    input.reference = Some("synthetic-report:v1".into());
    let metadata = f.record(input);
    assert!(!f.review(&c, vec![metadata]).eligible);
    let mut input = f.evidence_input(&c);
    input.kind = EvidenceKind::ArtifactReference;
    input.reference = Some("synthetic-report:v1".into());
    let artifact = f.record(input);
    assert!(f.review(&c, vec![artifact]).eligible);
    let mut edit = f.save_input();
    edit.evidence_policy = EvidencePolicy::TrustedValidation;
    edit.required_validators = vec![new_id()];
    f.service.save_criterion(edit).unwrap();
    let c = f.lock();
    let input = f.evidence_input(&c);
    let e = f.record(input);
    let snapshot = f.review(&c, vec![e]);
    assert!(!snapshot.eligible);
    assert!(!snapshot.missing.is_empty());
    let mut forged = f.evidence_input(&c);
    forged.kind = EvidenceKind::RegisteredValidator;
    let before = f.ws();
    assert_eq!(
        f.service.record_evidence(forged).unwrap_err().code,
        "unavailable_authority"
    );
    assert_eq!(f.ws(), before);
}

#[test]
fn generated_or_assisted_work_cannot_establish_independent_human_practice() {
    for assistance in [Assistance::Assisted, Assistance::Generated] {
        let mut f = Fixture::new();
        let mut edit = f.save_input();
        edit.requires_independent_work = true;
        f.service.save_criterion(edit).unwrap();
        let c = f.lock();
        let mut input = f.evidence_input(&c);
        input.assistance = assistance.clone();
        let e = f.record(input);
        let s = f.review(&c, vec![e]);
        assert!(!s.eligible);
        assert!(!s.conflicts.is_empty());
        assert!(f.service.accept_progress(f.accept_input(&s)).is_err());
        assert_eq!(f.ws().progress.wallet_bc, 0);
    }
}

#[test]
fn omitted_generated_evidence_still_conflicts_with_independent_practice() {
    let mut f = Fixture::new();
    let mut edit = f.save_input();
    edit.requires_independent_work = true;
    f.service.save_criterion(edit).unwrap();
    let c = f.lock();
    let personal = f.evidence(&c);
    let mut generated = f.evidence_input(&c);
    generated.assistance = Assistance::Generated;
    f.record(generated);
    let snapshot = f.review(&c, vec![personal]);
    assert!(!snapshot.eligible);
    assert!(!snapshot.conflicts.is_empty());
    assert!(
        f.service
            .accept_progress(f.accept_input(&snapshot))
            .is_err()
    );
}

#[test]
fn contradictory_registry_evidence_cannot_be_hidden_and_supersession_is_explicit() {
    let mut f = Fixture::new();
    let c = f.lock();
    let positive = f.evidence(&c);
    let mut contradictory = f.evidence_input(&c);
    contradictory.status = EvidenceStatus::Contradicts;
    let negative = f.record(contradictory);
    let s = f.review(&c, vec![positive.clone()]);
    assert_eq!(s.applicable_evidence.len(), 2);
    assert!(!s.eligible);
    assert!(!s.conflicts.is_empty());
    let before = f.ws();
    assert!(f.service.accept_progress(f.accept_input(&s)).is_err());
    assert_eq!(f.ws(), before);
    let mut correction = f.evidence_input(&c);
    correction.supersedes = Some(negative.clone());
    let corrected = f.record(correction);
    assert!(
        f.service
            .review_progress(f.review_input(&c, vec![negative]))
            .is_err()
    );
    let s = f.review(&c, vec![positive, corrected]);
    assert!(s.eligible);
    assert_eq!(
        f.service
            .accept_progress(f.accept_input(&s))
            .unwrap()
            .result
            .issued_bc,
        4
    );
}

#[test]
fn changed_registry_and_criterion_invalidate_stale_snapshots_even_with_current_generation() {
    let mut f = Fixture::new();
    let c = f.lock();
    let e = f.evidence(&c);
    let snapshot = f.review(&c, vec![e]);
    let mut input = f.accept_input(&snapshot);
    f.evidence(&c);
    input.expected_generation = f.ws().generation;
    let before = f.ws();
    assert_eq!(
        f.service.accept_progress(input.clone()).unwrap_err().code,
        "stale_review"
    );
    assert_eq!(f.ws(), before);
    let mut edit = f.save_input();
    edit.description = "A prospective revised investigation standard.".into();
    f.service.save_criterion(edit).unwrap();
    input.expected_generation = f.ws().generation;
    input.expected_parent_revision = f.ws().objectives[0].revision;
    assert_eq!(
        f.service.accept_progress(input).unwrap_err().code,
        "stale_review"
    );
    assert_eq!(f.ws().progress.wallet_bc, 0);
}

#[test]
fn corrections_require_fresh_review_retain_courtesy_credit_and_reacceptance_never_remints() {
    let mut f = Fixture::new();
    let (original, result) = f.accepted();
    f.correct(&result.acceptance_id, CorrectionKind::IncorrectAcceptance);
    let ws = f.ws();
    assert_eq!(ws.progress.wallet_bc, 4);
    assert_eq!(
        ws.progress.acceptances[0].status,
        AcceptanceStatus::Retracted
    );
    assert_eq!(ws.progress.corrections.len(), 1);
    assert_eq!(
        f.service.accept_progress(original.clone()).unwrap().result,
        result
    );
    let mut resurrection = original;
    resurrection.command_id = new_id();
    resurrection.expected_generation = f.ws().generation;
    let before = f.ws();
    assert!(f.service.accept_progress(resurrection).is_err());
    assert_eq!(f.ws(), before);
    let c = f.criterion();
    let e = f.evidence(&c);
    let s = f.review(&c, vec![e]);
    let result = f
        .service
        .accept_progress(f.accept_input(&s))
        .unwrap()
        .result;
    assert_eq!(result.issued_bc, 0);
    assert!(result.retained_reward);
    assert_eq!(f.ws().progress.acceptances.len(), 2);
    assert_eq!(f.ws().progress.ledger.len(), 1);
    for _ in 0..3 {
        f.reopen();
        assert_eq!(f.ws().progress.wallet_bc, 4);
        assert_eq!(f.ws().progress.grants.len(), 1);
    }
}

#[test]
fn correction_invalidates_all_preexisting_pending_reviews_and_import_preserves_causal_history() {
    for kind in [
        CorrectionKind::IncorrectAcceptance,
        CorrectionKind::HistoricalReopened,
    ] {
        let mut f = Fixture::new();
        let criterion = f.lock();
        let evidence = f.evidence(&criterion);
        let first = f.review(&criterion, vec![evidence.clone()]);
        let spare = f.review(&criterion, vec![evidence.clone()]);
        let original_command = f.accept_input(&first);
        let original = f
            .service
            .accept_progress(original_command.clone())
            .unwrap()
            .result;
        f.correct(&original.acceptance_id, kind);
        let corrected = f.ws();
        let correction_revision = corrected.progress.corrections[0].registry_revision;
        assert!(correction_revision > spare.registry_revision);
        let stale_command = f.accept_input(&spare);
        assert_eq!(
            f.service.accept_progress(stale_command).unwrap_err().code,
            "stale_review"
        );
        assert_eq!(f.ws(), corrected);
        assert_eq!(
            f.service.accept_progress(original_command).unwrap().result,
            original
        );

        // A fresh explicit review may use unchanged evidence; the reward remains consumed.
        let fresh = f.review(&criterion, vec![evidence]);
        assert_eq!(fresh.registry_revision, correction_revision);
        let reaccepted = f
            .service
            .accept_progress(f.accept_input(&fresh))
            .unwrap()
            .result;
        assert_eq!(reaccepted.issued_bc, 0);
        assert_eq!(reaccepted.balance_bc, 4);
        let valid = f.ws();
        let portable = f.service.export_bytes().unwrap();
        let mut target = Fixture::new();
        let preview = target
            .service
            .preview_import(&portable, "synthetic-corrected.json".into())
            .unwrap();
        let restored = target
            .service
            .restore_import(RestoreInput {
                token: preview.token,
                expected_generation: preview.generation,
            })
            .unwrap()
            .workspace;
        assert_eq!(restored.progress, valid.progress);

        // Recompute the command hash and preserve every ordinary relation while
        // substituting the second pre-correction review. Only causal validation
        // should prevent this superficially consistent imported resurrection.
        let mut forged = valid.clone();
        forged
            .progress
            .acceptances
            .iter_mut()
            .find(|a| a.id == reaccepted.acceptance_id)
            .unwrap()
            .snapshot_id = spare.id.clone();
        forged
            .progress
            .snapshots
            .iter_mut()
            .find(|s| s.id == spare.id)
            .unwrap()
            .disposition = ReviewDisposition::Accepted;
        forged
            .progress
            .snapshots
            .iter_mut()
            .find(|s| s.id == fresh.id)
            .unwrap()
            .disposition = ReviewDisposition::Pending;
        let command = forged
            .progress
            .commands
            .iter_mut()
            .find(|c| c.result.acceptance_id == reaccepted.acceptance_id)
            .unwrap();
        command.payload.reviewed_snapshot_id = spare.id.clone();
        command.payload.reviewed_snapshot_digest = spare.digest.clone();
        command.payload_digest = hash(&command.payload).unwrap();
        let bytes = serde_json::to_vec(&RecordSet::from(&forged)).unwrap();
        let before = target.ws();
        let error = target
            .service
            .preview_import(&bytes, "synthetic-pre-correction-review.json".into())
            .unwrap_err();
        assert_eq!(error.code, "invalid_import");
        assert!(error.message.contains("before its preceding correction"));
        assert_eq!(target.ws(), before);
    }
}

#[test]
fn imported_accepted_snapshot_cannot_outlive_stripped_acceptance_and_reward_history() {
    let mut f = Fixture::new();
    f.accepted();
    let before = f.ws();
    let mut records = RecordSet::from(&before);
    let state = records.progress.as_mut().unwrap();
    state.acceptances.clear();
    state.corrections.clear();
    state.grants.clear();
    state.ledger.clear();
    state.commands.clear();
    state.wallet_bc = 0;
    state.audit.retain(|event| {
        !matches!(
            event.kind.as_str(),
            "accomplishment_accepted" | "accomplishment_corrected" | "reward_issued"
        )
    });
    assert_eq!(state.snapshots[0].disposition, ReviewDisposition::Accepted);
    let bytes = serde_json::to_vec(&records).unwrap();
    let error = f
        .service
        .preview_import(&bytes, "synthetic-stripped-history.json".into())
        .unwrap_err();
    assert_eq!(error.code, "invalid_import");
    assert!(error.message.contains("accepted review"));
    assert_eq!(f.ws(), before);
}

#[test]
fn imported_correction_cannot_borrow_a_postacceptance_evidence_revision() {
    let mut f = Fixture::new();
    let (_, accepted) = f.accepted();
    let criterion = f.criterion();
    let evidence_id = f.evidence(&criterion);
    f.correct(&accepted.acceptance_id, CorrectionKind::IncorrectAcceptance);
    let before = f.ws();
    let mut records = RecordSet::from(&before);
    let state = records.progress.as_mut().unwrap();
    let evidence_revision = state
        .evidence
        .iter()
        .find(|e| e.id == evidence_id)
        .unwrap()
        .registry_revision;
    assert!(evidence_revision > state.snapshots[0].registry_revision);
    state.corrections[0].registry_revision = evidence_revision;
    let bytes = serde_json::to_vec(&records).unwrap();
    let error = f
        .service
        .preview_import(&bytes, "synthetic-shared-authority-revision.json".into())
        .unwrap_err();
    assert_eq!(error.code, "invalid_import");
    assert!(
        error
            .message
            .contains("share an evidence-registry revision")
    );
    assert_eq!(f.ws(), before);
}

#[test]
fn concurrent_same_command_returns_original_result_and_distinct_commands_cannot_overissue() {
    for same_command in [true, false] {
        let mut f = Fixture::new();
        let c = f.lock();
        let e = f.evidence(&c);
        let s = f.review(&c, vec![e]);
        let input = f.accept_input(&s);
        let barrier = Arc::new(Barrier::new(2));
        let mut workers = Vec::new();
        for index in 0..2 {
            let database = f.database.clone();
            let backups = f.backups.clone();
            let barrier = barrier.clone();
            let mut command = input.clone();
            if !same_command && index == 1 {
                command.command_id = new_id();
            }
            workers.push(std::thread::spawn(move || {
                let mut service = WorkspaceService::open(&database, &backups).unwrap();
                barrier.wait();
                service.accept_progress(command)
            }));
        }
        let outcomes = workers
            .into_iter()
            .map(|w| w.join().unwrap())
            .collect::<Vec<_>>();
        assert_eq!(
            outcomes.iter().filter(|r| r.is_ok()).count(),
            if same_command { 2 } else { 1 }
        );
        if same_command {
            assert_eq!(
                outcomes[0].as_ref().unwrap().result,
                outcomes[1].as_ref().unwrap().result
            );
        } else {
            assert_eq!(
                outcomes.iter().find_map(|r| r.as_ref().err()).unwrap().code,
                "conflict"
            );
        }
        assert_eq!(f.ws().progress.wallet_bc, 4);
        assert_eq!(f.ws().progress.acceptances.len(), 1);
        assert_eq!(f.ws().progress.grants.len(), 1);
    }
}

#[test]
fn concurrent_changed_payload_same_command_id_conflicts_after_first_commit() {
    let mut f = Fixture::new();
    let c = f.lock();
    let e = f.evidence(&c);
    let s = f.review(&c, vec![e]);
    let input = f.accept_input(&s);
    // Both commands are individually meaningful candidates; the second changes
    // optimistic metadata only, but may never inherit the first committed result.
    let mut changed = input.clone();
    changed.expected_parent_revision += 1;
    let mut other = WorkspaceService::open(&f.database, &f.backups).unwrap();
    let result = f.service.accept_progress(input).unwrap().result;
    assert_eq!(
        other.accept_progress(changed).unwrap_err().code,
        "idempotency_conflict"
    );
    assert_eq!(result.issued_bc, 4);
    assert_eq!(f.ws().progress.wallet_bc, 4);
}

#[test]
fn injected_failures_roll_back_acceptance_grant_ledger_audit_and_idempotency_together() {
    for table in ["reward_grants", "ledger_entries", "committed_commands"] {
        let mut f = Fixture::new();
        let c = f.lock();
        let e = f.evidence(&c);
        let s = f.review(&c, vec![e]);
        let input = f.accept_input(&s);
        let before = f.ws();
        let db = Connection::open(&f.database).unwrap();
        db.execute_batch(&format!("CREATE TRIGGER synthetic_fault BEFORE INSERT ON {table} BEGIN SELECT RAISE(ABORT,'synthetic interruption'); END;")).unwrap();
        assert!(f.service.accept_progress(input.clone()).is_err());
        assert_eq!(f.ws(), before);
        f.reopen();
        assert_eq!(f.ws(), before);
        db.execute_batch("DROP TRIGGER synthetic_fault;").unwrap();
        let result = f.service.accept_progress(input.clone()).unwrap().result;
        assert_eq!(result.issued_bc, 4);
        f.reopen();
        assert_eq!(f.service.accept_progress(input).unwrap().result, result);
    }
}

#[test]
fn lock_contention_has_a_bounded_controlled_error_without_partial_issuance() {
    let mut f = Fixture::new();
    let c = f.lock();
    let e = f.evidence(&c);
    let s = f.review(&c, vec![e]);
    let input = f.accept_input(&s);
    let before = f.ws();
    let connection = Connection::open(&f.database).unwrap();
    connection.execute_batch("BEGIN IMMEDIATE;").unwrap();
    let started = Instant::now();
    let error = f.service.accept_progress(input).unwrap_err();
    assert_eq!(error.code, "busy");
    assert!(started.elapsed() < Duration::from_secs(8));
    connection.execute_batch("ROLLBACK;").unwrap();
    assert_eq!(f.ws(), before);
}

#[test]
fn strict_dtos_reject_malformed_numbers_unknown_privileges_and_schema_versions() {
    let mut f = Fixture::new();
    let (_, result) = f.accepted();
    assert_eq!(result.issued_bc, 4);
    let base = serde_json::to_value(f.save_input()).unwrap();
    for allocation in [
        json!("4"),
        json!(true),
        json!(1.5),
        json!(-1),
        json!(4294967296u64),
        Value::Null,
    ] {
        let mut malformed = base.clone();
        malformed["allocation_bc"] = allocation;
        assert!(serde_json::from_value::<SaveCriterionInput>(malformed).is_err());
    }
    let command = f.ws().progress.commands[0].payload.clone();
    let mut absent_nullable = base.clone();
    absent_nullable
        .as_object_mut()
        .unwrap()
        .remove("artifact_id");
    assert!(serde_json::from_value::<SaveCriterionInput>(absent_nullable).is_err());
    for field in [
        "award_amount",
        "actor",
        "validator_privileges",
        "reward_policy_override",
        "sql",
    ] {
        let mut malformed = serde_json::to_value(&command).unwrap();
        malformed[field] = json!(4);
        assert!(serde_json::from_value::<AcceptProgressInput>(malformed).is_err());
    }
    let mut unsupported = command;
    unsupported.schema_version = 2;
    assert_eq!(
        f.service.accept_progress(unsupported).unwrap_err().code,
        "unsupported_schema"
    );
}

#[test]
fn rejected_review_stays_unaccepted_and_cannot_be_settled() {
    let mut f = Fixture::new();
    let c = f.lock();
    let e = f.evidence(&c);
    let s = f.review(&c, vec![e]);
    f.service
        .reject_review(RejectReviewInput {
            expected_generation: f.ws().generation,
            snapshot_id: s.id.clone(),
            reason: "The report does not yet describe the limitations.".into(),
        })
        .unwrap();
    assert!(f.service.accept_progress(f.accept_input(&s)).is_err());
    assert_eq!(f.ws().progress.wallet_bc, 0);
    assert_eq!(
        f.ws().progress.snapshots[0].disposition,
        ReviewDisposition::Rejected
    );
}

fn register_fixture_validator(
    f: &mut Fixture,
    c: &Criterion,
    id: String,
    statuses: &[EvidenceStatus],
) -> Vec<String> {
    let connection = Connection::open(&f.database).unwrap();
    connection
        .execute_batch("PRAGMA foreign_keys=ON; BEGIN IMMEDIATE;")
        .unwrap();
    let validator = ValidatorDefinition {
        id: id.clone(),
        parent_id: c.parent_id.clone(),
        criterion_lineage_id: c.criterion_lineage_id.clone(),
        criterion_version: c.criterion_version,
        artifact_id: c.artifact_id.clone(),
        artifact_version: c.artifact_version.clone(),
        scope: c.scope.clone(),
        source_id: "synthetic-checker".into(),
        source_provenance: "synthetic_registered_fixture".into(),
        available: true,
        registry_revision: next_registry(&connection).unwrap(),
    };
    insert(&connection, "validator_definitions", &id, &validator).unwrap();
    let mut ids = Vec::new();
    for status in statuses {
        let evidence = Evidence {
            id: new_id(),
            parent_id: c.parent_id.clone(),
            criterion_lineage_id: c.criterion_lineage_id.clone(),
            criterion_version: c.criterion_version,
            criterion_id: c.id.clone(),
            kind: EvidenceKind::RegisteredValidator,
            source_id: validator.source_id.clone(),
            source_provenance: validator.source_provenance.clone(),
            artifact_id: c.artifact_id.clone(),
            artifact_version: c.artifact_version.clone(),
            scope: c.scope.clone(),
            observed_at: now(),
            recorded_at: now(),
            registry_revision: next_registry(&connection).unwrap(),
            description: "Synthetic independently registered result; never a production adapter."
                .into(),
            reference: None,
            status: status.clone(),
            assistance: Assistance::None,
            uncertainty: None,
            supersedes: None,
            validator_id: Some(id.clone()),
        };
        insert(&connection, "evidence_records", &evidence.id, &evidence).unwrap();
        ids.push(evidence.id);
    }
    changed(&connection).unwrap();
    connection.execute_batch("COMMIT;").unwrap();
    ids
}

#[test]
fn p5t001_002_registered_contradictory_validator_cannot_be_omitted_from_claimant_refs() {
    let mut f = Fixture::new();
    let validator_id = new_id();
    let mut edit = f.save_input();
    edit.evidence_policy = EvidencePolicy::TrustedValidation;
    edit.required_validators = vec![validator_id.clone()];
    f.service.save_criterion(edit).unwrap();
    let c = f.lock();
    let ids = register_fixture_validator(
        &mut f,
        &c,
        validator_id,
        &[EvidenceStatus::Supports, EvidenceStatus::Contradicts],
    );
    let snapshot = f.review(&c, vec![ids[0].clone()]);
    assert_eq!(snapshot.applicable_evidence.len(), 2);
    assert_eq!(snapshot.applicable_validators.len(), 1);
    assert!(!snapshot.eligible);
    assert!(!snapshot.conflicts.is_empty());
    let before = f.ws();
    assert!(
        f.service
            .accept_progress(f.accept_input(&snapshot))
            .is_err()
    );
    assert_eq!(f.ws(), before);
}

#[test]
fn narrow_registered_fixture_positive_path_is_available_only_inside_test_build() {
    let mut f = Fixture::new();
    let validator_id = new_id();
    let mut edit = f.save_input();
    edit.evidence_policy = EvidencePolicy::TrustedValidation;
    edit.required_validators = vec![validator_id.clone()];
    f.service.save_criterion(edit).unwrap();
    let c = f.lock();
    let ids = register_fixture_validator(&mut f, &c, validator_id, &[EvidenceStatus::Supports]);
    let snapshot = f.review(&c, ids);
    assert!(snapshot.eligible);
    assert_eq!(
        f.service
            .accept_progress(f.accept_input(&snapshot))
            .unwrap()
            .result
            .issued_bc,
        4
    );
}

#[test]
fn sqlite_constraints_independently_preserve_locked_definitions_entitlements_and_ledger() {
    let mut f = Fixture::new();
    f.accepted();
    let before = f.ws();
    let connection = Connection::open(&f.database).unwrap();
    connection.execute_batch("PRAGMA foreign_keys=ON;").unwrap();
    let criterion = &before.progress.criteria[0];
    let mut changed = criterion.clone();
    changed.description = "A retrospective lower standard.".into();
    assert!(update(&connection, "criterion_versions", &changed.id, &changed).is_err());
    let mut slot = before.progress.slots[0].clone();
    slot.allocation_bc = 1;
    assert!(update(&connection, "reward_slots", &slot.id, &slot).is_err());
    let mut extra = slot.clone();
    extra.id = new_id();
    extra.criterion_lineage_id = new_id();
    extra.allocation_locked = false;
    assert!(insert(&connection, "reward_slots", &extra.id, &extra).is_err());
    let mut parent = before.progress.parents[0].clone();
    parent.lifetime_budget_bc = 5;
    assert!(update(&connection, "progress_parents", &parent.parent_id, &parent).is_err());
    let mut grant = before.progress.grants[0].clone();
    grant.id = new_id();
    assert!(insert(&connection, "reward_grants", &grant.id, &grant).is_err());
    let mut acceptance = before.progress.acceptances[0].clone();
    acceptance.id = new_id();
    assert!(insert(&connection, "acceptances", &acceptance.id, &acceptance).is_err());
    let mut ledger = before.progress.ledger[0].clone();
    ledger.amount_bc = 3;
    assert!(update(&connection, "ledger_entries", &ledger.id, &ledger).is_err());
    let mut payload = serde_json::to_value(&before.progress.slots[0]).unwrap();
    payload["id"] = json!(new_id());
    payload["allocation_bc"] = json!(1.5);
    assert!(
        connection
            .execute(
                "INSERT INTO reward_slots(id,payload) VALUES(?1,?2)",
                params![
                    payload["id"].as_str().unwrap(),
                    serde_json::to_string(&payload).unwrap()
                ]
            )
            .is_err()
    );
    assert_eq!(f.ws(), before);
}
