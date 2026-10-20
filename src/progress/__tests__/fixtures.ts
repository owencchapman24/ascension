import type { Criterion, Evidence, ProgressState, ReviewSnapshot, Workspace } from "../../platform/client";
import { emptyStudy } from "../../study/emptyStudy";

export const id = (value: number) => `00000000-0000-4000-8000-${String(value).padStart(12, "0")}`;
export const timestamp = "2026-10-19T12:00:00.000Z";
export const emptyProgress: ProgressState = { registry_revision: 0, parents: [], slots: [], criteria: [], evidence: [], validators: [], snapshots: [], acceptances: [], corrections: [], grants: [], ledger: [], audit: [], commands: [], wallet_bc: 0 };
export const criterion: Criterion = {
  id: id(11), parent_id: id(2), criterion_lineage_id: id(12), criterion_version: 1, reward_slot_id: id(13),
  description: "A diagram names three herbs and their positions.", state: "locked", evidence_policy: "user_attestation", scope: "parent outcome",
  artifact_id: null, artifact_version: null, requires_independent_work: false, required_validators: [], created_at: timestamp,
  locked_at: timestamp, lock_registry_revision: 2, registry_revision: 2,
};
export const evidence: Evidence = {
  id: id(14), parent_id: id(2), criterion_lineage_id: id(12), criterion_version: 1, criterion_id: id(11), kind: "user_attestation",
  source_id: "local_user", source_provenance: "user_recorded", artifact_id: null, artifact_version: null, scope: "parent outcome",
  observed_at: timestamp, recorded_at: timestamp, registry_revision: 3, description: "I drew and labeled three planting positions.", reference: null,
  status: "supports", assistance: "none", uncertainty: null, supersedes: null, validator_id: null,
};
export const snapshot: ReviewSnapshot = {
  id: id(15), parent_id: id(2), criterion_lineage_id: id(12), criterion_version: 1, criterion_id: id(11), registry_revision: 3,
  criterion, evidence_refs: [evidence.id], applicable_evidence: [evidence], applicable_validators: [], missing: [], conflicts: [], eligible: true,
  factual_summary: "The planting diagram is complete.", assistance: "none", digest: "synthetic-snapshot-digest", reviewed_at: timestamp,
  disposition: "pending", rejection_reason: null,
};
const metadata = { revision: 1, created_at: timestamp, updated_at: timestamp };
export const progressWorkspace: Workspace = {
  world: { chapter: null, spending: [], commands: [] },
  study: emptyStudy,
  generation: id(99), selected_objective_id: id(2), projects: [{ ...metadata, id: id(1), title: "Sample garden" }],
  objectives: [{ ...metadata, id: id(2), project_id: id(1), title: "Plan a small herb bed", criterion: criterion.description }],
  actions: [{ ...metadata, id: id(3), objective_id: id(2), description: "Sketch the bed on paper", stop_condition: "Stop after marking three positions." }], notes: [],
  progress: { ...emptyProgress, registry_revision: 3,
    parents: [{ parent_id: id(2), primary_lineage_id: id(12), reward_policy_version: "bc-v1", lifetime_budget_bc: 4, provenance: "prospective" }],
    slots: [{ id: id(13), parent_id: id(2), criterion_lineage_id: id(12), allocation_bc: 4, allocation_locked: true }], criteria: [criterion], evidence: [evidence],
  },
};
