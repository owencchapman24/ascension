import { useEffect, useRef, useState, type FormEvent } from "react";
import type { WorkCheckpoint, Workspace, Objective } from "../platform/client";
import type { StudyMutation } from "../study/OpportunityEditor";
import CheckpointEditor from "./CheckpointEditor";
import { organizedObjectives } from "./ObjectiveManager";

export default function DailyWork({ workspace, objective, busy, onMutation, onDirtyChange, onSelect }: {
  workspace: Workspace; objective: Objective; busy: boolean; onMutation: StudyMutation;
  onDirtyChange: (dirty: boolean) => void; onSelect: (id: string) => void;
}) {
  const [checkpoint, setCheckpoint] = useState<false | true | WorkCheckpoint>(false);
  const [mode, setMode] = useState<"none" | "small" | "choose" | "rest" | "resume">("none");
  const [step, setStep] = useState({ action: "", stop: "" });
  const [entry, setEntry] = useState<typeof step | null>(null);
  const [error, setError] = useState("");
  const [historyLimit, setHistoryLimit] = useState(20);
  const smallForm = useRef<HTMLFormElement>(null);
  const checkpointWasOpen = useRef(false);
  const history = workspace.daily.checkpoints.filter((item) => item.objective_id === objective.id).slice().reverse();
  const alternatives = organizedObjectives(workspace).filter((item) => item.organization.status === "active" && item.objective.id !== objective.id);
  useEffect(() => { onDirtyChange(!!checkpoint || mode === "small" && !!(step.action || step.stop)); }, [checkpoint, mode, step, onDirtyChange]);
  useEffect(() => { if (mode === "small") smallForm.current?.querySelector<HTMLInputElement>("input")?.focus(); }, [mode]);
  useEffect(() => { if (!checkpoint && checkpointWasOpen.current && !busy) document.getElementById("save-stopped")?.focus(); if (!busy) checkpointWasOpen.current = !!checkpoint; }, [checkpoint, busy]);
  function closeCheckpoint() { setCheckpoint(false); }
  function keepSmallEntry(event: FormEvent) {
    event.preventDefault();
    if (!step.action.trim() || !step.stop.trim() || [...step.action].length > 2000 || [...step.stop].length > 2000) { setError("Add one useful action and an observable stopping point, each at most 2,000 characters."); return; }
    setEntry({ ...step }); setMode("resume"); setError("");
  }
  return <div className="daily-work">
    {!checkpoint && <div className="button-row work-actions"><button type="button" onClick={() => { setMode("resume"); document.querySelector<HTMLElement>(".saved-action h3")?.focus(); }}>Resume work</button><button id="save-stopped" type="button" className="secondary" onClick={() => { setCheckpoint(true); setMode("none"); }}>Save where I stopped</button></div>}
    {mode === "resume" && !checkpoint && <p role="status" className="muted">Your work is ready above. Return whenever you want to save where you stopped.</p>}
    {entry && <aside className="smaller-entry" aria-label="Temporary smaller entry"><h3>Smaller entry for this visit</h3><p className="preserve-text">{entry.action}</p><p>Stop: {entry.stop}</p><p className="muted">For {objective.title}. Your main saved action remains above. Temporary for this Work view; editing other records, reloading, switching objectives, or closing the app clears it.</p><button type="button" className="secondary small" onClick={() => setEntry(null)}>Dismiss smaller entry</button></aside>}
    {!checkpoint && <details className="initiation-support"><summary>Need a different way to start?</summary><div className="button-row">
      <button type="button" className="secondary" onClick={() => setMode("small")}>Make the entry smaller</button><button type="button" className="secondary" onClick={() => setMode("choose")}>Choose another priority</button><button type="button" className="secondary" onClick={() => setMode("rest")}>Rest or step away</button>
    </div>
      {mode === "small" && <form ref={smallForm} onSubmit={keepSmallEntry} noValidate><p>What is one useful piece of this action you can do now?</p>{error && <p role="alert" className="field-error">{error}</p>}
        <label htmlFor="small-action">Small action</label><input id="small-action" value={step.action} onChange={(event) => setStep({ ...step, action: event.target.value })} />
        <label htmlFor="small-stop">Observable stopping point</label><input id="small-stop" value={step.stop} onChange={(event) => setStep({ ...step, stop: event.target.value })} />
        <div className="button-row"><button type="submit">Use this smaller entry</button><button type="button" className="secondary" onClick={() => setMode("none")}>Cancel smaller entry</button></div>
      </form>}
      {mode === "choose" && <div aria-label="Another active priority">{alternatives.length ? alternatives.map(({ objective: other }) => <div className="priority-choice" key={other.id}><p><strong>{other.title}</strong> · {workspace.projects.find((item) => item.id === other.project_id)?.title}</p><p>{workspace.actions.find((item) => item.objective_id === other.id)?.description ?? "No next action saved yet."}</p><button type="button" className="secondary" onClick={() => onSelect(other.id)}>Focus {other.title}</button></div>) : <p>No other active objectives. You can create or reactivate one below.</p>}</div>}
      {mode === "rest" && <p role="status" className="notice">You can step away. Your saved work will be here when you return.</p>}
    </details>}
    {checkpoint && <CheckpointEditor workspace={workspace} objective={objective} correction={checkpoint === true ? undefined : checkpoint} busy={busy} onMutation={onMutation} onClose={closeCheckpoint} />}
    <details className="checkpoint-history"><summary>Checkpoint history · {history.length}</summary><p className="muted">Factual self-reports and unverified references, separate from accepted accomplishments.</p>
      {!history.length && <p>No checkpoints yet.</p>}
      <ol>{history.slice(0, historyLimit).map((item) => <li key={item.id}><p className="muted"><time dateTime={item.recorded_at}>{new Date(item.recorded_at).toLocaleString()}</time> · Self-report{item.supersedes && " · Correction"}</p><p className="preserve-text">{item.note}</p>{item.reference && <p className="preserve-text">Unverified reference: {item.reference}</p>}{item.correction_reason && <p>Correction reason: {item.correction_reason}</p>}<details><summary>Saved action snapshot</summary><p>{item.description}</p><p>Stop: {item.stop_condition}</p>{item.resumption_note && <p>{item.resumption_note}</p>}</details>
        {history.some((other) => other.supersedes === item.id) ? <p className="muted">Corrected later; original account retained.</p> : <button type="button" className="secondary small" disabled={busy || !!checkpoint} onClick={() => { setCheckpoint(item); setMode("none"); }}>Correct checkpoint</button>}
      </li>)}</ol>{history.length > historyLimit && <button type="button" className="secondary" onClick={() => setHistoryLimit((value) => value + 20)}>Show 20 more checkpoints</button>}
    </details>
  </div>;
}
