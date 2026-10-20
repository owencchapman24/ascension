import { invoke, isTauri } from "@tauri-apps/api/core";

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
export interface RecordCounts { projects: number; objectives: number; actions: number; notes: number; criteria: number; evidence: number; snapshots: number; acceptances: number; corrections: number; grants: number; ledger: number; commands: number; wallet_bc: number }
export interface ExportResult { file_name: string }
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
export const createProject = (input: ProjectInput) => command<Workspace>("create_project", input);
export const updateProject = (input: ProjectInput & { id: string }) => command<Workspace>("update_project", input);
export const createObjective = (input: ObjectiveInput) => command<Workspace>("create_objective", input);
export const updateObjective = (input: GenerationInput & { id: string; title: string; criterion: string }) => command<Workspace>("update_objective", input);
export const selectObjective = (input: GenerationInput & { id: string | null }) => command<Workspace>("select_objective", input);
export const saveWork = (input: SaveWorkInput) => command<Workspace>("save_work", input);
export const exportWorkspace = () => command<ExportResult | null>("export_workspace");
export const previewImport = () => command<ImportPreview | null>("preview_import");
export const restoreImport = (input: { token: string; expected_generation: string }) => command<RestoreResult>("restore_import", input);
export const saveCriterion = (input: SaveCriterionInput) => command<Workspace>("save_criterion", input);
export const lockCriterion = (input: LockCriterionInput) => command<Workspace>("lock_criterion", input);
export const recordEvidence = (input: RecordEvidenceInput) => command<Workspace>("record_evidence", input);
export const reviewProgress = (input: ReviewProgressInput) => command<{ workspace: Workspace; snapshot: ReviewSnapshot }>("review_progress", input);
export const acceptProgress = (input: AcceptProgressInput) => command<{ workspace: Workspace; result: SettlementResult }>("accept_progress", input);
export const correctAcceptance = (input: CorrectAcceptanceInput) => command<Workspace>("correct_acceptance", input);
export const rejectReview = (input: RejectReviewInput) => command<Workspace>("reject_review", input);

export function normalizeError(error: unknown): CommandError {
  if (typeof error === "object" && error !== null && "code" in error && "message" in error && typeof error.code === "string" && typeof error.message === "string") {
    return { code: error.code, message: error.message };
  }
  if (error instanceof Error) return { code: "connection_error", message: error.message };
  return { code: "connection_error", message: "The desktop app could not finish this request. Your draft has been kept. Please try again." };
}
