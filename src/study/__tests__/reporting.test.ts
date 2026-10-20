import { describe, expect, it } from "vitest";
import type { StudyView } from "../../platform/client";
import { assignedSummary, differenceBounds, lowPressure, median, reviewScope, summarize } from "../reporting";
import { epoch, manifest, opportunity, outcome, studyWorkspace } from "./fixtures";

describe("honest opportunity descriptions", () => {
  it("keeps pending, missing, nonstarts and late starts distinct without inventing latency", () => {
    const opportunities = [opportunity, { ...opportunity, id: "nonstart", outcome_state: "observed_nonstart" as const }, { ...opportunity, id: "pending", outcome_state: "pending" as const }, { ...opportunity, id: "rest", eligible: false, unavailability: "Planned rest" }];
    const study = { ...studyWorkspace.study, opportunities, outcomes: [{ ...outcome, opportunity_id: "nonstart", state: "observed_nonstart" as const, latency_seconds: null, censored_at_seconds: 900, late_start: true }] };
    const result = summarize(study, opportunities);
    expect(result).toMatchObject({ planned: 4, eligible: 3, ineligible: 1, starts: 0, nonstarts: 1, missing: 1, pending: 1, late: 1, medianExactLatencySeconds: null, upper: null });
  });
  it("uses the latest correction while retaining technical-failure and correction counts", () => {
    const study = { ...studyWorkspace.study, opportunities: [{ ...opportunity, outcome_state: "observed_nonstart" as const }], outcomes: [{ ...outcome, technical_failure: true }, { ...outcome, id: "corrected", sequence: 2, state: "observed_nonstart" as const, latency_seconds: null, censored_at_seconds: 900, supersedes: outcome.id, correction_reason: "Timing was later", late_start: true }] };
    expect(summarize(study, study.opportunities)).toMatchObject({ starts: 0, nonstarts: 1, late: 1, technical: 1, corrections: 1, medianExactLatencySeconds: null });
  });
  it("keeps all assigned opportunities in bounds and computes opposing-arm extremes", () => {
    const e01 = { ...manifest, phase: "e01" as const, status: "completed" as const };
    const opportunities = Array.from({ length: 4 }, (_, index) => ({ ...opportunity, id: `o${index}`, outcome_state: index === 0 ? "observed_start" as const : "missing" as const }));
    const study: StudyView = { ...studyWorkspace.study, manifests: [e01], opportunities, assignments: opportunities.map((item, index) => ({ id: `a${index}`, opportunity_id: item.id, manifest_id: e01.id, position: index, condition: index < 2 ? "d" : "d_plus_b", assigned_at: opportunity.created_at, issued_bc_at_assignment: 0, h1_saturated: false, ledger_sequence: 0 })) };
    const scope = reviewScope(study, e01);
    const conventional = assignedSummary(study, e01, "d", epoch.id, scope); const plus = assignedSummary(study, e01, "d_plus_b", epoch.id, scope);
    expect(conventional).toMatchObject({ eligible: 2, starts: 1, missing: 1, lower: .5, upper: 1, deliveryUnrecorded: 2 });
    expect(plus).toMatchObject({ eligible: 2, starts: 0, missing: 2, lower: 0, upper: 1 });
    expect(differenceBounds(plus, conventional)).toEqual({ lower: -1, upper: .5 });
  });
  it("does not expose treatment comparisons before planned review or unrevealed assignments", () => {
    const e01 = { ...manifest, phase: "e01" as const }; const study: StudyView = { ...studyWorkspace.study, manifests: [e01], assignments: [{ id: "a", opportunity_id: opportunity.id, manifest_id: e01.id, position: 0, condition: "d", assigned_at: opportunity.created_at, issued_bc_at_assignment: 0, h1_saturated: false, ledger_sequence: 0 }] };
    expect(reviewScope(study, e01).allowed).toBe(false);
    expect(reviewScope(study, { ...e01, status: "completed" }).allowed).toBe(true);
    study.assignments[0].condition = null;
    expect(reviewScope(study, { ...e01, status: "completed" }).allowed).toBe(false);
  });
  it("freezes an active review at the planned matured prefix and never pools epochs", () => {
    const e01 = { ...manifest, phase: "e01" as const }; const opportunities = Array.from({ length: 31 }, (_, index) => ({ ...opportunity, id: `o${index}`, epoch_id: index === 0 ? "other-epoch" : epoch.id }));
    const study: StudyView = { ...studyWorkspace.study, manifests: [e01], opportunities, assignments: opportunities.map((item, index) => ({ id: `a${index}`, opportunity_id: item.id, manifest_id: e01.id, position: index, condition: "d", assigned_at: opportunity.created_at, issued_bc_at_assignment: 0, h1_saturated: false, ledger_sequence: 0 })) };
    const scope = reviewScope(study, e01); expect(scope).toMatchObject({ allowed: true, count: 30, totalAssigned: 31 });
    expect(assignedSummary(study, e01, "d", epoch.id, scope).eligible).toBe(29);
  });
  it("separates external deadlines at the prospectively specified 48-hour boundary", () => {
    expect(lowPressure(opportunity)).toBe(true);
    expect(lowPressure({ ...opportunity, deadline_at: "2026-01-04T12:00:00.000Z" })).toBe(false);
    expect(lowPressure({ ...opportunity, deadline_at: "2026-01-04T12:00:00.001Z" })).toBe(true);
  });
  it("summarizes only recorded capture estimates and keeps useful work separate", () => {
    const study: StudyView = { ...studyWorkspace.study, burdens: [ ["planning", 1], ["logging", 1], ["useful_work", 30], ["game", 3] ].map(([category, minutes], index) => ({ id: String(index), opportunity_id: opportunity.id, category: category as "planning", minutes: Number(minutes), note: null, created_at: opportunity.cue_at, method: "user_estimate_exclusive_minutes" })) };
    expect(summarize(study, study.opportunities)).toMatchObject({ medianCaptureMinutes: 2, captureEstimates: 1, managementMinutes: 5, usefulWorkMinutes: 30 });
    expect(median([])).toBeNull(); expect(median([4, 1, 2, 3])).toBe(2.5);
  });
});
