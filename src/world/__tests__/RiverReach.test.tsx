import { act, cleanup, render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import * as api from "../../platform/client";
import { id, progressWorkspace, timestamp } from "../../progress/__tests__/fixtures";
import RiverReach from "../RiverReach";
import type { WorldView } from "../types";
import { commandResult, completeWorld, fundedWorld, queuedWorld, validPreview, world } from "./fixtures";

vi.mock("../../platform/client", async importOriginal => ({ ...await importOriginal<typeof api>(), loadWorkspace: vi.fn(), loadWorld: vi.fn(), previewWorld: vi.fn(), worldCommand: vi.fn() }));
const onWorkspace = vi.fn(), onDirtyChange = vi.fn(), onPendingChange = vi.fn(), onModalChange = vi.fn(), onReturnToWork = vi.fn(), onChronicle = vi.fn();
async function open(value: WorldView = world, chronicle = false) {
  vi.mocked(api.loadWorld).mockResolvedValue(value);
  render(<RiverReach workspace={progressWorkspace} active chronicle={chronicle} busy={false} onWorkspace={onWorkspace} onDirtyChange={onDirtyChange} onPendingChange={onPendingChange} onModalChange={onModalChange} onReturnToWork={onReturnToWork} onChronicle={onChronicle} />);
  await screen.findByRole("heading", { name: chronicle ? "Chronicle" : "River Reach" });
  await waitFor(() => expect(api.loadWorld).toHaveBeenCalled());
  if (value.chapter && !chronicle) await screen.findByRole("heading", { name: value.chapter.completion ? value.chapter.sandbox_alternatives > 0 ? "Saved sandbox province" : "Completed province" : "Your province" });
}

describe("River Reach voluntary strategy interface", () => {
  beforeEach(() => { vi.clearAllMocks(); vi.mocked(api.previewWorld).mockResolvedValue(validPreview); vi.mocked(api.worldCommand).mockResolvedValue({ workspace: progressWorkspace, result: commandResult }); });
  afterEach(cleanup);

  it("enters a free initial camp only after an explicit action", async () => {
    await open({ ...world, chapter: null });
    expect(api.worldCommand).not.toHaveBeenCalled();
    expect(screen.getByRole("img")).toHaveAccessibleName(/six parcels/);
    await userEvent.click(screen.getByRole("button", { name: "Enter River Reach" }));
    expect(api.worldCommand).toHaveBeenCalledWith(expect.objectContaining({ chapter_id: null, expected_world_revision: 0, operation: { kind: "initialize" } }));
  });

  it("withholds a previously loaded province when the backend reports an active study on focus", async () => {
    await open(fundedWorld); expect(screen.getByRole("img")).toBeVisible();
    vi.mocked(api.loadWorkspace).mockResolvedValue(progressWorkspace);
    vi.mocked(api.loadWorld).mockRejectedValueOnce({ code: "study_active", message: "Strategic World is withheld during an active study." });
    await act(async () => { window.dispatchEvent(new Event("focus")); });
    expect(await screen.findByRole("alert")).toHaveTextContent("withheld"); expect(screen.queryByRole("img")).not.toBeInTheDocument();
    expect(screen.queryByRole("heading", { name: "Buildings of River Reach" })).not.toBeInTheDocument(); expect(onWorkspace).toHaveBeenCalledWith(progressWorkspace);
  });

  it("does not restore a stale strategic view when an older post-command read finishes after study withholding", async () => {
    const user = userEvent.setup(); await open(queuedWorld);
    let finish!: (value: WorldView) => void;
    vi.mocked(api.loadWorld).mockImplementationOnce(() => new Promise(resolve => { finish = resolve; }));
    await user.click(screen.getByRole("button", { name: "Complete construction" })); await user.click(screen.getByRole("button", { name: "Confirm construction and spend" }));
    await waitFor(() => expect(finish).toBeDefined());
    vi.mocked(api.loadWorkspace).mockResolvedValue(progressWorkspace);
    vi.mocked(api.loadWorld).mockRejectedValueOnce({ code: "study_active", message: "Strategic World is withheld during an active study." });
    await act(async () => { window.dispatchEvent(new Event("focus")); });
    expect(await screen.findByRole("alert")).toHaveTextContent("withheld");
    await act(async () => finish(fundedWorld));
    expect(screen.queryByRole("img")).not.toBeInTheDocument(); expect(screen.queryByText("Construction is complete. Funding and province are saved.")).not.toBeInTheDocument();
  });

  it("supports exact parcel selection by keyboard and sends no frontend prices or capabilities", async () => {
    const user = userEvent.setup(); await open();
    const a1 = screen.getByRole("button", { name: "A1" }); a1.focus(); await user.keyboard("{Enter}");
    await waitFor(() => expect(screen.getByRole("button", { name: "Queue construction" })).toBeEnabled());
    expect(a1).toHaveAttribute("aria-pressed", "true");
    await user.click(screen.getByRole("button", { name: "Queue construction" }));
    expect(api.worldCommand).toHaveBeenCalledWith(expect.objectContaining({ operation: { kind: "queue", building_type: "survey_lodge", parcels: ["a1"], reserve_available: true } }));
    expect(vi.mocked(api.worldCommand).mock.calls[0][0].operation).not.toHaveProperty("cost_bc");
    expect(vi.mocked(api.worldCommand).mock.calls[0][0].operation).not.toHaveProperty("capabilities");
  });

  it("rejects an authoritative invalid placement without a queue mutation", async () => {
    const user = userEvent.setup(); await open();
    vi.mocked(api.previewWorld).mockResolvedValue({ ...validPreview, legal: false, placement_legal: false, errors: ["Specialist parcels must share an orthogonal edge."] });
    await user.selectOptions(screen.getByLabelText("Building"), "foundry");
    await user.click(screen.getByRole("button", { name: "A1" })); await user.click(screen.getByRole("button", { name: "B2" }));
    expect(await screen.findByText("Specialist parcels must share an orthogonal edge.")).toBeVisible();
    expect(screen.getByRole("button", { name: "Queue construction" })).toBeDisabled(); expect(api.worldCommand).not.toHaveBeenCalled();
  });

  it("permits a legal construction preview requiring new funding but not free reconfiguration", async () => {
    const user = userEvent.setup(); await open();
    vi.mocked(api.previewWorld).mockResolvedValue({ ...validPreview, legal: false, additional_sites_needed: 1, occupied_sites: 1, errors: ["Additional funding is required."] });
    await user.click(screen.getByRole("button", { name: "A1" }));
    await waitFor(() => expect(screen.getByRole("button", { name: "Queue construction" })).toBeEnabled());
    await user.click(screen.getByRole("button", { name: "Reconfigure" }));
    await user.click(screen.getByRole("button", { name: "A1" })); await user.click(screen.getByRole("button", { name: "Add to layout preview" }));
    expect(await screen.findByText(/preview exceeds funded capacity/)).toBeVisible(); expect(screen.getByRole("button", { name: "Save free layout" })).toBeDisabled();
  });

  it("requires an explicit spend confirmation and contains keyboard focus", async () => {
    const user = userEvent.setup(); await open(queuedWorld);
    await user.click(screen.getByRole("button", { name: "Complete construction" }));
    const dialog = screen.getByRole("dialog", { name: "Complete this construction?" });
    expect(api.worldCommand).not.toHaveBeenCalled(); expect(dialog).toHaveTextContent("4 BC");
    expect(onModalChange).toHaveBeenLastCalledWith(true);
    const cancel = within(dialog).getByRole("button", { name: "Keep planning" }); expect(cancel).toHaveFocus();
    await user.keyboard("{Shift>}{Tab}{/Shift}"); expect(within(dialog).getByRole("button", { name: "Confirm construction and spend" })).toHaveFocus();
    await user.keyboard("{Escape}"); expect(screen.queryByRole("dialog")).not.toBeInTheDocument(); expect(api.worldCommand).not.toHaveBeenCalled();
    await user.click(screen.getByRole("button", { name: "Complete construction" })); await user.click(screen.getByRole("button", { name: "Confirm construction and spend" }));
    expect(api.worldCommand).toHaveBeenCalledWith(expect.objectContaining({ operation: { kind: "complete_construction" } }));
  });

  it("shows partial specialist reservation honestly and supports cancellation", async () => {
    const partial = { ...queuedWorld, chapter: { ...queuedWorld.chapter!, queue: { ...queuedWorld.chapter!.queue!, building_type: "foundry" as const, parcels: ["a1" as const, "b1" as const] } } };
    const user = userEvent.setup(); await open(partial);
    expect(screen.getByText(/4 \/ 8 BC reserved/)).toBeVisible(); expect(screen.getByRole("button", { name: "Complete construction" })).toBeDisabled();
    await user.click(screen.getByRole("button", { name: "Cancel blueprint" }));
    expect(api.worldCommand).toHaveBeenCalledWith(expect.objectContaining({ operation: { kind: "cancel_queue" } }));
  });

  it("explains reusable funded capacity and the exact additional cost of a saved blueprint", async () => {
    const reused: WorldView = { ...fundedWorld, bank: { issued_bc: 18, spent_bc: 8, reserved_bc: 4, available_bc: 6 }, chapter: { ...fundedWorld.chapter!, funded_site_count: 2, queue: { ...queuedWorld.chapter!.queue!, building_type: "foundry", parcels: ["a2", "b2"] } } };
    await open(reused);
    const queue = screen.getByRole("region", { name: "Foundry" });
    expect(within(queue).getByText("Total building capacity").nextElementSibling).toHaveTextContent("8 BC");
    expect(within(queue).getByText("Previously funded capacity usable").nextElementSibling).toHaveTextContent("1 site");
    expect(within(queue).getByText("Additional funded capacity needed").nextElementSibling).toHaveTextContent("1 site · 4 BC");
    expect(within(queue).getByText("Current affordability").nextElementSibling).toHaveTextContent("Fully funded");
    expect(within(queue).getByText("Globally available BC").nextElementSibling).toHaveTextContent("6 BC");
    expect(within(queue).getByText("Chapter spent BC").nextElementSibling).toHaveTextContent("8 BC");
    expect(within(queue).getByText("Placement status").nextElementSibling).toHaveTextContent("Legal saved blueprint");
    expect(screen.getByRole("button", { name: "Complete construction" })).toBeEnabled();
  });

  it("replaces a blueprint with one explicit atomic operation", async () => {
    const user = userEvent.setup(); await open(queuedWorld);
    await user.selectOptions(screen.getByLabelText("Building"), "observatory"); await user.click(screen.getByRole("button", { name: "B2" }));
    await waitFor(() => expect(screen.getByRole("button", { name: "Replace queued blueprint" })).toBeEnabled());
    await user.click(screen.getByRole("button", { name: "Replace queued blueprint" }));
    expect(api.worldCommand).toHaveBeenCalledTimes(1); expect(api.worldCommand).toHaveBeenCalledWith(expect.objectContaining({ operation: { kind: "replace_queue", building_type: "observatory", parcels: ["b2"], reserve_available: true } }));
  });

  it("moves an existing building in a draft with stable identity and commits only on Save", async () => {
    const user = userEvent.setup(); await open(fundedWorld);
    await user.click(screen.getByRole("button", { name: "Reconfigure" })); await user.click(screen.getByRole("button", { name: "Move or turn Survey Lodge 1" }));
    await user.click(screen.getByRole("button", { name: "B2" })); await user.click(screen.getByRole("button", { name: "Update draft placement" }));
    await waitFor(() => expect(screen.getByRole("button", { name: "Save free layout" })).toBeEnabled());
    expect(api.worldCommand).not.toHaveBeenCalled(); expect(onDirtyChange).toHaveBeenLastCalledWith(true);
    await user.click(screen.getByRole("button", { name: "Save free layout" }));
    expect(api.worldCommand).toHaveBeenCalledWith(expect.objectContaining({ operation: { kind: "reconfigure", placements: [{ ...fundedWorld.chapter!.layout[0], parcels: ["b2"] }] } }));
  });

  it("retains a failed layout draft and retries the exact command identity", async () => {
    const user = userEvent.setup(); await open(fundedWorld);
    await user.click(screen.getByRole("button", { name: "Reconfigure" })); await user.click(screen.getByRole("button", { name: "Remove Survey Lodge 1" }));
    vi.mocked(api.worldCommand).mockRejectedValueOnce({ code: "connection_error", message: "The command response was interrupted." });
    await waitFor(() => expect(screen.getByRole("button", { name: "Save free layout" })).toBeEnabled()); await user.click(screen.getByRole("button", { name: "Save free layout" }));
    expect(await screen.findByRole("alert")).toHaveTextContent("interrupted"); expect(screen.getByText("Unsaved layout")).toBeVisible();
    const first = vi.mocked(api.worldCommand).mock.calls[0][0]; await user.click(screen.getByRole("button", { name: "Retry exact request" }));
    expect(vi.mocked(api.worldCommand).mock.calls[1][0]).toEqual(first);
  });

  it("never labels an invalid overlapping draft as a satisfied route", async () => {
    const user = userEvent.setup(); await open(completeWorld);
    vi.mocked(api.previewWorld).mockResolvedValue({ ...validPreview, legal: false, placement_legal: false, capabilities: completeWorld.capabilities, errors: ["Parcel overlap."], routes: validPreview.routes.map(route => ({ ...route, satisfied: false })) });
    await user.click(screen.getByRole("button", { name: "Reconfigure" }));
    await user.click(screen.getByRole("button", { name: "A1" })); await user.click(screen.getByRole("button", { name: "Add to layout preview" }));
    expect(await screen.findByText("Parcel overlap.")).toBeVisible(); expect(screen.queryByText("Satisfied in preview")).not.toBeInTheDocument(); expect(screen.getByRole("button", { name: "Save free layout" })).toBeDisabled();
  });

  it("reviews the saved route and exact layout before committing the first ending", async () => {
    const user = userEvent.setup(); await open(completeWorld);
    await user.click(screen.getByRole("button", { name: "Review chapter ending" }));
    const dialog = screen.getByRole("dialog", { name: "Preserve this chapter ending?" });
    expect(dialog).toHaveTextContent("6 funded sites"); expect(dialog).toHaveTextContent("24 BC invested"); expect(dialog).toHaveTextContent("A1 + B1"); expect(api.worldCommand).not.toHaveBeenCalled();
    await user.click(within(dialog).getByRole("button", { name: "Commit chapter ending" }));
    expect(api.worldCommand).toHaveBeenCalledWith(expect.objectContaining({ operation: { kind: "complete_chapter" } }));
  });

  it("shows the immutable original Chronicle after a different sandbox layout", async () => {
    const ending = { id: id(220), chapter_id: completeWorld.chapter!.id, content_version: "river-reach-v1", route: "stone_causeway" as const, layout: completeWorld.chapter!.layout, capabilities: completeWorld.capabilities, funded_site_count: 6, spent_bc: 24, completed_at: timestamp, command_id: id(221), narrative: "The first stone arches join the river banks." };
    await open({ ...fundedWorld, chapter: { ...fundedWorld.chapter!, completion: ending, sandbox_alternatives: 2 } }, true);
    expect(screen.getByText(ending.narrative)).toBeVisible(); expect(screen.getByText(/2 later sandbox layout changes/)).toBeVisible();
    expect(screen.getByRole("img")).toHaveAccessibleName(/3 buildings/); expect(screen.getByLabelText("Stone causeway arches")).toBeInTheDocument();
    expect(screen.queryByRole("button", { name: "Commit chapter ending" })).not.toBeInTheDocument(); expect(api.worldCommand).not.toHaveBeenCalled();
  });

  it("continues labeling a saved sandbox province separately from its original ending", async () => {
    const ending = { id: id(220), chapter_id: completeWorld.chapter!.id, content_version: "river-reach-v1", route: "stone_causeway" as const, layout: completeWorld.chapter!.layout, capabilities: completeWorld.capabilities, funded_site_count: 6, spent_bc: 24, completed_at: timestamp, command_id: id(221), narrative: "The first stone arches join the river banks." };
    await open({ ...fundedWorld, chapter: { ...fundedWorld.chapter!, completion: ending, sandbox_alternatives: 2 } });
    expect(screen.getByRole("heading", { name: "Saved sandbox province" })).toBeVisible();
    expect(screen.getByText(/saved sandbox layout is separate from the original Stone Causeway ending/)).toBeVisible();
    expect(screen.getByRole("button", { name: "View original Chronicle ending" })).toBeEnabled();
    expect(screen.queryByRole("heading", { name: "Your province" })).not.toBeInTheDocument();
  });
});
