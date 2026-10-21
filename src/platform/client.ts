import { invoke, isTauri } from "@tauri-apps/api/core";
import type { WorldCommandInput, WorldCommandResult, WorldPreview, WorldPreviewInput, WorldState, WorldView } from "../world/types";

export interface RecordMetadata { id: string; revision: number; created_at: string; updated_at: string }
export interface Project extends RecordMetadata { title: string }
export interface Objective extends RecordMetadata { project_id: string; title: string; criterion: string }
export interface NextAction extends RecordMetadata { objective_id: string; description: string; stop_condition: string }
export interface ResumptionNote extends RecordMetadata { objective_id: string; text: string }
export interface Workspace {
  generation: string;
  selected_objective_id: string | null;
  projects: Project[];
  objectives: Objective[];
  actions: NextAction[];
  notes: ResumptionNote[];
  progress: ProgressState;
  study: StudyView;
  world: WorldState;
  daily: DailyState;
}
export type ObjectiveStatus = "active" | "paused" | "archived";
export interface ObjectiveOrganization { objective_id: string; status: ObjectiveStatus; priority: number; revision: number; updated_at: string }
export interface WorkCheckpoint {
  id: string; command_id: string; sequence: number; objective_id: string; note: string; reference: string | null;
  provenance: "self_report"; recorded_at: string; description: string; stop_condition: string;
  action_id: string; action_revision: number; resumption_note: string | null; supersedes: string | null; correction_reason: string | null;
}
export interface DailyState { objectives: ObjectiveOrganization[]; checkpoints: WorkCheckpoint[]; commands: { id: string; digest: string; input: RecordCheckpointInput; checkpoint_id: string }[] }
export interface RecordCheckpointInput {
  expected_generation: string; command_id: string; objective_id: string; note: string; reference: string | null;
  description: string; stop_condition: string; resumption_note: string | null;
  supersedes: string | null; correction_reason: string | null;
}
export type EvidencePolicy = "user_attestation" | "referenced_evidence" | "trusted_validation";
export type Assistance = "none" | "assisted" | "generated";
export type EvidenceKind = "user_attestation" | "self_report" | "artifact_reference" | "imported_metadata" | "registered_validator" | "third_party_claim";
export type EvidenceStatus = "supports" | "contradicts" | "uncertain";
export type CorrectionKind = "incorrect_acceptance" | "historical_reopened";
export interface RewardParent { parent_id: string; primary_lineage_id: string; reward_policy_version: "bc-v1"; lifetime_budget_bc: number; provenance: string }
export interface RewardSlot { id: string; parent_id: string; criterion_lineage_id: string; allocation_bc: number; allocation_locked: boolean }
export interface Criterion {
  id: string; parent_id: string; criterion_lineage_id: string; criterion_version: number; reward_slot_id: string;
  description: string; state: "draft" | "locked"; evidence_policy: EvidencePolicy; scope: string;
  artifact_id: string | null; artifact_version: string | null; requires_independent_work: boolean; required_validators: string[];
  created_at: string; locked_at: string | null; lock_registry_revision: number | null;
  registry_revision: number;
}
export interface Evidence {
  id: string; parent_id: string; criterion_lineage_id: string; criterion_version: number; criterion_id: string;
  kind: EvidenceKind; source_id: string; source_provenance: string; artifact_id: string | null; artifact_version: string | null;
  scope: string; observed_at: string; recorded_at: string; registry_revision: number; description: string;
  reference: string | null; status: EvidenceStatus; assistance: Assistance; uncertainty: string | null; supersedes: string | null; validator_id: string | null;
}
export interface ValidatorDefinition { id: string; parent_id: string; criterion_lineage_id: string; criterion_version: number; artifact_id: string | null; artifact_version: string | null; scope: string; source_id: string; source_provenance: string; available: boolean; registry_revision: number }
export interface ReviewSnapshot {
  id: string; parent_id: string; criterion_lineage_id: string; criterion_version: number; criterion_id: string;
  registry_revision: number; criterion: Criterion; evidence_refs: string[]; applicable_evidence: Evidence[];
  applicable_validators: ValidatorDefinition[]; missing: string[]; conflicts: string[]; eligible: boolean;
  factual_summary: string; assistance: Assistance; digest: string; reviewed_at: string;
  disposition: "pending" | "accepted" | "rejected"; rejection_reason: string | null;
}
export interface Acceptance {
  id: string; parent_id: string; criterion_lineage_id: string; criterion_version: number; criterion_id: string; reward_slot_id: string;
  snapshot_id: string; actor: "local_user"; method: "user_confirmed"; assistance: Assistance; factual_summary: string;
  accepted_at: string; status: "accepted" | "retracted" | "historical"; correction_id: string | null;
}
export interface Correction { id: string; acceptance_id: string; kind: CorrectionKind; reason: string; created_at: string; registry_revision: number }
export interface RewardGrant { id: string; parent_id: string; reward_slot_id: string; criterion_lineage_id: string; acceptance_id: string; reward_policy_version: "bc-v1"; amount_bc: number; created_at: string }
export interface LedgerEntry { id: string; grant_id: string; sequence: number; amount_bc: number; created_at: string }
export interface AuditEvent { id: string; kind: "criterion_saved" | "criterion_locked" | "evidence_recorded" | "review_requested" | "review_rejected" | "accomplishment_accepted" | "accomplishment_corrected" | "reward_issued"; subject_id: string; actor: "local_user"; created_at: string }
export interface CommittedCommand { id: string; ledger_sequence: number; payload_digest: string; payload: AcceptProgressInput; result: SettlementResult }
export interface ProgressState {
  registry_revision: number; parents: RewardParent[]; slots: RewardSlot[]; criteria: Criterion[]; evidence: Evidence[];
  validators: ValidatorDefinition[]; snapshots: ReviewSnapshot[]; acceptances: Acceptance[]; corrections: Correction[];
  grants: RewardGrant[]; ledger: LedgerEntry[]; audit: AuditEvent[]; commands: CommittedCommand[]; wallet_bc: number;
}
export interface SettlementResult { acceptance_id: string; grant_id: string | null; issued_bc: number; balance_bc: number; retained_reward: boolean }
export interface RecordCounts { projects: number; objectives: number; actions: number; notes: number; criteria: number; evidence: number; snapshots: number; acceptances: number; corrections: number; grants: number; ledger: number; commands: number; wallet_bc: number; opportunities: number; assignments: number; outcome_events: number; study_manifests: number; schedule_epochs: number; exposures: number; burdens: number; reflections: number; world_commands: number; world_spends: number; world_buildings: number; world_reserved_bc: number; world_spent_bc: number; world_funded_sites: number; world_endings: number; objective_planning: number; checkpoints: number; checkpoint_commands: number }
export interface ExportResult { file_name: string }
export interface BackupStatus { state: "checking" | "idle" | "running" | "ready" | "warning"; latest_successful_day: string | null; snapshot_count: number; warning: string | null }
export interface ImportPreview {
  token: string;
  generation: string;
  file_name: string;
  current_counts: RecordCounts;
  incoming_counts: RecordCounts;
  schema_version: number;
  warning: string;
}
export interface RestoreResult { workspace: Workspace; backup_created: boolean; unchanged: boolean }
export interface CommandError { code: string; message: string }
interface GenerationInput { expected_generation: string }
export interface SaveCriterionInput extends GenerationInput {
  parent_id: string; criterion_lineage_id: string | null; description: string; allocation_bc: number; evidence_policy: EvidencePolicy;
  scope: string; artifact_id: string | null; artifact_version: string | null; requires_independent_work: boolean; required_validators: string[];
}
export interface LockCriterionInput extends GenerationInput { parent_id: string; criterion_lineage_id: string; criterion_version: number }
export interface RecordEvidenceInput extends LockCriterionInput {
  kind: EvidenceKind; description: string; reference: string | null; observed_at: string | null; status: EvidenceStatus;
  assistance: Assistance; uncertainty: string | null; supersedes: string | null;
}
export interface ReviewProgressInput extends LockCriterionInput { evidence_refs: string[]; factual_summary: string; assistance: Assistance }
export interface AcceptProgressInput extends ReviewProgressInput {
  schema_version: 1; command_id: string; expected_parent_revision: number; reviewed_snapshot_id: string; reviewed_snapshot_digest: string;
}
export interface CorrectAcceptanceInput extends GenerationInput { acceptance_id: string; kind: CorrectionKind; reason: string }
export interface RejectReviewInput extends GenerationInput { snapshot_id: string; reason: string }
export interface ProjectInput extends GenerationInput { title: string }
export interface ObjectiveInput extends GenerationInput { project_id: string; title: string; criterion: string }
export interface SaveWorkInput extends GenerationInput {
  objective_id: string;
  description: string;
  stop_condition: string;
  note: string | null;
}

async function command<T>(name: string, input?: object): Promise<T> {
  if (!isTauri()) {
    throw new Error("Open the Ascension desktop app to use your local workspace. A native desktop connection is required.");
  }
  return input === undefined ? invoke<T>(name) : invoke<T>(name, { input });
}

export const loadWorkspace = () => command<Workspace>("load_workspace");
export const loadWorld = () => command<WorldView>("load_world");
export const previewWorld = (input: WorldPreviewInput) => command<WorldPreview>("preview_world", input);
export const worldCommand = (input: WorldCommandInput) => command<{ workspace: Workspace; result: WorldCommandResult }>("world_command", input);
export const createProject = (input: ProjectInput) => command<Workspace>("create_project", input);
export const updateProject = (input: ProjectInput & { id: string }) => command<Workspace>("update_project", input);
export const createObjective = (input: ObjectiveInput) => command<Workspace>("create_objective", input);
export const updateObjective = (input: GenerationInput & { id: string; title: string; criterion: string }) => command<Workspace>("update_objective", input);
export const selectObjective = (input: GenerationInput & { id: string | null }) => command<Workspace>("select_objective", input);
export const saveWork = (input: SaveWorkInput) => command<Workspace>("save_work", input);
export const setObjectiveStatus = (input: GenerationInput & { objective_id: string; status: ObjectiveStatus }) => command<Workspace>("set_objective_status", input);
export const reorderObjectives = (input: GenerationInput & { objective_ids: string[] }) => command<Workspace>("reorder_objectives", input);
export const recordCheckpoint = (input: RecordCheckpointInput) => command<{ workspace: Workspace; checkpoint: WorkCheckpoint }>("record_checkpoint", input);
export const exportWorkspace = () => command<ExportResult | null>("export_workspace");
export const backupStatus = () => command<BackupStatus>("backup_status");
export const retryBackup = () => command<BackupStatus>("retry_backup");
export const previewRecovery = () => command<ImportPreview | null>("preview_recovery");
export const exportRecovery = () => command<ExportResult | null>("export_recovery");
export const previewImport = () => command<ImportPreview | null>("preview_import");
export const restoreImport = (input: { token: string; expected_generation: string }) => command<RestoreResult>("restore_import", input);
export const saveCriterion = (input: SaveCriterionInput) => command<Workspace>("save_criterion", input);
export const lockCriterion = (input: LockCriterionInput) => command<Workspace>("lock_criterion", input);
export const recordEvidence = (input: RecordEvidenceInput) => command<Workspace>("record_evidence", input);
export const reviewProgress = (input: ReviewProgressInput) => command<{ workspace: Workspace; snapshot: ReviewSnapshot }>("review_progress", input);
export const acceptProgress = (input: AcceptProgressInput) => command<{ workspace: Workspace; result: SettlementResult }>("accept_progress", input);
export const correctAcceptance = (input: CorrectAcceptanceInput) => command<Workspace>("correct_acceptance", input);
export const rejectReview = (input: RejectReviewInput) => command<Workspace>("reject_review", input);

export type StudyPhase = "e00" | "e01";
export type StudyStatus = "configured" | "ready" | "active" | "paused" | "completed" | "abandoned";
export type AssignedCondition = "d" | "d_plus_b";
export type OutcomeState = "pending" | "observed_start" | "observed_nonstart" | "missing";
export type TimePrecision = "exact" | "approximate" | "unknown";
export type ExposureKind = "cue_delivered" | "preview_delivered" | "checkpoint_delivered" | "seen" | "voluntary_world" | "deviation";
export type BurdenCategory = "planning" | "logging" | "review" | "game" | "correction" | "other" | "useful_work";
export interface ScheduleEpoch { id: string; label: string; effective_from: string; timezone: string; created_at: string }
export interface StudyManifest {
  id: string; phase: StudyPhase; protocol_version: string; created_at: string; epoch_id: string;
  algorithm: string; block_size: number; allocation_hash: string; window_seconds: number; preview_seconds: number;
  reveal_rule: string; presentation_policy: string; analysis_definition: string; review_after: number[]; status: StudyStatus;
}
export interface ProtocolEvent { id: string; manifest_id: string; status: StudyStatus; reason: string; acknowledgements: string[]; created_at: string; sequence: number }
export interface WorkOpportunity {
  id: string; project_id: string; objective_id: string; criterion_id: string; criterion_version: number; criterion_snapshot: string; criterion_registry_revision: number;
  action_id: string; action_revision: number; action_snapshot: string; stop_snapshot: string;
  domain: string; meaningful_start: string; cue_at: string; window_seconds: number; timezone: string;
  epoch_id: string; eligible: boolean; eligibility_decided_at: string; unavailability: string | null;
  priority: "ordinary" | "high"; available_minutes: number; deadline_at: string | null; context: string | null;
  manifest_id: string | null; protocol_event_id: string | null; created_at: string; outcome_state: OutcomeState;
}
export interface StudyAssignment { id: string; opportunity_id: string; manifest_id: string; position: number; condition: AssignedCondition | null; assigned_at: string; issued_bc_at_assignment: number; h1_saturated: boolean; ledger_sequence: number }
export interface OpportunityOutcome {
  id: string; opportunity_id: string; sequence: number; report: "started" | "nonstart" | "unknown"; started_at: string | null;
  precision: TimePrecision; uncertainty_seconds: number | null; observed_at: string; source: string; note: string | null;
  technical_failure: boolean; uncertainty_reason: string | null; correction_reason: string | null; supersedes: string | null;
  state: OutcomeState; latency_seconds: number | null; censored_at_seconds: number | null; late_start: boolean;
  priority_advanced: boolean | null; quality_note: string | null; acceptance_id: string | null;
}
export interface StudyExposure { id: string; opportunity_id: string | null; kind: ExposureKind; note: string | null; created_at: string; issued_bc: number; h1_saturated: boolean; ledger_sequence: number; protocol_event_id: string | null; preference_sequence: number }
export interface StudyBurden { id: string; opportunity_id: string | null; category: BurdenCategory; minutes: number; note: string | null; created_at: string; method: string }
export interface StudyReflection { id: string; manifest_id: string | null; note: string; created_at: string }
export interface StudyPresentation { opportunity_id: string | null; condition: AssignedCondition | null; boundary: "none" | "preview" | "checkpoint"; world_allowed: boolean; enabled: boolean }
export interface StudyView {
  epochs: ScheduleEpoch[]; manifests: StudyManifest[]; protocol_events: ProtocolEvent[]; opportunities: WorkOpportunity[];
  assignments: StudyAssignment[]; outcomes: OpportunityOutcome[]; exposures: StudyExposure[]; burdens: StudyBurden[];
  reflections: StudyReflection[]; presentation: StudyPresentation;
  preferences: { id: string; sequence: number; enabled: boolean; created_at: string }[];
}
export interface CreateScheduleEpochInput extends GenerationInput { label: string; effective_from: string; timezone: string }
export interface ConfigureStudyInput extends GenerationInput { phase: StudyPhase; epoch_id: string }
export interface TransitionStudyInput extends GenerationInput { manifest_id: string; status: Exclude<StudyStatus, "configured">; reason: string; acknowledgements: string[] }
export interface PlanOpportunityInput extends GenerationInput {
  objective_id: string; epoch_id: string; manifest_id: string | null; domain: string; meaningful_start: string;
  cue_at: string; timezone: string; window_seconds: number; eligible: boolean; unavailability: string | null;
  priority: "ordinary" | "high"; available_minutes: number; deadline_at: string | null; context: string | null;
}
export interface RecordOpportunityOutcomeInput extends GenerationInput {
  opportunity_id: string; report: "started" | "nonstart" | "unknown"; started_at: string | null;
  precision: TimePrecision; uncertainty_seconds: number | null; note: string | null;
  technical_failure: boolean; uncertainty_reason: string | null; correction_reason: string | null;
  priority_advanced: boolean | null; quality_note: string | null; acceptance_id: string | null;
}
export interface RecordStudyExposureInput extends GenerationInput { opportunity_id: string | null; kind: ExposureKind; note: string | null }
export interface RecordStudyBurdenInput extends GenerationInput { opportunity_id: string | null; category: BurdenCategory; minutes: number; note: string | null }
export interface RecordStudyReflectionInput extends GenerationInput { manifest_id: string | null; note: string }
export const createScheduleEpoch = (input: CreateScheduleEpochInput) => command<Workspace>("create_schedule_epoch", input);
export const configureStudy = (input: ConfigureStudyInput) => command<Workspace>("configure_study", input);
export const transitionStudy = (input: TransitionStudyInput) => command<Workspace>("transition_study", input);
export const planOpportunity = (input: PlanOpportunityInput) => command<Workspace>("plan_opportunity", input);
export const planOrdinaryOpportunity = (input: GenerationInput & { objective_id: string; meaningful_start: string; cue_at: string; timezone: string; available_minutes: number }) => command<Workspace>("plan_ordinary_opportunity", input);
export const recordOpportunityOutcome = (input: RecordOpportunityOutcomeInput) => command<Workspace>("record_opportunity_outcome", input);
export const recordStudyExposure = (input: RecordStudyExposureInput) => command<Workspace>("record_study_exposure", input);
export const recordStudyBurden = (input: RecordStudyBurdenInput) => command<Workspace>("record_study_burden", input);
export const recordStudyReflection = (input: RecordStudyReflectionInput) => command<Workspace>("record_study_reflection", input);
export const setConstructionEnabled = (input: GenerationInput & { enabled: boolean }) => command<Workspace>("set_construction_enabled", input);

export function normalizeError(error: unknown): CommandError {
  if (typeof error === "object" && error !== null && "code" in error && "message" in error && typeof error.code === "string" && typeof error.message === "string") {
    return { code: error.code, message: error.message };
  }
  if (error instanceof Error) return { code: "connection_error", message: error.message };
  return { code: "connection_error", message: "The desktop app could not finish this request. Your draft has been kept. Please try again." };
}
