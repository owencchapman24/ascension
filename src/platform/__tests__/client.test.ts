import { invoke, isTauri } from "@tauri-apps/api/core";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { getAppStatus } from "../client";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn(), isTauri: vi.fn() }));

describe("desktop command client", () => {
  beforeEach(() => vi.resetAllMocks());

  it("rejects browser-only use without reporting a native connection", async () => {
    vi.mocked(isTauri).mockReturnValue(false);
    await expect(getAppStatus()).rejects.toThrow("A native desktop connection is required.");
    expect(invoke).not.toHaveBeenCalled();
  });

  it("uses only the narrow status command", async () => {
    vi.mocked(isTauri).mockReturnValue(true);
    vi.mocked(invoke).mockResolvedValue({ app_name: "Ascension", schema_version: 1 });
    await expect(getAppStatus()).resolves.toEqual({ app_name: "Ascension", schema_version: 1 });
    expect(invoke).toHaveBeenCalledWith("get_app_status");
  });
});
