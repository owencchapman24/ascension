import { act, cleanup, render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import * as api from "../../platform/client";
import DailyWork from "../DailyWork";
import ObjectiveManager from "../ObjectiveManager";
import BackupPanel from "../BackupPanel";
import { id, progressWorkspace, timestamp } from "../../progress/__tests__/fixtures";

vi.mock("../../platform/client", async (load) => ({ ...await load<typeof api>(), loadWorkspace: vi.fn(), recordCheckpoint: vi.fn(), setObjectiveStatus: vi.fn(), reorderObjectives: vi.fn(), backupStatus: vi.fn(), retryBackup: vi.fn() }));
const checkpoint: api.WorkCheckpoint = { id: id(201), command_id: id(202), sequence: 1, objective_id: id(2), note: "Marked the first position.", reference: "paper diagram", provenance: "self_report", recorded_at: timestamp, action_id: id(3), action_revision: 1, description: "Sketch the bed on paper", stop_condition: "Stop after marking three positions.", resumption_note: "Continue at the second position.", supersedes: null, correction_reason: null };
const mutation = vi.fn(async (task: () => Promise<api.Workspace>) => { try { await task(); return true; } catch { return false; } });
const onDirty = vi.fn(); const onSelect = vi.fn();
function panel(workspace = progressWorkspace) { return render(<DailyWork workspace={workspace} objective={workspace.objectives[0]} busy={false} onMutation={mutation} onDirtyChange={onDirty} onSelect={onSelect} />); }
beforeEach(() => { vi.clearAllMocks(); vi.mocked(api.recordCheckpoint).mockResolvedValue({ workspace: progressWorkspace, checkpoint }); });
afterEach(cleanup);

describe("daily work checkpoint", () => {
  it("prefills saved context, commits one complete command and waits for confirmation", async () => {
    const user = userEvent.setup(); panel({ ...progressWorkspace, notes: [{ id: id(4), objective_id: id(2), revision: 1, created_at: timestamp, updated_at: timestamp, text: "Start at the sunny corner." }] });
    await user.click(screen.getByRole("button", { name: "Save where I stopped" }));
    expect(screen.getByLabelText("Next action")).toHaveValue(progressWorkspace.actions[0].description);
    expect(screen.getByLabelText("Stopping condition")).toHaveValue(progressWorkspace.actions[0].stop_condition);
    expect(screen.getByLabelText("Resumption note (optional)")).toHaveValue("Start at the sunny corner.");
    expect(screen.getByLabelText("What changed?")).toHaveFocus();
    await user.type(screen.getByLabelText("What changed?"), checkpoint.note);
    await user.clear(screen.getByLabelText("Next action")); await user.type(screen.getByLabelText("Next action"), "Label the next position");
    let resolve!: (result: { workspace: api.Workspace; checkpoint: api.WorkCheckpoint }) => void;
    vi.mocked(api.recordCheckpoint).mockReturnValueOnce(new Promise((done) => { resolve = done; }));
    await user.click(screen.getByRole("button", { name: "Save checkpoint" }));
    expect(screen.getByLabelText("What changed?")).toHaveValue(checkpoint.note);
    expect(api.recordCheckpoint).toHaveBeenCalledWith(expect.objectContaining({ expected_generation: progressWorkspace.generation, objective_id: id(2), note: checkpoint.note, description: "Label the next position", stop_condition: progressWorkspace.actions[0].stop_condition, resumption_note: "Start at the sunny corner.", reference: null, supersedes: null, correction_reason: null }));
    expect(api.recordCheckpoint).toHaveBeenCalledTimes(1);
    await act(async () => resolve({ workspace: progressWorkspace, checkpoint }));
    expect(screen.queryByLabelText("What changed?")).not.toBeInTheDocument();
    await waitFor(() => expect(screen.getByRole("button", { name: "Save where I stopped" })).toHaveFocus());
  });

  it("keeps the full command unchanged after lost response, including generation and identity", async () => {
    vi.mocked(api.recordCheckpoint).mockRejectedValueOnce(new Error("response unavailable"));
    const user = userEvent.setup(); const view = panel(); await user.click(screen.getByRole("button", { name: "Save where I stopped" })); await user.type(screen.getByLabelText("What changed?"), checkpoint.note);
    await user.click(screen.getByRole("button", { name: "Save checkpoint" }));
    expect(screen.getByRole("alert")).toHaveTextContent("Your draft is kept"); expect(screen.getByLabelText("What changed?")).toHaveValue(checkpoint.note);
    const original = structuredClone(vi.mocked(api.recordCheckpoint).mock.calls[0][0]);
    view.rerender(<DailyWork workspace={{ ...progressWorkspace, generation: id(999) }} objective={progressWorkspace.objectives[0]} busy={false} onMutation={mutation} onDirtyChange={onDirty} onSelect={onSelect} />);
    await user.click(screen.getByRole("button", { name: "Retry same checkpoint" }));
    expect(vi.mocked(api.recordCheckpoint).mock.calls[1][0]).toEqual(original); expect(original.command_id).toMatch(/^[0-9a-f-]{36}$/);
  });

  it("requires factual text and preserves the draft on validation failure", async () => {
    const user = userEvent.setup(); panel(); await user.click(screen.getByRole("button", { name: "Save where I stopped" })); await user.click(screen.getByRole("button", { name: "Save checkpoint" }));
    expect(screen.getByRole("alert")).toHaveTextContent("What changed needs a nonempty value"); expect(screen.getByLabelText("What changed?")).toHaveFocus(); expect(api.recordCheckpoint).not.toHaveBeenCalled(); expect(onDirty).toHaveBeenLastCalledWith(true);
  });
  it("finds an already committed checkpoint after an ambiguous response without sending another save", async () => {
    vi.mocked(api.recordCheckpoint).mockRejectedValueOnce(new Error("response unavailable"));
    const user = userEvent.setup(); panel(); await user.click(screen.getByRole("button", { name: "Save where I stopped" })); await user.type(screen.getByLabelText("What changed?"), checkpoint.note); await user.click(screen.getByRole("button", { name: "Save checkpoint" }));
    const input = vi.mocked(api.recordCheckpoint).mock.calls[0][0];
    vi.mocked(api.loadWorkspace).mockResolvedValue({ ...progressWorkspace, daily: { ...progressWorkspace.daily, checkpoints: [{ ...checkpoint, command_id: input.command_id }], commands: [{ id: input.command_id, digest: "synthetic", input, checkpoint_id: checkpoint.id }] } });
    await user.click(screen.getByRole("button", { name: "Check saved history and keep draft" })); expect(screen.queryByLabelText("What changed?")).not.toBeInTheDocument(); expect(api.recordCheckpoint).toHaveBeenCalledTimes(1);
  });
  it("recovers from a definitive generation conflict without discarding the draft", async () => {
    vi.mocked(api.recordCheckpoint).mockRejectedValueOnce({ code: "conflict", message: "Saved records changed." }); vi.mocked(api.loadWorkspace).mockResolvedValue({ ...progressWorkspace, generation: id(998) });
    const user = userEvent.setup(); const view = panel(); await user.click(screen.getByRole("button", { name: "Save where I stopped" })); await user.type(screen.getByLabelText("What changed?"), "Keep this checkpoint"); await user.click(screen.getByRole("button", { name: "Save checkpoint" }));
    const firstId = vi.mocked(api.recordCheckpoint).mock.calls[0][0].command_id;
    expect(screen.getByLabelText("What changed?")).toBeEnabled(); expect(screen.getByRole("button", { name: "Save checkpoint" })).toBeDisabled();
    await user.click(screen.getByRole("button", { name: "Check saved history and keep draft" }));
    view.rerender(<DailyWork workspace={{ ...progressWorkspace, generation: id(998) }} objective={progressWorkspace.objectives[0]} busy={false} onMutation={mutation} onDirtyChange={onDirty} onSelect={onSelect} />);
    expect(screen.getByLabelText("What changed?")).toHaveValue("Keep this checkpoint"); await user.click(screen.getByRole("button", { name: "Save checkpoint" }));
    expect(vi.mocked(api.recordCheckpoint).mock.calls[1][0].expected_generation).toBe(id(998)); expect(vi.mocked(api.recordCheckpoint).mock.calls[1][0].command_id).not.toBe(firstId);
  });

  it("corrects an original self-report with explicit lineage without editing old history", async () => {
    const user = userEvent.setup(); panel({ ...progressWorkspace, daily: { ...progressWorkspace.daily, checkpoints: [checkpoint] } });
    await user.click(screen.getByText("Checkpoint history · 1")); expect(screen.getByText("Unverified reference: paper diagram")).toBeVisible(); await user.click(screen.getByRole("button", { name: "Correct checkpoint" }));
    await user.clear(screen.getByLabelText("What changed?")); await user.type(screen.getByLabelText("What changed?"), "The position was only outlined.");
    await user.click(screen.getByRole("button", { name: "Save checkpoint correction" })); expect(api.recordCheckpoint).not.toHaveBeenCalled();
    await user.type(screen.getByLabelText("Correction reason"), "The earlier note overstated the result."); await user.click(screen.getByRole("button", { name: "Save checkpoint correction" }));
    expect(api.recordCheckpoint).toHaveBeenCalledWith(expect.objectContaining({ supersedes: checkpoint.id, correction_reason: "The earlier note overstated the result.", note: "The position was only outlined." })); expect(checkpoint.note).toBe("Marked the first position.");
  });
});

describe("voluntary initiation support", () => {
  it("uses keyboard-accessible smaller entry without altering the main action or any authority", async () => {
    const user = userEvent.setup(); panel(); screen.getByText("Need a different way to start?").focus(); await user.keyboard("{Enter}{Tab}{Enter}");
    expect(screen.getByLabelText("Small action")).toHaveFocus(); await user.type(screen.getByLabelText("Small action"), "Mark one point"); await user.type(screen.getByLabelText("Observable stopping point"), "One point is visible"); await user.click(screen.getByRole("button", { name: "Use this smaller entry" }));
    expect(screen.getByRole("complementary", { name: "Temporary smaller entry" })).toHaveTextContent("Mark one point"); expect(progressWorkspace.actions[0].description).toBe("Sketch the bed on paper"); expect(mutation).not.toHaveBeenCalled();
  });
  it("requires an explicit alternate focus and makes rest and resume side-effect free", async () => {
    const other = { ...progressWorkspace.objectives[0], id: id(5), title: "Gather containers" };
    const user = userEvent.setup(); panel({ ...progressWorkspace, objectives: [...progressWorkspace.objectives, other], daily: { ...progressWorkspace.daily, objectives: [...progressWorkspace.daily.objectives, { objective_id: other.id, status: "active", priority: 1, revision: 1, updated_at: timestamp }] } });
    await user.click(screen.getByRole("button", { name: "Resume work" })); await user.click(screen.getByText("Need a different way to start?")); await user.click(screen.getByRole("button", { name: "Rest or step away" })); expect(screen.getByRole("status")).toHaveTextContent("You can step away"); expect(mutation).not.toHaveBeenCalled(); expect(onSelect).not.toHaveBeenCalled();
    await user.click(screen.getByRole("button", { name: "Choose another priority" })); expect(onSelect).not.toHaveBeenCalled(); await user.click(screen.getByRole("button", { name: "Focus Gather containers" })); expect(onSelect).toHaveBeenCalledWith(other.id); expect(mutation).not.toHaveBeenCalled();
  });
});

describe("organization and recovery controls", () => {
  it("sends complete explicit active ordering and separate lifecycle commands", async () => {
    const other = { ...progressWorkspace.objectives[0], id: id(5), title: "Gather containers" }; const workspace = { ...progressWorkspace, objectives: [...progressWorkspace.objectives, other], daily: { ...progressWorkspace.daily, objectives: [...progressWorkspace.daily.objectives, { objective_id: other.id, status: "active" as const, priority: 1, revision: 1, updated_at: timestamp }] } };
    vi.mocked(api.reorderObjectives).mockResolvedValue(workspace); vi.mocked(api.setObjectiveStatus).mockResolvedValue(workspace);
    const user = userEvent.setup(); render(<ObjectiveManager workspace={workspace} busy={false} onMutation={mutation} onSelect={onSelect} />); await user.click(screen.getByText("Manage objectives and priority"));
    await user.click(screen.getByRole("button", { name: "Move Gather containers higher" })); expect(api.reorderObjectives).toHaveBeenCalledWith({ expected_generation: workspace.generation, objective_ids: [other.id, id(2)] });
    await user.click(screen.getByRole("button", { name: "Pause Gather containers" })); expect(api.setObjectiveStatus).toHaveBeenLastCalledWith({ expected_generation: workspace.generation, objective_id: other.id, status: "paused" });
    await user.click(screen.getByRole("button", { name: "Archive Gather containers" })); expect(api.setObjectiveStatus).toHaveBeenLastCalledWith({ expected_generation: workspace.generation, objective_id: other.id, status: "archived" });
  });
  it("keeps archived context inspectable and offers restoration without claiming completion", async () => {
    const workspace = { ...progressWorkspace, daily: { ...progressWorkspace.daily, objectives: [{ ...progressWorkspace.daily.objectives[0], status: "archived" as const }], checkpoints: [checkpoint] } };
    const user = userEvent.setup(); render(<ObjectiveManager workspace={workspace} busy={false} onMutation={mutation} onSelect={onSelect} />); await user.click(screen.getByText("Manage objectives and priority"));
    const archived = screen.getByRole("region", { name: "archived objectives" }); await user.click(within(archived).getByText("Saved context and history")); expect(within(archived).getByText(/Checkpoint/)).toHaveTextContent(checkpoint.note);
    await user.click(screen.getByRole("button", { name: `Restore ${workspace.objectives[0].title}` })); expect(api.setObjectiveStatus).toHaveBeenCalledWith({ expected_generation: workspace.generation, objective_id: id(2), status: "active" });
  });
  it("reports backup failure separately, retries, and only previews recovery explicitly", async () => {
    vi.mocked(api.backupStatus).mockResolvedValue({ state: "warning", latest_successful_day: null, snapshot_count: 0, warning: "Recovery storage is unavailable." }); vi.mocked(api.retryBackup).mockResolvedValue({ state: "ready", latest_successful_day: "2026-10-21", snapshot_count: 1, warning: null });
    const preview = vi.fn(); const user = userEvent.setup(); render(<BackupPanel generation={id(99)} busy={false} onPreview={preview} onExport={vi.fn()} />);
    expect(await screen.findByRole("alert")).toHaveTextContent("Recovery storage is unavailable"); expect(screen.getByRole("status")).toHaveTextContent("Your saved work remains available"); expect(preview).not.toHaveBeenCalled(); await user.click(screen.getByRole("button", { name: "Preview recovery" })); expect(preview).toHaveBeenCalledTimes(1);
    await user.click(screen.getByRole("button", { name: "Check recovery now" })); expect(api.retryBackup).toHaveBeenCalledTimes(1);
  });
  it("keeps the previous verified recovery date and count visible when a newer snapshot fails", async () => {
    vi.mocked(api.backupStatus).mockResolvedValue({ state: "warning", latest_successful_day: "2026-10-20", snapshot_count: 3, warning: "The new recovery copy could not be saved." });
    render(<BackupPanel generation={id(99)} busy={false} onPreview={vi.fn()} onExport={vi.fn()} />);
    expect(await screen.findByRole("alert")).toHaveTextContent("The new recovery copy could not be saved"); expect(screen.getByText("Verified recovery copies: 3. Latest verified date: 2026-10-20.")).toBeVisible();
  });
});
