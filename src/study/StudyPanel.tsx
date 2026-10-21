import { useEffect, useRef, useState, type FormEvent } from "react";
import * as api from "../platform/client";
import OpportunityEditor, { type StudyMutation } from "./OpportunityEditor";
import OutcomeEditor from "./OutcomeEditor";
import StudySettings from "./StudySettings";
import StudyReport from "./StudyReport";
import { displayTime, optionalText, StudyField } from "./fields";
import { latestOutcome } from "./reporting";
import { useVisible } from "./useVisible";

export const outcomeLabels: Record<api.OutcomeState, string> = { pending: "Pending", observed_start: "Observed start", observed_nonstart: "Observed nonstart", missing: "Missing / unknown" };
type Editor = { kind: "plan" } | { kind: "outcome"; id: string } | { kind: "burden" } | { kind: "reflection" } | { kind: "deviation"; id: string };
interface Props { workspace: api.Workspace; objective?: api.Objective; busy: boolean; onMutation: StudyMutation; onDirtyChange: (dirty: boolean) => void }
export default function StudyPanel({ workspace, objective, busy, onMutation, onDirtyChange }: Props) {
  const [editor, setEditor] = useState<Editor | null>(null);
  const [expanded, setExpanded] = useState(false);
  const [historyExpanded, setHistoryExpanded] = useState(false);
  const [historyLimit, setHistoryLimit] = useState(50);
  const [settingsDirty, setSettingsDirty] = useState(false);
  const [text, setText] = useState("");
  const [minutes, setMinutes] = useState("");
  const [category, setCategory] = useState<api.BurdenCategory>("logging");
  const [opportunityId, setOpportunityId] = useState("");
  const [error, setError] = useState("");
  const [failedCue, setFailedCue] = useState("");
  const formRef = useRef<HTMLFormElement>(null);
  const cueRef = useRef<HTMLDivElement>(null);
  const cueAttempts = useRef(new Set<string>());
  const study = workspace.study;
  const sorted = [...study.opportunities].sort((a, b) => b.cue_at.localeCompare(a.cue_at));
  const selected = editor?.kind === "outcome" ? study.opportunities.find((item) => item.id === editor.id) : undefined;
  const current = sorted.find((item) => item.eligible && Date.parse(item.cue_at) <= Date.now() && Date.now() < Date.parse(item.cue_at) + item.window_seconds * 1000);
  const activeManifest = study.manifests.find((item) => item.status === "active");
  const cueVisible = useVisible(cueRef, !!current);
  useEffect(() => { onDirtyChange(!!editor || settingsDirty); }, [editor, settingsDirty, onDirtyChange]);
  useEffect(() => { if (editor) formRef.current?.querySelector<HTMLElement>("input, textarea, select")?.focus(); }, [editor]);
  useEffect(() => {
    if (!current || !cueVisible || busy || study.exposures.some((item) => item.opportunity_id === current.id && item.kind === "cue_delivered") || cueAttempts.current.has(current.id) || failedCue === `${current.id}:${workspace.generation}`) return;
    cueAttempts.current.add(current.id);
    void onMutation(() => api.recordStudyExposure({ expected_generation: workspace.generation, opportunity_id: current.id, kind: "cue_delivered", note: "Opportunity cue rendered in the visible Work interface during its declared window." }), "Work cue displayed. This is not evidence of a meaningful start or of noticing the cue.").then((saved) => { if (!saved) { cueAttempts.current.delete(current.id); setFailedCue(`${current.id}:${workspace.generation}`); } else setFailedCue(""); });
  }, [current, cueVisible, study.exposures, busy, workspace.generation, onMutation, failedCue]);
  function open(next: Editor) { setEditor(next); setText(""); setMinutes(""); setError(""); setOpportunityId(""); }
  function close() { setEditor(null); setError(""); setText(""); }
  async function saveSupplement(event: FormEvent) {
    event.preventDefault(); if (!editor) return;
    if (editor.kind !== "burden" && !text.trim()) { setError("Add a short factual note before saving."); document.getElementById("study-extra-note")?.focus(); return; }
    if (editor.kind === "burden" && (!minutes || !Number.isInteger(Number(minutes)) || Number(minutes) < 0 || Number(minutes) > 1440)) { setError("Estimate 0–1440 whole minutes, without double-counting another category."); document.getElementById("study-burden-minutes")?.focus(); return; }
    const saved = await onMutation(() => editor.kind === "burden" ? api.recordStudyBurden({ expected_generation: workspace.generation, opportunity_id: optionalText(opportunityId), category, minutes: Number(minutes), note: optionalText(text) })
      : editor.kind === "deviation" ? api.recordStudyExposure({ expected_generation: workspace.generation, opportunity_id: editor.id, kind: "deviation", note: text })
        : api.recordStudyReflection({ expected_generation: workspace.generation, manifest_id: activeManifest?.id ?? null, note: text }), "Optional context saved locally.");
    if (saved) close();
  }
  return <section className="study-panel" aria-label="Optional work opportunity tracking">
    {current && <div className="opportunity-cue" ref={cueRef}><p className="eyebrow">Your planned work opportunity</p><p className="preserve-text"><strong>{current.meaningful_start}</strong></p><p>Stop condition: {current.stop_snapshot}</p><p className="muted">Work if this still fits. You may rest or stop. Record what happened afterward.</p>{failedCue && !study.exposures.some((item) => item.opportunity_id === current.id && item.kind === "cue_delivered") && <button type="button" className="secondary small" disabled={busy} onClick={() => void onMutation(api.loadWorkspace, "Saved cue delivery records checked.").then((loaded) => { if (loaded) setFailedCue(""); })}>Check and retry cue delivery record</button>}</div>}
    <details open={expanded} onToggle={(event) => { if (event.target === event.currentTarget) setExpanded(event.currentTarget.open); }}><summary>Work opportunities <span className="muted">· optional</span></summary><div hidden={!expanded}>
      <p>Declare realistic work occasions and record starts, nonstarts, or uncertainty. The saved action above remains available without tracking.</p>
      {!editor && <div className="button-row"><button type="button" className="secondary" disabled={!objective || !workspace.actions.some((item) => item.objective_id === objective.id) || settingsDirty} onClick={() => open({ kind: "plan" })}>Plan an opportunity</button>
        <button type="button" className="secondary" disabled={settingsDirty} onClick={() => open({ kind: "burden" })}>Optional time estimate</button><button type="button" className="secondary" disabled={settingsDirty} onClick={() => open({ kind: "reflection" })}>Optional weekly reflection</button></div>}
      {editor?.kind === "plan" && objective && <OpportunityEditor workspace={workspace} objective={objective} epochs={study.epochs} manifests={study.manifests} busy={busy} onMutation={onMutation} onClose={close} />}
      {selected && <OutcomeEditor workspace={workspace} opportunity={selected} correcting={!!latestOutcome(study, selected.id)} busy={busy} onMutation={onMutation} onClose={close} />}
      {editor && ["burden", "reflection", "deviation"].includes(editor.kind) && <form className="study-editor" data-active-draft="true" ref={formRef} onSubmit={(event) => void saveSupplement(event)} noValidate>
        <h3>{editor.kind === "burden" ? "Optional time estimate" : editor.kind === "deviation" ? "Record changed circumstances" : "Optional weekly reflection"}</h3><p className="draft-label">Unsaved draft</p>
        {error && <p role="alert" className="field-error">{error}</p>}
        {editor.kind === "burden" ? <>
          <p>Estimate time spent on this activity alone. Exclude minutes already recorded under another category. Useful work is separate from managing the app.</p>
          <StudyField id="study-burden-opportunity" label="Related opportunity (optional)"><select id="study-burden-opportunity" value={opportunityId} onChange={(event) => setOpportunityId(event.target.value)}><option value="">General administration — not a capture-time estimate</option>{sorted.map((item) => <option key={item.id} value={item.id}>{displayTime(item.cue_at)} · {item.action_snapshot.slice(0, 60)}</option>)}</select></StudyField>
          <StudyField id="study-burden-category" label="Time category"><select id="study-burden-category" value={category} onChange={(event) => setCategory(event.target.value as api.BurdenCategory)}>{([ ["planning", "Planning"], ["logging", "Outcome logging"], ["review", "Progress review"], ["game", "Construction inspection"], ["correction", "Corrections or rework"], ["other", "Other administration"], ["useful_work", "Useful real-world work"] ] as const).map(([value, label]) => <option key={value} value={value}>{label}</option>)}</select></StudyField>
          <StudyField id="study-burden-minutes" label="Estimated exclusive minutes"><input id="study-burden-minutes" type="number" min={0} max={1440} step={1} value={minutes} onChange={(event) => setMinutes(event.target.value)} /></StudyField>
        </> : editor.kind === "reflection" ? <p>Optional prompts: usefulness, enjoyment, pressure, fatigue, work or sleep displacement. Personal self-report, not a clinical measure. Aim for no more than five minutes; no daily survey is required.</p>
          : <p>Record a disruption, emergency, changed goal, or override. The original eligibility, assignment, action, and denominator remain intact.</p>}
        <StudyField id="study-extra-note" label={editor.kind === "burden" ? "Estimate note (optional)" : "Short factual note"}><textarea id="study-extra-note" rows={3} maxLength={2000} value={text} onChange={(event) => setText(event.target.value)} /></StudyField>
        <div className="button-row"><button type="submit" disabled={busy}>Save optional context</button><button type="button" className="secondary" onClick={close}>Discard context draft</button></div>
      </form>}
      {!editor && <StudySettings workspace={workspace} epochs={study.epochs} manifests={study.manifests} busy={busy} onMutation={onMutation} onDirtyChange={setSettingsDirty} />}
      <details className="opportunity-history" onToggle={(event) => { if (event.target === event.currentTarget) setHistoryExpanded(event.currentTarget.open); }}><summary>Opportunity history · {sorted.length} planned</summary>
        {historyExpanded && (sorted.length === 0 ? <p>No opportunities planned. Nothing is required each day.</p> : <><ol>{sorted.slice(0, historyLimit).map((item) => {
          const latest = latestOutcome(study, item.id); const assignment = study.assignments.find((record) => record.opportunity_id === item.id);
          const manifest = study.manifests.find((record) => record.id === item.manifest_id);
          return <li key={item.id} data-opportunity-id={item.id}><div className="opportunity-history-heading"><h3>{displayTime(item.cue_at)}</h3><span className="outcome-label">{item.eligible ? outcomeLabels[item.outcome_state] : "Known ineligible"}</span></div>
            <p className="preserve-text"><strong>{item.action_snapshot}</strong></p><p className="preserve-text">Meaningful start: {item.meaningful_start}</p>
            <p className="muted">{item.domain} · {manifest?.phase.toUpperCase() ?? "Ordinary use"} · {study.epochs.find((epoch) => epoch.id === item.epoch_id)?.label} · {item.window_seconds / 60}-minute window · {item.timezone}</p>
            {assignment && <p className="muted">Assignment {assignment.position + 1} retained · {assignment.condition === null ? "condition hidden until the declared boundary" : assignment.condition === "d" ? "D" : "D + construction"}{assignment.h1_saturated && " · scene saturated at assignment"}</p>}
            {item.unavailability && <p>Declared unavailability: {item.unavailability}</p>}
            {latest?.late_start && <p>Late start recorded separately; primary-window outcome remains a nonstart.</p>}
            {latest?.note && <p className="preserve-text">{latest.note}</p>}
            {latest?.uncertainty_reason && <p>Uncertainty: {latest.uncertainty_reason}</p>}
            {latest?.technical_failure && <p>A technical problem was reported; this opportunity is retained.</p>}
            {latest?.priority_advanced !== null && latest?.priority_advanced !== undefined && <p>Declared priority advanced: {latest.priority_advanced ? "yes" : "no"} (self-reported).</p>}
            {latest?.quality_note && <p>Domain-specific quality: {latest.quality_note}</p>}
            {latest?.acceptance_id && <p>Linked accomplishment: {workspace.progress.acceptances.find((record) => record.id === latest.acceptance_id)?.factual_summary} · {workspace.progress.acceptances.find((record) => record.id === latest.acceptance_id)?.status}</p>}
            <div className="button-row"><button type="button" className="secondary small" disabled={!!editor || settingsDirty || Date.parse(item.cue_at) > Date.now()} onClick={() => open({ kind: "outcome", id: item.id })}>{latest ? "Correct or add a late-start observation" : "Record outcome"}</button><button type="button" className="secondary small" disabled={!!editor || settingsDirty} onClick={() => open({ kind: "deviation", id: item.id })}>Record disruption or override</button></div>
            <details><summary>Original declaration and observation history</summary><p>Stop condition: {item.stop_snapshot}</p><p>Priority: {item.priority} · available {item.available_minutes} minutes · criterion version {item.criterion_version} · declared {displayTime(item.created_at)}</p>
              {item.deadline_at && <p>External deadline: {displayTime(item.deadline_at)}</p>}{item.context && <p>Optional context: {item.context}</p>}
              {study.outcomes.filter((event) => event.opportunity_id === item.id).sort((a, b) => a.sequence - b.sequence).map((event) => <div className="observation-event" key={event.id}><p>Observation {event.sequence}: {outcomeLabels[event.state]} · {event.precision} timing · {displayTime(event.observed_at)}</p>{event.started_at && <p>Reported start: {displayTime(event.started_at)}{event.uncertainty_seconds && ` ±${event.uncertainty_seconds} seconds`}</p>}{event.note && <p>{event.note}</p>}{event.correction_reason && <p>Correction reason: {event.correction_reason}</p>}<p className="muted">{event.censored_at_seconds !== null ? `Right-censored at ${event.censored_at_seconds} seconds.` : event.latency_seconds !== null ? `Known latency: ${event.latency_seconds} seconds.` : "Exact latency unknown."}</p></div>)}
              {study.exposures.filter((event) => event.opportunity_id === item.id).map((event) => <p key={event.id}>{event.kind.replace(/_/g, " ")} · {displayTime(event.created_at)}{event.note ? ` · ${event.note}` : ""}</p>)}
            </details>
          </li>;
        })}</ol>{historyLimit < sorted.length && <button type="button" className="secondary" onClick={() => setHistoryLimit((value) => value + 50)}>Show 50 more opportunities</button>}</>)}
      </details>
      <StudyReport study={study} />
      {study.reflections.length > 0 && <details className="study-reflections"><summary>Saved optional reflections</summary>{study.reflections.map((item) => <div key={item.id}><p className="muted">{displayTime(item.created_at)}</p><p className="preserve-text">{item.note}</p></div>)}</details>}
    </div></details>
  </section>;
}
