import { act, cleanup, render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import App from "../App";
import * as api from "../platform/client";
import type { ImportPreview, Workspace } from "../platform/client";

vi.mock("../platform/client", async (importOriginal) => {
  const original = await importOriginal<typeof api>();
  return { ...original, loadWorkspace: vi.fn(), createProject: vi.fn(), updateProject: vi.fn(), createObjective: vi.fn(), updateObjective: vi.fn(), selectObjective: vi.fn(), saveWork: vi.fn(), exportWorkspace: vi.fn(), previewImport: vi.fn(), restoreImport: vi.fn() };
});

const projectId = "00000000-0000-4000-8000-000000000001";
const objectiveId = "00000000-0000-4000-8000-000000000002";
const secondId = "00000000-0000-4000-8000-000000000005";
const generation = "00000000-0000-4000-8000-000000000010";
const metadata = { revision: 1, created_at: "2026-10-16T12:00:00.000Z", updated_at: "2026-10-16T12:00:00.000Z" };
const saved: Workspace = {
  generation, selected_objective_id: objectiveId,
  projects: [{ ...metadata, id: projectId, title: "Sample garden" }],
  objectives: [{ ...metadata, id: objectiveId, project_id: projectId, title: "Plan a small herb bed", criterion: "A diagram names three herbs and their positions." }, { ...metadata, id: secondId, project_id: projectId, title: "List supplies", criterion: "A list includes soil and containers." }],
  actions: [{ ...metadata, id: "00000000-0000-4000-8000-000000000003", objective_id: objectiveId, description: "Sketch the bed on paper", stop_condition: "Stop after marking three planting spots." }],
  notes: [{ ...metadata, id: "00000000-0000-4000-8000-000000000004", objective_id: objectiveId, text: "Compare the sunny corner next time." }],
};
const incoming: ImportPreview = { token: "00000000-0000-4000-8000-000000000020", generation, file_name: "sample-records.json", current_counts: { projects: 1, objectives: 2, actions: 1, notes: 1 }, incoming_counts: { projects: 2, objectives: 3, actions: 2, notes: 0 } };

async function ready() { render(<App />); await screen.findByRole("heading", { name: "Work" }); }
async function editAction(user: ReturnType<typeof userEvent.setup>) {
  await user.click(screen.getByRole("button", { name: "Edit next action and note" }));
  return screen.getByLabelText("Next action: what will you do?");
}

describe("Work", () => {
  beforeEach(() => { vi.resetAllMocks(); vi.mocked(api.loadWorkspace).mockResolvedValue(structuredClone(saved)); });
  afterEach(cleanup);

  it("immediately presents the saved objective, action, stop and note on launch", async () => {
    await ready();
    expect(screen.getByRole("heading", { name: "Sketch the bed on paper" })).toBeInTheDocument();
    expect(screen.getByText("Stop after marking three planting spots.")).toBeInTheDocument();
    expect(screen.getByText("Compare the sunny corner next time.")).toBeInTheDocument();
    expect(screen.getByText("A diagram names three herbs and their positions.")).toBeInTheDocument();
    expect(screen.getByLabelText("Parent objective")).toHaveValue(objectiveId);
    expect(screen.queryByRole("textbox")).not.toBeInTheDocument();
    const context = screen.getByText("Objective and criterion").closest("details");
    expect(context).not.toHaveAttribute("open");
    expect(screen.getByRole("heading", { name: "Sketch the bed on paper" }).compareDocumentPosition(context!))
      .toBe(Node.DOCUMENT_POSITION_FOLLOWING);
  });

  it("edits project and objective fields without changing their relationships", async () => {
    vi.mocked(api.updateProject).mockResolvedValue(saved);
    vi.mocked(api.updateObjective).mockResolvedValue(saved);
    const user = userEvent.setup({ delay: null }); await ready();
    await user.click(screen.getByRole("button", { name: "Edit project" }));
    await user.clear(screen.getByLabelText("Project name"));
    await user.paste("Sample planting plan");
    await user.click(screen.getByRole("button", { name: "Save project" }));
    expect(api.updateProject).toHaveBeenCalledWith({ expected_generation: generation, id: projectId, title: "Sample planting plan" });
    await user.click(screen.getByRole("button", { name: "Edit objective" }));
    await user.clear(screen.getByLabelText(/Criterion:/));
    await user.click(screen.getByRole("button", { name: "Save objective" }));
    expect(await screen.findByRole("alert")).toHaveTextContent("Criterion needs a nonempty value.");
    expect(api.updateObjective).not.toHaveBeenCalled();
    await user.paste("The diagram has three labels.");
    await user.click(screen.getByRole("button", { name: "Save objective" }));
    expect(api.updateObjective).toHaveBeenCalledWith({ expected_generation: generation, id: objectiveId, title: saved.objectives[0].title, criterion: "The diagram has three labels." });
  });

  it("creates a project and then links a bounded objective with an explicit criterion", async () => {
    const empty: Workspace = { ...saved, selected_objective_id: null, projects: [], objectives: [], actions: [], notes: [] };
    const withProject = { ...empty, generation: "project-generation", projects: saved.projects };
    const withObjective = { ...withProject, selected_objective_id: objectiveId, objectives: [saved.objectives[0]] };
    vi.mocked(api.loadWorkspace).mockResolvedValue(empty);
    vi.mocked(api.createProject).mockResolvedValue(withProject);
    vi.mocked(api.createObjective).mockResolvedValue(withObjective);
    const user = userEvent.setup({ delay: null }); await ready();
    await user.click(screen.getByRole("button", { name: "Create your first project" }));
    await user.type(screen.getByLabelText("Project name"), "Sample garden");
    await user.click(screen.getByRole("button", { name: "Save project" }));
    expect(api.createProject).toHaveBeenCalledWith({ expected_generation: generation, title: "Sample garden" });
    await user.click(screen.getByRole("button", { name: "New objective" }));
    await user.type(screen.getByLabelText("Bounded objective"), "Plan a small herb bed");
    await user.type(screen.getByLabelText(/Criterion:/), "A diagram names three herbs and their positions.");
    await user.click(screen.getByRole("button", { name: "Save objective" }));
    expect(api.createObjective).toHaveBeenCalledWith({ expected_generation: "project-generation", project_id: projectId, title: "Plan a small herb bed", criterion: "A diagram names three herbs and their positions." });
    expect(screen.getByLabelText("Parent objective")).toHaveValue(objectiveId);
    expect(screen.getByRole("button", { name: "Add next action" })).toBeInTheDocument();
  });

  it("rejects whitespace-only required input without calling the backend", async () => {
    const user = userEvent.setup({ delay: null }); await ready();
    const input = await editAction(user); await user.clear(input); await user.type(input, "   ");
    await user.click(screen.getByRole("button", { name: "Save next action and note" }));
    expect(await screen.findByRole("alert")).toHaveTextContent("Next action needs a nonempty value.");
    expect(input).toHaveFocus(); expect(api.saveWork).not.toHaveBeenCalled();
  });

  it("counts Unicode scalars and preserves accepted text instead of truncating it", async () => {
    const user = userEvent.setup({ delay: null }); await ready();
    await user.click(screen.getByRole("button", { name: "New project" }));
    const title = "🌱".repeat(120);
    await user.click(screen.getByLabelText("Project name")); await user.paste(title);
    vi.mocked(api.createProject).mockResolvedValue(saved);
    await user.click(screen.getByRole("button", { name: "Save project" }));
    expect(api.createProject).toHaveBeenCalledWith({ expected_generation: generation, title });
  });

  it("preserves edits and the previously saved action when a save fails", async () => {
    vi.mocked(api.saveWork).mockRejectedValue({ code: "storage_error", message: "The records could not be saved." });
    const user = userEvent.setup({ delay: null }); await ready();
    const input = await editAction(user); await user.clear(input); await user.type(input, "Draw the sunny corner");
    await user.click(screen.getByRole("button", { name: "Save next action and note" }));
    expect(await screen.findByRole("alert")).toHaveTextContent("The records could not be saved.");
    expect(input).toHaveValue("Draw the sunny corner");
    expect(screen.getByRole("heading", { name: "Sketch the bed on paper" })).toBeInTheDocument();
    expect(screen.queryByText("Next action and note saved on this device.")).not.toBeInTheDocument();
  });

  it("announces a save only after the backend commits and prevents duplicate submission", async () => {
    let resolve!: (value: Workspace) => void;
    vi.mocked(api.saveWork).mockReturnValue(new Promise((done) => { resolve = done; }));
    const user = userEvent.setup({ delay: null }); await ready();
    const input = await editAction(user); await user.clear(input); await user.type(input, "Draw the sunny corner");
    await user.clear(screen.getByLabelText(/Resumption note/));
    await user.click(screen.getByRole("button", { name: "Save next action and note" }));
    expect(screen.getByRole("button", { name: "Saving..." })).toBeDisabled();
    expect(screen.queryByText("Next action and note saved on this device.")).not.toBeInTheDocument();
    expect(screen.getByRole("heading", { name: "Sketch the bed on paper" })).toBeInTheDocument();
    expect(api.saveWork).toHaveBeenCalledOnce();
    expect(api.saveWork).toHaveBeenCalledWith({ expected_generation: generation, objective_id: objectiveId, description: "Draw the sunny corner", stop_condition: saved.actions[0].stop_condition, note: null });
    await act(async () => resolve({ ...saved, generation: "new-generation", actions: [{ ...saved.actions[0], description: "Draw the sunny corner", revision: 2 }], notes: [] }));
    expect(await screen.findByText("Next action and note saved on this device.")).toBeInTheDocument();
    expect(screen.getByRole("heading", { name: "Draw the sunny corner" })).toBeInTheDocument();
    expect(screen.getByText("No resumption note saved.")).toBeInTheDocument();
  });

  it("guards objective switches and keeps selection authoritative until confirmed", async () => {
    vi.mocked(api.selectObjective).mockResolvedValue({ ...saved, selected_objective_id: secondId });
    const user = userEvent.setup({ delay: null }); await ready(); const input = await editAction(user); await user.type(input, " draft");
    await user.selectOptions(screen.getByLabelText("Parent objective"), secondId);
    const dialog = await screen.findByRole("dialog", { name: "Keep your unsaved draft?" });
    expect(api.selectObjective).not.toHaveBeenCalled();
    expect(screen.getByLabelText("Parent objective")).toHaveValue(objectiveId);
    expect(within(dialog).getByRole("button", { name: "Keep editing" })).toHaveFocus();
    await user.keyboard("{Escape}"); expect(input).toHaveValue("Sketch the bed on paper draft");
    await user.selectOptions(screen.getByLabelText("Parent objective"), secondId);
    await user.click(screen.getByRole("button", { name: "Discard draft and continue" }));
    expect(api.selectObjective).toHaveBeenCalledWith({ expected_generation: generation, id: secondId });
    await waitFor(() => expect(screen.getByLabelText("Parent objective")).toHaveValue(secondId));
    expect(screen.queryByRole("textbox")).not.toBeInTheDocument();
  });

  it("keeps a conflict draft until the user explicitly reloads and discards it", async () => {
    vi.mocked(api.saveWork).mockRejectedValue({ code: "conflict", message: "The local workspace changed." });
    const user = userEvent.setup({ delay: null }); await ready(); const input = await editAction(user); await user.type(input, " draft");
    await user.click(screen.getByRole("button", { name: "Save next action and note" }));
    expect(await screen.findByRole("alert")).toHaveTextContent("The local workspace changed.");
    await user.click(screen.getAllByRole("button", { name: "Reload saved records" })[0]);
    await user.click(screen.getByRole("button", { name: "Keep editing" }));
    expect(api.loadWorkspace).toHaveBeenCalledOnce(); expect(input).toHaveValue("Sketch the bed on paper draft");
    await user.click(screen.getAllByRole("button", { name: "Reload saved records" })[0]);
    await user.click(screen.getByRole("button", { name: "Discard draft and continue" }));
    expect(await screen.findByText("Saved records reloaded.")).toBeInTheDocument();
    expect(api.loadWorkspace).toHaveBeenCalledTimes(2);
  });

  it("previews both count sets and allows cancellation without restoring", async () => {
    vi.mocked(api.previewImport).mockResolvedValue(incoming);
    const user = userEvent.setup({ delay: null }); await ready(); await user.click(screen.getByRole("button", { name: "Import records" }));
    const dialog = await screen.findByRole("dialog", { name: "Review import" });
    expect(within(dialog).getByText("sample-records.json")).toBeInTheDocument();
    expect(within(dialog).getByRole("row", { name: "Objectives 2 3" })).toBeInTheDocument();
    expect(within(dialog).getByText(/replaces all local records/)).toBeInTheDocument();
    expect(api.restoreImport).not.toHaveBeenCalled();
    await user.click(within(dialog).getByRole("button", { name: "Cancel import" }));
    expect(screen.queryByRole("dialog")).not.toBeInTheDocument(); expect(api.restoreImport).not.toHaveBeenCalled();
    expect(screen.getByRole("heading", { name: "Sketch the bed on paper" })).toBeInTheDocument();
  });

  it("requires explicit replacement and guards a dirty draft before restore", async () => {
    vi.mocked(api.previewImport).mockResolvedValue(incoming);
    vi.mocked(api.restoreImport).mockResolvedValue({ workspace: { ...saved, selected_objective_id: secondId }, backup_created: true, unchanged: false });
    const user = userEvent.setup({ delay: null }); await ready(); const input = await editAction(user); await user.type(input, " draft");
    await user.click(screen.getByRole("button", { name: "Import records" }));
    await user.click(await screen.findByRole("button", { name: "Replace local records" }));
    expect(api.restoreImport).not.toHaveBeenCalled(); await user.click(screen.getByRole("button", { name: "Keep editing" }));
    expect(input).toHaveValue("Sketch the bed on paper draft");
    expect(screen.queryByRole("dialog")).not.toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: "Import records" }));
    await user.click(screen.getByRole("button", { name: "Replace local records" }));
    await user.click(screen.getByRole("button", { name: "Discard draft and continue" }));
    expect(api.restoreImport).toHaveBeenCalledWith({ token: incoming.token, expected_generation: incoming.generation });
    expect(await screen.findByText(/A backup of the previous local records was created/)).toBeInTheDocument();
    expect(screen.getByLabelText("Parent objective")).toHaveValue(secondId);
  });

  it("treats canceled native file dialogs as cancellation", async () => {
    vi.mocked(api.exportWorkspace).mockResolvedValue(null); vi.mocked(api.previewImport).mockResolvedValue(null);
    const user = userEvent.setup({ delay: null }); await ready(); await user.click(screen.getByRole("button", { name: "Export records" }));
    await user.click(screen.getByRole("button", { name: "Import records" }));
    expect(screen.queryByRole("dialog")).not.toBeInTheDocument(); expect(screen.queryByText(/exported to/)).not.toBeInTheDocument();
    expect(api.restoreImport).not.toHaveBeenCalled();
  });

  it("keeps local records and the draft intact when a confirmed restore fails", async () => {
    vi.mocked(api.previewImport).mockResolvedValue(incoming);
    vi.mocked(api.restoreImport).mockRejectedValue({ code: "backup_failed", message: "A backup could not be created. No records were replaced." });
    const user = userEvent.setup({ delay: null }); await ready();
    const input = await editAction(user); await user.type(input, " draft");
    await user.click(screen.getByRole("button", { name: "Import records" }));
    await user.click(await screen.findByRole("button", { name: "Replace local records" }));
    await user.click(screen.getByRole("button", { name: "Discard draft and continue" }));
    expect(await screen.findByRole("alert")).toHaveTextContent("No records were replaced.");
    await user.click(screen.getByRole("button", { name: "Cancel import" }));
    expect(input).toHaveValue("Sketch the bed on paper draft");
    expect(screen.getByRole("heading", { name: "Sketch the bed on paper" })).toBeInTheDocument();
  });

  it("shows a native connection failure and supports retry by keyboard", async () => {
    vi.mocked(api.loadWorkspace).mockRejectedValueOnce(new Error("A native desktop connection is required."));
    const user = userEvent.setup({ delay: null }); render(<App />);
    expect(await screen.findByRole("alert")).toHaveTextContent("A native desktop connection is required.");
    await user.tab(); expect(screen.getByRole("button", { name: "Retry connection" })).toHaveFocus();
    await user.keyboard("{Enter}"); expect(await screen.findByRole("heading", { name: "Work" })).toBeInTheDocument();
  });
});
