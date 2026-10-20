import { cleanup, fireEvent, render, screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { act, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import * as api from "../../platform/client";
import StudyPanel from "../StudyPanel";
import OutcomeEditor from "../OutcomeEditor";
import WorldPanel from "../WorldPanel";
import StudyReport from "../StudyReport";
import { E01_ACKNOWLEDGEMENTS } from "../StudySettings";
import { manifest, opportunity, outcome, studyWorkspace } from "./fixtures";
import { localDateTime } from "../fields";

vi.mock("../../platform/client", async (load) => ({ ...await load<typeof api>(), planOpportunity: vi.fn(), recordOpportunityOutcome: vi.fn(), transitionStudy: vi.fn(), configureStudy: vi.fn(), createScheduleEpoch: vi.fn(), recordStudyBurden: vi.fn(), recordStudyReflection: vi.fn(), recordStudyExposure: vi.fn() }));
const mutation = vi.fn(async (task: () => Promise<api.Workspace>) => { try { await task(); return true; } catch { return false; } });
const onDirtyChange = vi.fn();
async function panel(workspace = studyWorkspace) { render(<StudyPanel workspace={workspace} objective={workspace.objectives[0]} busy={false} onMutation={mutation} onDirtyChange={onDirtyChange} />); await userEvent.click(screen.getByText(/Work opportunities/)); }
describe("optional opportunity workflow", () => {
  beforeEach(() => { vi.clearAllMocks(); vi.mocked(api.planOpportunity).mockResolvedValue(studyWorkspace); vi.mocked(api.recordOpportunityOutcome).mockResolvedValue(studyWorkspace); vi.mocked(api.transitionStudy).mockResolvedValue(studyWorkspace); });
  afterEach(cleanup);
  it("starts closed and does not enroll, assign, or record a meaningful start merely by rendering", () => {
    render(<StudyPanel workspace={studyWorkspace} objective={studyWorkspace.objectives[0]} busy={false} onMutation={mutation} onDirtyChange={onDirtyChange} />);
    expect(screen.queryByRole("textbox")).not.toBeInTheDocument(); expect(api.planOpportunity).not.toHaveBeenCalled(); expect(api.transitionStudy).not.toHaveBeenCalled(); expect(api.recordOpportunityOutcome).not.toHaveBeenCalled();
  });
  it("prefills the saved action, locks prospective context, and preserves failed drafts", async () => {
    const user = userEvent.setup(); await panel(); await user.click(screen.getByRole("button", { name: "Plan an opportunity" }));
    const definition = screen.getByLabelText("What observable first action will count as a meaningful start?"); expect(definition).toHaveValue("Sketch the bed on paper"); expect(definition).toHaveFocus();
    await user.clear(definition); await user.type(definition, "Mark and name the first planting position");
    const cue = localDateTime(new Date(Date.now() + 3600_000)); fireEvent.change(screen.getByLabelText("Available work time"), { target: { value: cue } });
    vi.mocked(api.planOpportunity).mockRejectedValueOnce({ code: "busy", message: "Try again" });
    await user.click(screen.getByRole("button", { name: "Confirm opportunity" }));
    expect(definition).toHaveValue("Mark and name the first planting position");
    expect(api.planOpportunity).toHaveBeenCalledWith(expect.objectContaining({ expected_generation: studyWorkspace.generation, objective_id: opportunity.objective_id, window_seconds: 900, eligible: true, meaningful_start: "Mark and name the first planting position", manifest_id: manifest.id }));
    await user.click(screen.getByRole("button", { name: "Confirm opportunity" })); expect(screen.queryByLabelText("Available work time")).not.toBeInTheDocument();
  });
  it("allows planned rest with a prospective unavailability reason without negative outcomes", async () => {
    const user = userEvent.setup(); await panel(); await user.click(screen.getByRole("button", { name: "Plan an opportunity" }));
    await user.click(screen.getByLabelText("I have available time and intend to attempt this work."));
    await user.click(screen.getByRole("button", { name: "Confirm opportunity" })); expect(screen.getByRole("alert")).toHaveTextContent("known unavailability");
    await user.type(screen.getByLabelText("Known unavailability"), "Planned rest"); await user.click(screen.getByRole("button", { name: "Confirm opportunity" }));
    expect(api.planOpportunity).toHaveBeenCalledWith(expect.objectContaining({ eligible: false, unavailability: "Planned rest" })); expect(api.recordOpportunityOutcome).not.toHaveBeenCalled();
  });
  it("records uncertainty without manufacturing a timestamp or nonstart", async () => {
    const user = userEvent.setup(); render(<OutcomeEditor workspace={studyWorkspace} opportunity={opportunity} correcting={false} busy={false} onMutation={mutation} onClose={vi.fn()} />);
    await user.click(screen.getByRole("button", { name: "Save observation" }));
    expect(api.recordOpportunityOutcome).toHaveBeenCalledWith(expect.objectContaining({ report: "unknown", started_at: null, precision: "unknown", uncertainty_seconds: null, correction_reason: null }));
  });
  it("requires uncertainty context for a recalled start with unknown timing and keeps input on failure", async () => {
    const user = userEvent.setup(); render(<OutcomeEditor workspace={studyWorkspace} opportunity={opportunity} correcting={false} busy={false} onMutation={mutation} onClose={vi.fn()} />);
    await user.click(screen.getByLabelText("Started")); await user.click(screen.getByRole("button", { name: "Save observation" }));
    expect(api.recordOpportunityOutcome).not.toHaveBeenCalled(); const field = screen.getByLabelText(/Uncertainty or changed/); expect(field).toHaveFocus();
    await user.type(field, "I cannot place it inside the window"); await user.click(screen.getByRole("button", { name: "Save observation" }));
    expect(api.recordOpportunityOutcome).toHaveBeenCalledWith(expect.objectContaining({ report: "started", started_at: null, precision: "unknown", uncertainty_reason: "I cannot place it inside the window" }));
  });
  it("requires an explicit correction reason and retains exact late-time input", async () => {
    const user = userEvent.setup(); render(<OutcomeEditor workspace={studyWorkspace} opportunity={opportunity} correcting busy={false} onMutation={mutation} onClose={vi.fn()} />);
    await user.click(screen.getByLabelText("Started")); await user.selectOptions(screen.getByLabelText(/How precisely/), "exact");
    fireEvent.change(screen.getByLabelText("Reported meaningful-start time"), { target: { value: localDateTime(new Date("2026-01-02T12:25:00.000Z")) } });
    await user.click(screen.getByRole("button", { name: "Save observation" })); expect(api.recordOpportunityOutcome).not.toHaveBeenCalled();
    await user.type(screen.getByLabelText("Reason for this outcome correction"), "The first start was actually later"); await user.click(screen.getByRole("button", { name: "Save observation" }));
    expect(api.recordOpportunityOutcome).toHaveBeenCalledWith(expect.objectContaining({ started_at: "2026-01-02T12:25:00.000Z", correction_reason: "The first start was actually later" }));
  });
  it("offers explicit E00 beginning with no automatic activation or randomized assignment", async () => {
    const user = userEvent.setup(); const workspace = { ...studyWorkspace, study: { ...studyWorkspace.study, manifests: [{ ...manifest, status: "configured" as const }] } }; await panel(workspace);
    await user.click(screen.getByText("Schedules and optional studies")); await user.click(screen.getByRole("button", { name: "Review E00 lifecycle" })); expect(api.transitionStudy).not.toHaveBeenCalled();
    await user.click(screen.getByRole("button", { name: "Begin E00 feasibility logging" })); expect(api.transitionStudy).toHaveBeenCalledWith(expect.objectContaining({ manifest_id: manifest.id, status: "active", acknowledgements: [] }));
  });
  it("requires all owner acknowledgements and E00 assessment for explicit E01 activation", async () => {
    const user = userEvent.setup(); const workspace = { ...studyWorkspace, study: { ...studyWorkspace.study, manifests: [{ ...manifest, phase: "e01" as const, status: "ready" as const }] } }; await panel(workspace);
    await user.click(screen.getByText("Schedules and optional studies")); await user.click(screen.getByRole("button", { name: "Review E01 lifecycle" }));
    const activate = screen.getByRole("button", { name: "Activate E01 with owner approval" }); expect(activate).toBeDisabled();
    for (const [,label] of E01_ACKNOWLEDGEMENTS) await user.click(screen.getByLabelText(label)); expect(activate).toBeDisabled();
    await user.type(screen.getByLabelText("E00 feasibility assessment and protocol decision"), "Synthetic feasibility assessment for this isolated test"); expect(activate).toBeEnabled(); expect(api.transitionStudy).not.toHaveBeenCalled();
    await user.click(activate); expect(api.transitionStudy).toHaveBeenCalledWith(expect.objectContaining({ status: "active", acknowledgements: E01_ACKNOWLEDGEMENTS.map(([key]) => key) }));
  });
  it("retains the locked action and both observations in history", async () => {
    const user = userEvent.setup(); const workspace = { ...studyWorkspace, study: { ...studyWorkspace.study, outcomes: [outcome, { ...outcome, id: "correction", sequence: 2, supersedes: outcome.id, correction_reason: "Corrected recollection", state: "missing" as const }] } }; await panel(workspace);
    await user.click(screen.getByText(/Opportunity history/)); await user.click(screen.getByText("Original declaration and observation history"));
    expect(screen.getByText(/Observation 1: Observed start/)).toBeVisible(); expect(screen.getByText(/Observation 2: Missing/)).toBeVisible(); expect(screen.getByText(/Corrected recollection/)).toBeVisible();
  });
  it("keeps active arm comparisons hidden before the review checkpoint", async () => {
    const user = userEvent.setup(); render(<StudyReport study={{ ...studyWorkspace.study, manifests: [{ ...manifest, phase: "e01" }] }} />); await user.click(screen.getByText("Descriptive reports"));
    expect(screen.getByText(/Treatment comparisons stay hidden/)).toBeVisible(); expect(screen.queryByRole("table")).not.toBeInTheDocument();
  });
});

describe("construction isolation", () => {
  beforeEach(() => vi.clearAllMocks()); afterEach(() => { cleanup(); vi.unstubAllGlobals(); vi.restoreAllMocks(); });
  it("keeps fiction hidden by default and never records delivery for an off-screen scene", async () => {
    const user = userEvent.setup(); render(<WorldPanel workspace={studyWorkspace} busy={false} onMutation={mutation} />);
    expect(screen.queryByText(/issued Build Credits/)).not.toBeInTheDocument(); expect(screen.queryByRole("img")).not.toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: "View construction" })); expect(screen.getByText(/Nonexperimental exposure during E00/)).toBeVisible(); expect(api.recordStudyExposure).not.toHaveBeenCalled();
  });
  it("P5T029 matches numeric support and disallows construction in D or during active work", () => {
    const e01 = { ...manifest, phase: "e01" as const }; const base = { ...studyWorkspace, study: { ...studyWorkspace.study, manifests: [e01], presentation: { opportunity_id: opportunity.id, condition: "d" as const, boundary: "checkpoint" as const, world_allowed: false, enabled: true } } };
    const { rerender } = render(<WorldPanel workspace={base} busy={false} onMutation={mutation} />); const numeric = screen.getByText(/issued Build Credits/).textContent;
    expect(screen.queryByRole("img")).not.toBeInTheDocument(); expect(screen.queryByRole("button", { name: "View construction" })).not.toBeInTheDocument();
    rerender(<WorldPanel workspace={{ ...base, study: { ...base.study, presentation: { ...base.study.presentation, condition: "d_plus_b", world_allowed: true } } }} busy={false} onMutation={mutation} />);
    expect(screen.getByText(/issued Build Credits/).textContent).toBe(numeric); expect(screen.getByRole("img")).toBeInTheDocument();
    rerender(<WorldPanel workspace={{ ...base, study: { ...base.study, presentation: { ...base.study.presentation, condition: "d_plus_b", boundary: "none", world_allowed: false, enabled: true } } }} busy={false} onMutation={mutation} />);
    expect(screen.queryByRole("img")).not.toBeInTheDocument(); expect(api.recordStudyExposure).not.toHaveBeenCalled();
  });
  it("does not call acceptance, settlement, or opportunity-outcome commands from optional construction", async () => {
    render(<WorldPanel workspace={studyWorkspace} busy={false} onMutation={mutation} />); await userEvent.click(screen.getByRole("button", { name: "View construction" }));
    expect(api.recordOpportunityOutcome).not.toHaveBeenCalled(); expect(api.planOpportunity).not.toHaveBeenCalled(); expect(within(screen.getByRole("region", { name: "Optional construction" })).queryByRole("button", { name: /accept/i })).not.toBeInTheDocument();
  });
  it("records delivery only after viewport visibility, and never infers that it was seen", async () => {
    let intersect: IntersectionObserverCallback = () => {};
    vi.stubGlobal("IntersectionObserver", class { constructor(callback: IntersectionObserverCallback) { intersect = callback; } observe() {} disconnect() {} });
    vi.spyOn(HTMLElement.prototype, "getBoundingClientRect").mockReturnValue({ width: 500, height: 300, top: 10, bottom: 310, left: 0, right: 500, x: 0, y: 10, toJSON: () => ({}) });
    vi.mocked(api.recordStudyExposure).mockResolvedValue(studyWorkspace);
    render(<WorldPanel workspace={studyWorkspace} busy={false} onMutation={mutation} />); await userEvent.click(screen.getByRole("button", { name: "View construction" }));
    expect(api.recordStudyExposure).not.toHaveBeenCalled();
    const modal = document.createElement("div"); modal.setAttribute("aria-modal", "true"); document.body.append(modal);
    await act(async () => { intersect([{ isIntersecting: true } as IntersectionObserverEntry], {} as IntersectionObserver); });
    expect(api.recordStudyExposure).not.toHaveBeenCalled();
    await act(async () => { modal.remove(); });
    await waitFor(() => expect(api.recordStudyExposure).toHaveBeenCalledWith(expect.objectContaining({ kind: "voluntary_world", opportunity_id: null })));
    expect(api.recordStudyExposure).toHaveBeenCalledTimes(1); expect(api.recordStudyExposure).not.toHaveBeenCalledWith(expect.objectContaining({ kind: "seen" }));
  });
});
