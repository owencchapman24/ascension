import type { AssignedCondition, OpportunityOutcome, StudyManifest, StudyView, WorkOpportunity } from "../platform/client";

export function latestOutcome(study: StudyView, id: string): OpportunityOutcome | undefined {
  return study.outcomes.filter((item) => item.opportunity_id === id).sort((a, b) => b.sequence - a.sequence)[0];
}
export function median(values: number[]): number | null {
  if (!values.length) return null;
  const sorted = [...values].sort((a, b) => a - b); const middle = Math.floor(sorted.length / 2);
  return sorted.length % 2 ? sorted[middle] : (sorted[middle - 1] + sorted[middle]) / 2;
}
export function summarize(study: StudyView, opportunities: WorkOpportunity[]) {
  const eligible = opportunities.filter((item) => item.eligible);
  const outcomes = eligible.map((item) => latestOutcome(study, item.id)).filter((item): item is OpportunityOutcome => !!item);
  const starts = eligible.filter((item) => item.outcome_state === "observed_start").length;
  const nonstarts = eligible.filter((item) => item.outcome_state === "observed_nonstart").length;
  const missing = eligible.filter((item) => item.outcome_state === "missing").length;
  const pending = eligible.filter((item) => item.outcome_state === "pending").length;
  const burden = study.burdens.filter((item) => eligible.some((opportunity) => opportunity.id === item.opportunity_id));
  const capture = eligible.flatMap((item) => {
    const entries = burden.filter((entry) => entry.opportunity_id === item.id && (entry.category === "planning" || entry.category === "logging"));
    return entries.length ? [entries.reduce((sum, entry) => sum + entry.minutes, 0)] : [];
  });
  const delivered = eligible.filter((item) => study.exposures.some((event) => event.opportunity_id === item.id && event.kind === "cue_delivered")).length;
  const knownTiming = outcomes.filter((item) => item.latency_seconds !== null).map((item) => item.latency_seconds!);
  return { planned: opportunities.length, eligible: eligible.length, ineligible: opportunities.length - eligible.length,
    starts, nonstarts, missing, pending, ascertainable: starts + nonstarts, completeness: eligible.length ? (starts + nonstarts) / eligible.length : null,
    lower: eligible.length ? starts / eligible.length : null, upper: eligible.length && pending === 0 ? (starts + missing) / eligible.length : null,
    late: outcomes.filter((item) => item.late_start).length,
    technical: eligible.filter((item) => study.outcomes.some((event) => event.opportunity_id === item.id && event.technical_failure)).length,
    deviations: eligible.filter((item) => study.exposures.some((event) => event.opportunity_id === item.id && ["deviation", "voluntary_world"].includes(event.kind))).length,
    delivered, deliveryUnrecorded: eligible.length - delivered,
    seen: eligible.filter((item) => study.exposures.some((event) => event.opportunity_id === item.id && event.kind === "seen")).length,
    corrections: study.outcomes.filter((item) => item.supersedes && eligible.some((opportunity) => opportunity.id === item.opportunity_id)).length,
    prospective: eligible.filter((item) => Date.parse(item.created_at) < Date.parse(item.cue_at) && !!item.meaningful_start.trim()).length,
    medianCaptureMinutes: median(capture), captureEstimates: capture.length,
    managementMinutes: burden.filter((item) => item.category !== "useful_work").reduce((sum, item) => sum + item.minutes, 0),
    usefulWorkMinutes: burden.filter((item) => item.category === "useful_work").reduce((sum, item) => sum + item.minutes, 0),
    medianExactLatencySeconds: median(knownTiming), exactLatencyCount: knownTiming.length,
    priorityAdvanced: outcomes.filter((item) => item.priority_advanced === true).length,
    priorityReported: outcomes.filter((item) => item.priority_advanced !== null).length,
    qualityNotes: outcomes.filter((item) => item.quality_note !== null).length,
    linkedAcceptanceIds: [...new Set(outcomes.flatMap((item) => item.acceptance_id ? [item.acceptance_id] : []))],
  };
}

export function reviewScope(study: StudyView, manifest: StudyManifest, now = Date.now()) {
  const assignments = study.assignments.filter((item) => item.manifest_id === manifest.id).sort((a, b) => a.position - b.position);
  const mature = (count: number) => assignments.length >= count && assignments.slice(0, count).every((assignment) => {
    const opportunity = study.opportunities.find((item) => item.id === assignment.opportunity_id);
    return assignment.condition !== null && !!opportunity && Date.parse(opportunity.cue_at) + opportunity.window_seconds * 1000 <= now;
  });
  const completed = ["completed", "abandoned"].includes(manifest.status);
  const count = completed && assignments.length && mature(assignments.length) ? assignments.length
    : [...manifest.review_after].sort((a, b) => b - a).find(mature) ?? 0;
  return { allowed: count > 0, count, totalAssigned: assignments.length, assignments: assignments.slice(0, count) };
}

export function assignedSummary(study: StudyView, manifest: StudyManifest, condition: AssignedCondition, epochId: string, scope = reviewScope(study, manifest), filter: (opportunity: WorkOpportunity, position: number) => boolean = () => true) {
  const selected = scope.assignments.filter((item) => item.condition === condition);
  return summarize(study, study.opportunities.filter((opportunity) => {
    const assignment = selected.find((item) => item.opportunity_id === opportunity.id);
    return !!assignment && opportunity.epoch_id === epochId && filter(opportunity, assignment.position);
  }));
}
export function differenceBounds(plus: ReturnType<typeof summarize>, conventional: ReturnType<typeof summarize>) {
  return plus.lower !== null && plus.upper !== null && conventional.lower !== null && conventional.upper !== null
    ? { lower: plus.lower - conventional.upper, upper: plus.upper - conventional.lower } : null;
}
export const lowPressure = (opportunity: WorkOpportunity) => opportunity.deadline_at === null || Date.parse(opportunity.deadline_at) - Date.parse(opportunity.cue_at) > 48 * 60 * 60_000;
export const percent = (value: number | null) => value === null ? "Not available" : `${(value * 100).toFixed(1)}%`;
