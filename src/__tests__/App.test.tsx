import { cleanup, render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import App from "../App";
import { getAppStatus } from "../platform/client";

vi.mock("../platform/client", () => ({ getAppStatus: vi.fn() }));

describe("Work desktop connection", () => {
  beforeEach(() => vi.resetAllMocks());
  afterEach(cleanup);

  it("shows the confirmed native status", async () => {
    vi.mocked(getAppStatus).mockResolvedValue({ app_name: "Ascension", schema_version: 1 });
    render(<App />);
    expect(screen.getByRole("heading", { name: "Work" })).toBeInTheDocument();
    expect(await screen.findByText("Desktop connection ready")).toBeInTheDocument();
    expect(screen.getByText("Ascension · Record schema 1")).toBeInTheDocument();
  });

  it("keeps a failed connection visible and lets the user retry by keyboard", async () => {
    vi.mocked(getAppStatus)
      .mockRejectedValueOnce(new Error("A native desktop connection is required."))
      .mockResolvedValueOnce({ app_name: "Ascension", schema_version: 1 });
    render(<App />);
    expect(await screen.findByRole("alert")).toHaveTextContent("A native desktop connection is required.");
    expect(screen.queryByText("Desktop connection ready")).not.toBeInTheDocument();
    const user = userEvent.setup();
    await user.tab();
    expect(screen.getByRole("button", { name: "Retry connection" })).toHaveFocus();
    await user.keyboard("{Enter}");
    expect(await screen.findByText("Desktop connection ready")).toBeInTheDocument();
  });
});
