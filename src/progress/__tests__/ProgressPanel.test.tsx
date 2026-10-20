import { useState } from "react";
import { act, cleanup, render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import * as api from "../../platform/client";
import type { Workspace } from "../../platform/client";
import ProgressPanel from "../ProgressPanel";
import { criterion, evidence, id, progressWorkspace, snapshot, timestamp } from "./fixtures";

vi.mock("../../platform/client", async (original) => ({ ...(await original<typeof api>()), saveCriterion: vi.fn(), lockCriterion: vi.fn(), recordEvidence: vi.fn(), reviewProgress: vi.fn(), acceptProgress: vi.fn(), correctAcceptance: vi.fn(), rejectReview: vi.fn() }));
const dirtyChanged = vi.fn();
function Harness({ initial = progressWorkspace }: { initial?: Workspace }) {
  const [workspace, setWorkspace] = useState(initial);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState("");
  return <><p>Saved balance: {workspace.progress.wallet_bc}</p>{error && <p role="alert">{error}</p>}
    <fieldset disabled={busy}><ProgressPanel workspace={workspace} objective={workspace.objectives[0]} busy={busy} onDirtyChange={dirtyChanged}
      onMutation={async (task) => { setBusy(true); setError(""); try { setWorkspace(await task()); return true; } catch (failure) { setError(api.normalizeError(failure).message); return false; } finally { setBusy(false); } }} /></fieldset></>;
}
async function ready(initial = progressWorkspace) {
  const user = userEvent.setup({ delay: null }); render(<Harness initial={structuredClone(initial)} />);
  await user.click(screen.getByText("Progress and evidence")); return user;
}
const reviewed = { ...progressWorkspace, progress: { ...progressWorkspace.progress, snapshots: [snapshot] } };
const accepted: Workspace = {
  ...reviewed, generation: id(100), progress: { ...reviewed.progress, wallet_bc: 4, snapshots: [{ ...snapshot, disposition: "accepted" }],
    acceptances: [{ id: id(16), parent_id: id(2), criterion_lineage_id: id(12), criterion_version: 1, criterion_id: criterion.id,
      reward_slot_id: id(13), snapshot_id: snapshot.id, actor: "local_user", method: "user_confirmed", assistance: "none",
      factual_summary: snapshot.factual_summary, accepted_at: timestamp, status: "accepted", correction_id: null }],
    grants: [{ id: id(17), parent_id: id(2), reward_slot_id: id(13), criterion_lineage_id: id(12), acceptance_id: id(16), reward_policy_version: "bc-v1", amount_bc: 4, created_at: timestamp }],
    ledger: [{ id: id(18), grant_id: id(17), sequence: 1, amount_bc: 4, created_at: timestamp }],
  },
};

describe("progress review", () => {
  beforeEach(() => vi.resetAllMocks());
  afterEach(cleanup);

  it("requires prospective lock confirmation and keeps the draft until a committed lock returns", async () => {
    const draft: Workspace = { ...progressWorkspace, progress: { ...progressWorkspace.progress, criteria: [{ ...criterion, state: "draft", locked_at: null, lock_registry_revision: null }], evidence: [] } };
    let finish!: (value: Workspace) => void;
    vi.mocked(api.lockCriterion).mockReturnValue(new Promise((resolve) => { finish = resolve; }));
    const user = await ready(draft);
    expect(screen.getByRole("button", { name: "Lock criterion" })).toBeDisabled();
    await user.click(screen.getByLabelText(/I have reviewed this criterion/));
    await user.click(screen.getByRole("button", { name: "Lock criterion" }));
    expect(api.lockCriterion).toHaveBeenCalledWith({ expected_generation: draft.generation, parent_id: id(2), criterion_lineage_id: id(12), criterion_version: 1 });
    expect(screen.getByRole("heading", { name: "Criterion draft · version 1" })).toBeInTheDocument();
    expect(screen.queryByRole("button", { name: "Record evidence" })).not.toBeInTheDocument();
    await act(async () => finish(progressWorkspace));
    expect(screen.getByRole("heading", { name: "Locked criterion · version 1" })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Record evidence" })).toBeInTheDocument();
  });

  it("prepares a new criterion version under the existing immutable reward identity", async () => {
    vi.mocked(api.saveCriterion).mockResolvedValue(progressWorkspace);
    const user = await ready();
    await user.click(screen.getByLabelText("Show fictional rewards"));
    await user.click(screen.getByRole("button", { name: "Prepare next criterion version" }));
    expect(screen.getByLabelText("Acceptance criterion")).toHaveFocus();
    expect(screen.getByLabelText("Predeclared Build Credit allocation")).toBeDisabled();
    await user.clear(screen.getByLabelText("Acceptance criterion"));
    await user.type(screen.getByLabelText("Acceptance criterion"), "The revised diagram labels four positions.");
    await user.click(screen.getByRole("button", { name: "Save criterion draft" }));
    expect(api.saveCriterion).toHaveBeenCalledWith(expect.objectContaining({ parent_id: id(2), criterion_lineage_id: id(12), allocation_bc: 4, description: "The revised diagram labels four positions.", required_validators: [] }));
    expect(screen.queryByLabelText("Acceptance criterion")).not.toBeInTheDocument();
  });

  it("blocks overallocated new criteria and rejects fractional draft allocations without losing input", async () => {
    const draft: Workspace = { ...progressWorkspace, progress: { ...progressWorkspace.progress,
      criteria: [{ ...criterion, state: "draft", locked_at: null, lock_registry_revision: null }],
      slots: [{ ...progressWorkspace.progress.slots[0], allocation_locked: false }], evidence: [],
    } };
    const user = await ready(draft);
    expect(screen.getByRole("button", { name: "Add meaningful criterion" })).toBeDisabled();
    await user.click(screen.getByLabelText("Show fictional rewards"));
    await user.click(screen.getByRole("button", { name: "Edit criterion draft" }));
    await user.clear(screen.getByLabelText("Predeclared Build Credit allocation"));
    await user.type(screen.getByLabelText("Predeclared Build Credit allocation"), "1.5");
    await user.click(screen.getByRole("button", { name: "Save criterion draft" }));
    expect(await screen.findByRole("alert")).toHaveTextContent("The declared allocation must be a whole number from one through four.");
    expect(api.saveCriterion).not.toHaveBeenCalled();
    expect(screen.getByLabelText("Predeclared Build Credit allocation")).toHaveValue(1.5);
  });

  it("preserves failed evidence drafts and records source, assistance, uncertainty and explicit supersession on retry", async () => {
    vi.mocked(api.recordEvidence).mockRejectedValueOnce({ code: "storage_error", message: "Evidence could not be saved." }).mockResolvedValue(progressWorkspace);
    const user = await ready(); await user.click(screen.getByRole("button", { name: "Record evidence" }));
    expect(screen.queryByRole("option", { name: "Registered validator" })).not.toBeInTheDocument();
    await user.selectOptions(screen.getByLabelText("Evidence source"), "artifact_reference");
    await user.type(screen.getByLabelText("What happened or was observed?"), "The revised drawing contains four labels.");
    await user.type(screen.getByLabelText("Reference or source description (optional)"), "Synthetic garden diagram v2");
    await user.selectOptions(screen.getByLabelText("How was the work produced?"), "assisted");
    await user.type(screen.getByLabelText("Uncertainty or limitations (optional)"), "Sun exposure remains uncertain.");
    await user.selectOptions(screen.getByLabelText("Explicitly supersedes earlier evidence"), evidence.id);
    await user.click(screen.getByRole("button", { name: "Save evidence" }));
    expect(await screen.findByRole("alert")).toHaveTextContent("Evidence could not be saved.");
    expect(screen.getByLabelText("What happened or was observed?")).toHaveValue("The revised drawing contains four labels.");
    expect(screen.getByText("Saved balance: 0")).toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: "Save evidence" }));
    expect(api.recordEvidence).toHaveBeenLastCalledWith(expect.objectContaining({ kind: "artifact_reference", reference: "Synthetic garden diagram v2", assistance: "assisted", uncertainty: "Sun exposure remains uncertain.", supersedes: evidence.id, observed_at: null }));
    expect(screen.queryByLabelText("What happened or was observed?")).not.toBeInTheDocument();
    await waitFor(() => expect(dirtyChanged).toHaveBeenLastCalledWith(false));
  });

  it("shows contradictory evidence from the authoritative snapshot even when the client omits its reference", async () => {
    const contrary = { ...evidence, id: id(24), status: "contradicts" as const, description: "One required label is missing." };
    const conflictSnapshot = { ...snapshot, applicable_evidence: [evidence, contrary], eligible: false, conflicts: ["An applicable record contradicts completion."] };
    vi.mocked(api.reviewProgress).mockResolvedValue({ workspace: { ...reviewed, progress: { ...reviewed.progress, snapshots: [conflictSnapshot] } }, snapshot: conflictSnapshot });
    const user = await ready({ ...progressWorkspace, progress: { ...progressWorkspace.progress, evidence: [evidence, contrary] } });
    await user.click(screen.getByRole("button", { name: "Review progress" }));
    await user.type(screen.getByLabelText("Factual progress to review"), "The diagram is ready for review.");
    await user.click(screen.getByLabelText("One required label is missing. (contradicts)"));
    await user.click(screen.getByRole("button", { name: "Save review snapshot" }));
    expect(api.reviewProgress).toHaveBeenCalledWith(expect.objectContaining({ evidence_refs: [evidence.id] }));
    expect(screen.getByText("One required label is missing.")).toBeInTheDocument();
    expect(screen.getByText("An applicable record contradicts completion.")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Accept qualifying progress" })).toBeDisabled();
    expect(api.acceptProgress).not.toHaveBeenCalled();
  });

  it("keeps acceptance uncommitted until acknowledgment and reuses the same intent when retrying a failed command", async () => {
    let finish!: (value: { workspace: Workspace; result: api.SettlementResult }) => void;
    vi.mocked(api.acceptProgress).mockRejectedValueOnce({ code: "storage_error", message: "Acceptance could not be confirmed." })
      .mockReturnValueOnce(new Promise((resolve) => { finish = resolve; }));
    const user = await ready(reviewed);
    await user.click(screen.getByRole("button", { name: "Accept qualifying progress" }));
    expect(await screen.findByRole("alert")).toHaveTextContent("Acceptance could not be confirmed.");
    const first = vi.mocked(api.acceptProgress).mock.calls[0][0];
    expect(first).toEqual({ schema_version: 1, command_id: expect.any(String), expected_generation: reviewed.generation, parent_id: id(2), criterion_lineage_id: id(12), criterion_version: 1,
      expected_parent_revision: 1, reviewed_snapshot_id: snapshot.id, reviewed_snapshot_digest: snapshot.digest, evidence_refs: [evidence.id], factual_summary: snapshot.factual_summary, assistance: "none" });
    await user.click(screen.getByRole("button", { name: "Accept qualifying progress" }));
    expect(vi.mocked(api.acceptProgress).mock.calls[1][0]).toEqual(first);
    expect(screen.getByText("Saved balance: 0")).toBeInTheDocument();
    expect(screen.queryByRole("heading", { name: "Accomplishment history" })).not.toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Accept qualifying progress" })).toBeDisabled();
    await act(async () => finish({ workspace: accepted, result: { acceptance_id: id(16), grant_id: id(17), issued_bc: 4, balance_bc: 4, retained_reward: false } }));
    expect(screen.getByText("Saved balance: 4")).toBeInTheDocument();
    expect(screen.getByRole("heading", { name: "Accomplishment history" })).toBeInTheDocument();
    expect(screen.queryByRole("button", { name: "Accept qualifying progress" })).not.toBeInTheDocument();
  });

  it("displays the exact newly committed review even when the clock moved backward", async () => {
    const earlierClock = { ...snapshot, id: id(25), reviewed_at: "2026-10-18T12:00:00.000Z", factual_summary: "Fresh review after correcting the explanation." };
    vi.mocked(api.reviewProgress).mockResolvedValue({ workspace: { ...reviewed, progress: { ...reviewed.progress, snapshots: [snapshot, earlierClock] } }, snapshot: earlierClock });
    const user = await ready(reviewed);
    await user.click(screen.getByRole("button", { name: "Review progress" }));
    await user.type(screen.getByLabelText("Factual progress to review"), earlierClock.factual_summary);
    await user.click(screen.getByRole("button", { name: "Save review snapshot" }));
    const review = screen.getByRole("region", { name: "Saved evidence review" });
    expect(within(review).getByText(earlierClock.factual_summary)).toBeInTheDocument();
    expect(within(review).queryByText(snapshot.factual_summary)).not.toBeInTheDocument();
  });

  it("reports the corrected current status when an exact acceptance retry returns its original result", async () => {
    const corrected: Workspace = { ...accepted, progress: { ...accepted.progress,
      acceptances: [{ ...accepted.progress.acceptances[0], status: "retracted", correction_id: id(19) }],
      corrections: [{ id: id(19), acceptance_id: id(16), kind: "incorrect_acceptance", reason: "A required label was absent.", created_at: timestamp, registry_revision: 4 }],
    } };
    vi.mocked(api.acceptProgress).mockRejectedValueOnce({ code: "connection_error", message: "The response was interrupted." })
      .mockResolvedValueOnce({ workspace: corrected, result: { acceptance_id: id(16), grant_id: id(17), issued_bc: 4, balance_bc: 4, retained_reward: false } });
    const user = await ready(reviewed);
    await user.click(screen.getByRole("button", { name: "Accept qualifying progress" }));
    expect(await screen.findByRole("alert")).toHaveTextContent("The response was interrupted.");
    await user.click(screen.getByRole("button", { name: "Accept qualifying progress" }));
    expect(vi.mocked(api.acceptProgress).mock.calls[1][0]).toEqual(vi.mocked(api.acceptProgress).mock.calls[0][0]);
    expect(await screen.findByRole("status")).toHaveTextContent("Its acceptance is now retracted; the correction remains in effect.");
    expect(screen.queryByText("Progress accepted and saved on this device.")).not.toBeInTheDocument();
    expect(screen.getByText("Acceptance retracted")).toBeInTheDocument();
    expect(screen.getByText("Saved balance: 4")).toBeInTheDocument();
  });

  it("blocks stale snapshot acceptance and leaves pending reviews without a mutation", async () => {
    const user = await ready({ ...reviewed, progress: { ...reviewed.progress, registry_revision: 4 } });
    expect(screen.getByText(/Evidence or criteria changed after this review/)).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Accept qualifying progress" })).toBeDisabled();
    await user.type(screen.getByLabelText("Reason to reject this review (optional)"), "Need another observation.");
    await user.click(screen.getByRole("button", { name: "Leave pending" }));
    expect(api.acceptProgress).not.toHaveBeenCalled(); expect(api.rejectReview).not.toHaveBeenCalled();
    expect(screen.getByLabelText("Reason to reject this review (optional)")).toHaveValue("");
  });

  it("saves an explicit rejection without creating an accomplishment", async () => {
    vi.mocked(api.rejectReview).mockResolvedValue({ ...reviewed, progress: { ...reviewed.progress, snapshots: [{ ...snapshot, disposition: "rejected", rejection_reason: "The diagram remains incomplete." }] } });
    const user = await ready(reviewed);
    await user.type(screen.getByLabelText("Reason to reject this review (optional)"), "The diagram remains incomplete.");
    await user.click(screen.getByRole("button", { name: "Reject review" }));
    expect(api.rejectReview).toHaveBeenCalledWith({ expected_generation: reviewed.generation, snapshot_id: snapshot.id, reason: "The diagram remains incomplete." });
    expect(screen.getByText("Rejection reason: The diagram remains incomplete.")).toBeInTheDocument();
    expect(screen.queryByRole("heading", { name: "Accomplishment history" })).not.toBeInTheDocument();
  });

  it("separates corrected factual history from retained fictional credit and hides rewards on request", async () => {
    const corrected: Workspace = { ...accepted, progress: { ...accepted.progress,
      acceptances: [{ ...accepted.progress.acceptances[0], status: "historical", correction_id: id(19) }],
      corrections: [{ id: id(19), acceptance_id: id(16), kind: "historical_reopened", reason: "The planting plan changed later.", created_at: timestamp, registry_revision: 4 }],
    } };
    vi.mocked(api.correctAcceptance).mockResolvedValue(corrected);
    const user = await ready(accepted);
    expect(screen.queryByText(/4 Build Credits/)).not.toBeInTheDocument();
    await user.click(screen.getByLabelText("Show fictional rewards"));
    expect(screen.getByText("4 Build Credits")).toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: "Correct this accomplishment" }));
    await user.selectOptions(screen.getByLabelText("What needs correcting?"), "historical_reopened");
    await user.type(screen.getByLabelText("Factual correction reason"), "The planting plan changed later.");
    await user.click(screen.getByRole("button", { name: "Save factual correction" }));
    expect(api.correctAcceptance).toHaveBeenCalledWith({ expected_generation: accepted.generation, acceptance_id: id(16), kind: "historical_reopened", reason: "The planting plan changed later." });
    const history = screen.getByRole("region", { name: "Accomplishment history" });
    expect(within(history).getByText("Historically accepted; reopened")).toBeInTheDocument();
    expect(within(history).getByText(/retained as fictional credit; this is not current accepted progress/)).toBeInTheDocument();
    await user.click(screen.getByLabelText("Show fictional rewards"));
    expect(screen.queryByText(/retained as fictional credit/)).not.toBeInTheDocument();
    expect(within(history).getByText("Historically accepted; reopened")).toBeInTheDocument();
  });
});
