import { invoke, isTauri } from "@tauri-apps/api/core";
import { beforeEach, describe, expect, it, vi } from "vitest";
import * as client from "../client";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn(), isTauri: vi.fn() }));

describe("desktop command boundary", () => {
  beforeEach(() => { vi.resetAllMocks(); vi.mocked(isTauri).mockReturnValue(true); });

  it("rejects browser-only access without invoking native commands", async () => {
    vi.mocked(isTauri).mockReturnValue(false);
    await expect(client.loadWorkspace()).rejects.toThrow("A native desktop connection is required.");
    await expect(client.previewImport()).rejects.toThrow("A native desktop connection is required.");
    expect(invoke).not.toHaveBeenCalled();
  });

  it("loads the authoritative workspace through a narrow command", async () => {
    const workspace = { generation: "generation", selected_objective_id: null, projects: [], objectives: [], actions: [], notes: [] };
    vi.mocked(invoke).mockResolvedValue(workspace);
    await expect(client.loadWorkspace()).resolves.toBe(workspace);
    expect(invoke).toHaveBeenCalledWith("load_workspace");
  });

  it("uses bounded world commands and previews without issuing reward authority", async () => {
    await client.loadWorld(); expect(invoke).toHaveBeenLastCalledWith("load_world");
    const preview = { chapter_id: "chapter", content_version: "river-reach-v1" as const, placements: [{ id: null, building_type: "foundry" as const, parcels: ["a1" as const, "b1" as const] }] };
    await client.previewWorld(preview); expect(invoke).toHaveBeenLastCalledWith("preview_world", { input: preview });
    const command = { command_id: "intent", schema_version: 1 as const, content_version: "river-reach-v1" as const, chapter_id: "chapter", expected_world_revision: 3, operation: { kind: "queue" as const, building_type: "foundry" as const, parcels: ["a1" as const, "b1" as const], reserve_available: true } };
    await client.worldCommand(command); expect(invoke).toHaveBeenLastCalledWith("world_command", { input: command });
    expect(command.operation).not.toHaveProperty("cost_bc"); expect(command.operation).not.toHaveProperty("grant_id"); expect(command.operation).not.toHaveProperty("capabilities");
  });

  it("does not expose the world economy in a browser-only connection", async () => {
    vi.mocked(isTauri).mockReturnValue(false);
    await expect(client.loadWorld()).rejects.toThrow("native desktop connection");
    await expect(client.worldCommand({ command_id: "intent", schema_version: 1, content_version: "river-reach-v1", chapter_id: null, expected_world_revision: 0, operation: { kind: "initialize" } })).rejects.toThrow("native desktop connection");
    expect(invoke).not.toHaveBeenCalled();
  });

  it("serializes create and update input with exact snake_case names", async () => {
    await client.createProject({ expected_generation: "generation", title: "Sample" });
    expect(invoke).toHaveBeenLastCalledWith("create_project", { input: { expected_generation: "generation", title: "Sample" } });
    await client.updateProject({ expected_generation: "generation", id: "project", title: "Renamed" });
    expect(invoke).toHaveBeenLastCalledWith("update_project", { input: { expected_generation: "generation", id: "project", title: "Renamed" } });
    await client.createObjective({ expected_generation: "generation", project_id: "project", title: "Bounded", criterion: "Observable" });
    expect(invoke).toHaveBeenLastCalledWith("create_objective", { input: { expected_generation: "generation", project_id: "project", title: "Bounded", criterion: "Observable" } });
    await client.updateObjective({ expected_generation: "generation", id: "objective", title: "Changed", criterion: "Specific" });
    expect(invoke).toHaveBeenLastCalledWith("update_objective", { input: { expected_generation: "generation", id: "objective", title: "Changed", criterion: "Specific" } });
  });

  it("sends the objective identity, stop condition, nullable note and expected generation atomically", async () => {
    const input = { expected_generation: "generation", objective_id: "objective", description: "Sketch", stop_condition: "Three positions", note: null };
    await client.saveWork(input);
    expect(invoke).toHaveBeenCalledWith("save_work", { input });
    await client.selectObjective({ expected_generation: "generation", id: null });
    expect(invoke).toHaveBeenLastCalledWith("select_objective", { input: { expected_generation: "generation", id: null } });
  });

  it("uses Rust-owned file dialogs and preserves native cancellation", async () => {
    vi.mocked(invoke).mockResolvedValue(null);
    await expect(client.exportWorkspace()).resolves.toBeNull();
    expect(invoke).toHaveBeenLastCalledWith("export_workspace");
    await expect(client.previewImport()).resolves.toBeNull();
    expect(invoke).toHaveBeenLastCalledWith("preview_import");
    await client.restoreImport({ token: "preview-token", expected_generation: "preview-generation" });
    expect(invoke).toHaveBeenLastCalledWith("restore_import", { input: { token: "preview-token", expected_generation: "preview-generation" } });
  });

  it("preserves controlled backend errors for field or conflict feedback", async () => {
    const failure = { code: "conflict", message: "The workspace changed." };
    vi.mocked(invoke).mockRejectedValue(failure);
    await expect(client.saveWork({ expected_generation: "generation", objective_id: "objective", description: "Sketch", stop_condition: "Done", note: null })).rejects.toEqual(failure);
    expect(client.normalizeError(failure)).toEqual(failure);
    expect(client.normalizeError("unexpected internal payload").message).not.toContain("unexpected internal payload");
  });

  it("serializes opportunity declarations and uncertain observations without reward authority", async () => {
    const plan: client.PlanOpportunityInput = { expected_generation: "generation", objective_id: "objective", epoch_id: "epoch", manifest_id: null, domain: "coding", meaningful_start: "Reproduce the named failure", cue_at: "2026-10-19T12:00:00.000Z", timezone: "UTC", window_seconds: 900, eligible: true, unavailability: null, priority: "ordinary", available_minutes: 30, deadline_at: null, context: null };
    await client.planOpportunity(plan); expect(invoke).toHaveBeenLastCalledWith("plan_opportunity", { input: plan });
    const observation: client.RecordOpportunityOutcomeInput = { expected_generation: "generation", opportunity_id: "opportunity", report: "unknown", started_at: null, precision: "unknown", uncertainty_seconds: null, note: null, technical_failure: true, uncertainty_reason: "App unavailable; timing unknown", correction_reason: null, priority_advanced: null, quality_note: null, acceptance_id: null };
    await client.recordOpportunityOutcome(observation); expect(invoke).toHaveBeenLastCalledWith("record_opportunity_outcome", { input: observation });
    expect(observation).not.toHaveProperty("latency_seconds"); expect(observation).not.toHaveProperty("state"); expect(plan).not.toHaveProperty("condition"); expect(plan).not.toHaveProperty("amount_bc");
  });

  it("uses typed explicit lifecycle, exposure, burden and preference commands", async () => {
    const epoch = { expected_generation: "generation", label: "Synthetic routine", effective_from: "2026-01-01T00:00:00.000Z", timezone: "UTC" };
    await client.createScheduleEpoch(epoch); expect(invoke).toHaveBeenLastCalledWith("create_schedule_epoch", { input: epoch });
    const config = { expected_generation: "generation", phase: "e00" as const, epoch_id: "epoch" };
    await client.configureStudy(config); expect(invoke).toHaveBeenLastCalledWith("configure_study", { input: config });
    const transition = { expected_generation: "generation", manifest_id: "manifest", status: "active" as const, reason: "Owner explicitly starts E00", acknowledgements: [] };
    await client.transitionStudy(transition); expect(invoke).toHaveBeenLastCalledWith("transition_study", { input: transition });
    const exposure = { expected_generation: "generation", opportunity_id: "opportunity", kind: "seen" as const, note: "Self-reported" };
    await client.recordStudyExposure(exposure); expect(invoke).toHaveBeenLastCalledWith("record_study_exposure", { input: exposure });
    const burden = { expected_generation: "generation", opportunity_id: null, category: "logging" as const, minutes: 2, note: null };
    await client.recordStudyBurden(burden); expect(invoke).toHaveBeenLastCalledWith("record_study_burden", { input: burden });
    const reflection = { expected_generation: "generation", manifest_id: null, note: "Synthetic weekly reflection" };
    await client.recordStudyReflection(reflection); expect(invoke).toHaveBeenLastCalledWith("record_study_reflection", { input: reflection });
    const preference = { expected_generation: "generation", enabled: false };
    await client.setConstructionEnabled(preference); expect(invoke).toHaveBeenLastCalledWith("set_construction_enabled", { input: preference });
  });

  it("cannot plan, enroll, log, or toggle construction from a browser-only connection", async () => {
    vi.mocked(isTauri).mockReturnValue(false);
    await expect(client.configureStudy({ expected_generation: "g", phase: "e01", epoch_id: "e" })).rejects.toThrow("native desktop connection");
    await expect(client.setConstructionEnabled({ expected_generation: "g", enabled: true })).rejects.toThrow("native desktop connection");
    expect(invoke).not.toHaveBeenCalled();
  });

  it("sends bounded progress inputs without supplying actor, grants, wallet or reward authority", async () => {
    const identity = { expected_generation: "generation", parent_id: "parent", criterion_lineage_id: "lineage", criterion_version: 2 };
    const criterion = { expected_generation: "generation", parent_id: "parent", criterion_lineage_id: "lineage", description: "Four labels", allocation_bc: 4,
      evidence_policy: "user_attestation" as const, scope: "drawing", artifact_id: null, artifact_version: null, requires_independent_work: false, required_validators: [] };
    await client.saveCriterion(criterion); expect(invoke).toHaveBeenLastCalledWith("save_criterion", { input: criterion });
    await client.lockCriterion(identity); expect(invoke).toHaveBeenLastCalledWith("lock_criterion", { input: identity });
    const evidence = { ...identity, kind: "user_attestation" as const, description: "I added four labels.", reference: null, observed_at: null, status: "supports" as const, assistance: "none" as const, uncertainty: null, supersedes: null };
    await client.recordEvidence(evidence); expect(invoke).toHaveBeenLastCalledWith("record_evidence", { input: evidence });
    const review = { ...identity, evidence_refs: ["evidence"], factual_summary: "The drawing is labeled.", assistance: "none" as const };
    await client.reviewProgress(review); expect(invoke).toHaveBeenLastCalledWith("review_progress", { input: review });
    const acceptance = { ...review, schema_version: 1 as const, command_id: "intent", expected_parent_revision: 3, reviewed_snapshot_id: "snapshot", reviewed_snapshot_digest: "digest" };
    await client.acceptProgress(acceptance); expect(invoke).toHaveBeenLastCalledWith("accept_progress", { input: acceptance });
    const correction = { expected_generation: "generation", acceptance_id: "acceptance", kind: "incorrect_acceptance" as const, reason: "One label was absent." };
    await client.correctAcceptance(correction); expect(invoke).toHaveBeenLastCalledWith("correct_acceptance", { input: correction });
    const rejection = { expected_generation: "generation", snapshot_id: "snapshot", reason: "Incomplete" };
    await client.rejectReview(rejection); expect(invoke).toHaveBeenLastCalledWith("reject_review", { input: rejection });
  });
});
