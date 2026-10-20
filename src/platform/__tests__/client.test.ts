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
});
