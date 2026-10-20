import { useEffect, useRef, useState, type FormEvent } from "react";
import * as api from "../platform/client";
import { displayTime, optionalText, StudyField, utcDateTime } from "./fields";
import type { StudyMutation } from "./OpportunityEditor";

interface Props {
  workspace: api.Workspace;
  opportunity: { id: string; objective_id: string; meaningful_start: string; cue_at: string; window_seconds: number };
  correcting: boolean;
  busy: boolean;
  onMutation: StudyMutation;
  onClose: () => void;
}
export default function OutcomeEditor({ workspace, opportunity, correcting, busy, onMutation, onClose }: Props) {
  const [draft, setDraft] = useState({ report: "unknown" as api.RecordOpportunityOutcomeInput["report"], started: "", precision: "unknown" as api.TimePrecision, uncertainty: "", note: "", technical_failure: false, uncertainty_reason: "", correction_reason: "", priority_advanced: "", quality_note: "", acceptance_id: "" });
  const [error, setError] = useState("");
  const ref = useRef<HTMLFormElement>(null);
  useEffect(() => { ref.current?.querySelector<HTMLElement>("input")?.focus(); }, []);
  async function save(event: FormEvent) {
    event.preventDefault();
    const fail = (message: string, id: string) => { setError(message); document.getElementById(id)?.focus(); };
    const started_at = draft.report === "started" ? utcDateTime(draft.started) : null;
    if (draft.report === "started" && draft.precision !== "unknown" && !started_at) return fail("Enter the reported start time, or choose unknown precision.", "outcome-time");
    if (draft.report === "started" && draft.precision === "approximate" && (!draft.uncertainty || !Number.isFinite(Number(draft.uncertainty)) || Number(draft.uncertainty) <= 0)) return fail("Give a positive uncertainty range around the approximate time.", "outcome-uncertainty");
    if (draft.report === "started" && draft.precision === "unknown" && !draft.uncertainty_reason.trim()) return fail("Explain what is unknown without inventing a start time.", "outcome-reason");
    if (correcting && !draft.correction_reason.trim()) return fail("Explain this correction. The earlier observation will remain in history.", "outcome-correction");
    setError("");
    const saved = await onMutation(() => api.recordOpportunityOutcome({ expected_generation: workspace.generation, opportunity_id: opportunity.id,
      report: draft.report, started_at: draft.precision === "unknown" ? null : started_at,
      precision: draft.report === "started" ? draft.precision : "unknown", uncertainty_seconds: draft.report === "started" && draft.precision === "approximate" ? Number(draft.uncertainty) * 60 : null,
      note: optionalText(draft.note), technical_failure: draft.technical_failure, uncertainty_reason: optionalText(draft.uncertainty_reason), correction_reason: correcting ? optionalText(draft.correction_reason) : null,
      priority_advanced: draft.priority_advanced === "" ? null : draft.priority_advanced === "yes", quality_note: optionalText(draft.quality_note), acceptance_id: optionalText(draft.acceptance_id),
    }), correcting ? "Outcome correction saved; the original observation is retained." : "Observation saved. This does not accept an accomplishment or issue Build Credits.");
    if (saved) onClose();
  }
  return <form className="study-editor" data-active-draft="true" ref={ref} onSubmit={(event) => void save(event)} noValidate>
    <h3>{correcting ? "Correct an observation" : "Record what happened"}</h3><p className="draft-label">Unsaved draft</p>
    <p className="preserve-text"><strong>Declared meaningful start:</strong> {opportunity.meaningful_start}</p>
    <p>Window: {displayTime(opportunity.cue_at)} to {displayTime(new Date(Date.parse(opportunity.cue_at) + opportunity.window_seconds * 1000).toISOString())}.</p>
    {error && <p role="alert" className="field-error">{error}</p>}
    <fieldset className="outcome-choices"><legend>What do you know about this opportunity?</legend>
      {([["started", "Started"], ["nonstart", "Did not start within the window"], ["unknown", "Not sure / leave unknown"]] as const).map(([value, label]) => <label className="check-label" key={value}><input type="radio" name="outcome-report" value={value} checked={draft.report === value} onChange={() => setDraft({ ...draft, report: value })} />{label}</label>)}
    </fieldset>
    {draft.report === "started" && <>
      <StudyField id="outcome-precision" label="How precisely do you know the start time?"><select id="outcome-precision" value={draft.precision} onChange={(event) => setDraft({ ...draft, precision: event.target.value as api.TimePrecision })}><option value="unknown">Unknown — do not infer a time</option><option value="approximate">Approximate recollection</option><option value="exact">Exact recorded time</option></select></StudyField>
      {draft.precision !== "unknown" && <StudyField id="outcome-time" label="Reported meaningful-start time"><input id="outcome-time" type="datetime-local" step={1} value={draft.started} onChange={(event) => setDraft({ ...draft, started: event.target.value })} /></StudyField>}
      {draft.precision === "approximate" && <StudyField id="outcome-uncertainty" label="Uncertainty either side (minutes)"><input id="outcome-uncertainty" type="number" min={0.1} step={0.1} value={draft.uncertainty} onChange={(event) => setDraft({ ...draft, uncertainty: event.target.value })} /></StudyField>}
      <p className="muted">A late start stays separate from primary-window success. A recollection that crosses the window boundary remains uncertain.</p>
    </>}
    <StudyField id="outcome-note" label="Short factual note (optional)"><textarea id="outcome-note" rows={2} maxLength={2000} value={draft.note} onChange={(event) => setDraft({ ...draft, note: event.target.value })} /></StudyField>
    <StudyField id="outcome-reason" label="Uncertainty or changed circumstances (optional unless timing is unknown)"><input id="outcome-reason" maxLength={2000} value={draft.uncertainty_reason} onChange={(event) => setDraft({ ...draft, uncertainty_reason: event.target.value })} /></StudyField>
    <details className="study-options"><summary>Useful progress, quality, or technical problems (optional)</summary>
      <label className="check-label"><input type="checkbox" checked={draft.technical_failure} onChange={(event) => setDraft({ ...draft, technical_failure: event.target.checked })} />A technical problem affected this opportunity.</label>
      <StudyField id="outcome-priority" label="Did the declared priority advance?"><select id="outcome-priority" value={draft.priority_advanced} onChange={(event) => setDraft({ ...draft, priority_advanced: event.target.value })}><option value="">Not recorded</option><option value="yes">Yes</option><option value="no">No</option></select></StudyField>
      <StudyField id="outcome-quality" label="Domain-specific progress or quality note"><textarea id="outcome-quality" rows={2} maxLength={2000} value={draft.quality_note} onChange={(event) => setDraft({ ...draft, quality_note: event.target.value })} /></StudyField>
      <StudyField id="outcome-acceptance" label="Link an existing accepted accomplishment"><select id="outcome-acceptance" value={draft.acceptance_id} onChange={(event) => setDraft({ ...draft, acceptance_id: event.target.value })}><option value="">None</option>{workspace.progress.acceptances.filter((item) => item.parent_id === opportunity.objective_id).map((item) => <option key={item.id} value={item.id}>{item.factual_summary.slice(0, 80)} · {item.status}</option>)}</select></StudyField>
      <p className="muted">This link does not accept work or issue currency. Record and accept qualifying evidence separately in Progress and evidence.</p>
    </details>
    {correcting && <StudyField id="outcome-correction" label="Reason for this outcome correction"><textarea id="outcome-correction" rows={2} maxLength={2000} value={draft.correction_reason} onChange={(event) => setDraft({ ...draft, correction_reason: event.target.value })} /></StudyField>}
    <div className="button-row"><button type="submit" disabled={busy}>Save observation</button><button type="button" className="secondary" onClick={onClose}>Discard observation draft</button></div>
  </form>;
}
