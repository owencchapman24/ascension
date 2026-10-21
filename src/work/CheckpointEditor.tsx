import { useEffect, useRef, useState, type FormEvent } from "react";
import * as api from "../platform/client";
import type { StudyMutation } from "../study/OpportunityEditor";

export default function CheckpointEditor({ workspace, objective, correction, busy, onMutation, onClose }: {
  workspace: api.Workspace; objective: api.Objective; correction?: api.WorkCheckpoint;
  busy: boolean; onMutation: StudyMutation; onClose: () => void;
}) {
  const action = workspace.actions.find((item) => item.objective_id === objective.id);
  const savedNote = workspace.notes.find((item) => item.objective_id === objective.id);
  const missingObjective = !workspace.objectives.some((item) => item.id === objective.id);
  const [draft, setDraft] = useState({ note: correction?.note ?? "", reference: correction?.reference ?? "",
    description: action?.description ?? "", stop_condition: action?.stop_condition ?? "", resumption_note: savedNote?.text ?? "", correction_reason: "" });
  const [error, setError] = useState("");
  const [needsRefresh, setNeedsRefresh] = useState(false);
  const pending = useRef<api.RecordCheckpointInput | null>(null);
  const form = useRef<HTMLFormElement>(null);
  useEffect(() => { form.current?.querySelector<HTMLTextAreaElement>("textarea")?.focus(); }, []);
  async function save(event: FormEvent) {
    event.preventDefault();
    const fields: [keyof typeof draft, string, number, boolean][] = [
      ["note", "What changed", 8000, true], ["description", "Next action", 2000, true], ["stop_condition", "Stopping condition", 2000, true],
      ["resumption_note", "Resumption note", 8000, false], ["reference", "Supporting reference", 2000, false], ["correction_reason", "Correction reason", 2000, !!correction],
    ];
    for (const [key, label, limit, required] of fields) {
      if (required && !draft[key].trim() || [...draft[key]].length > limit || draft[key].includes("\0")) {
        setError(`${label} needs ${required ? "a nonempty value and " : ""}at most ${limit.toLocaleString()} characters, without null characters.`);
        document.getElementById(`checkpoint-${key}`)?.focus(); return;
      }
    }
    // Preserve the complete original command on transport failure, including its
    // generation. A lost response must replay the committed identity, not insert twice.
    pending.current ??= { expected_generation: workspace.generation, command_id: crypto.randomUUID(), objective_id: objective.id,
      note: draft.note, reference: draft.reference.trim() || null, description: draft.description, stop_condition: draft.stop_condition,
      resumption_note: draft.resumption_note.trim() ? draft.resumption_note : null, supersedes: correction?.id ?? null, correction_reason: correction ? draft.correction_reason : null };
    const input = pending.current;
    setError("");
    let rejection: api.CommandError | null = null;
    const saved = await onMutation(async () => {
      try { return (await api.recordCheckpoint(input)).workspace; }
      catch (cause) {
        const failure = api.normalizeError(cause);
        if (["conflict", "invalid_input", "validation_error", "idempotency_conflict"].includes(failure.code)) { rejection = failure; pending.current = null; setNeedsRefresh(failure.code === "conflict" || failure.code === "idempotency_conflict"); }
        throw cause;
      }
    }, correction ? "Checkpoint correction saved. The original account remains in history." : "Checkpoint and next action saved on this device.");
    if (saved) onClose();
    else setError(rejection ? `${(rejection as api.CommandError).message} Your draft is kept.` : "Saving was not confirmed. Your draft is kept. Retry the same checkpoint before making further changes.");
  }
  async function checkSaved() {
    let found = false;
    const saved = await onMutation(async () => { const next = await api.loadWorkspace(); found = !!pending.current && next.daily.commands.some((item) => item.id === pending.current?.command_id); return next; }, "Saved checkpoint history checked; your draft was retained.");
    if (!saved) return;
    if (found) onClose();
    else { setNeedsRefresh(false); setError(pending.current ? "This command is not in saved history. Retry the same checkpoint to settle it safely." : "Saved records refreshed. Review your retained draft, then save when ready."); }
  }
  const frozen = pending.current !== null;
  function change(key: keyof typeof draft, value: string) { setDraft({ ...draft, [key]: value }); setError(""); }
  return <form ref={form} className="checkpoint-editor" onSubmit={(event) => void save(event)} noValidate>
    <div className="section-heading"><h3>{correction ? "Correct a factual checkpoint" : "Save where I stopped"}</h3><span className="draft-label">Unsaved until confirmed</span></div>
    <p className="muted">Objective: {objective.title}</p>
    <p className="muted">A personal account of work. Accomplishment acceptance stays in Progress and evidence.</p>
    {correction && <p>Correcting: <span className="preserve-text">{correction.note}</span></p>}
    {error && <p role="alert" className="field-error">{error}</p>}
    <fieldset className="optional-controls" disabled={busy}>
      <legend className="visually-hidden">Checkpoint details</legend>
      <label htmlFor="checkpoint-note">What changed?</label><textarea id="checkpoint-note" rows={2} readOnly={frozen} value={draft.note} onChange={(event) => change("note", event.target.value)} />
      {correction && <><label htmlFor="checkpoint-correction_reason">Correction reason</label><input id="checkpoint-correction_reason" readOnly={frozen} value={draft.correction_reason} onChange={(event) => change("correction_reason", event.target.value)} /></>}
      <label htmlFor="checkpoint-description">Next action</label><textarea id="checkpoint-description" rows={2} readOnly={frozen} value={draft.description} onChange={(event) => change("description", event.target.value)} />
      <label htmlFor="checkpoint-stop_condition">Stopping condition</label><input id="checkpoint-stop_condition" readOnly={frozen} value={draft.stop_condition} onChange={(event) => change("stop_condition", event.target.value)} />
      <label htmlFor="checkpoint-resumption_note">Resumption note (optional)</label><textarea id="checkpoint-resumption_note" rows={2} readOnly={frozen} value={draft.resumption_note} onChange={(event) => change("resumption_note", event.target.value)} />
      <details><summary>Supporting reference (optional)</summary><label htmlFor="checkpoint-reference">Artifact or source reference</label><input id="checkpoint-reference" readOnly={frozen} value={draft.reference} onChange={(event) => change("reference", event.target.value)} /><p className="muted">Saved as an unverified reference. Files are not opened or scanned.</p></details>
    </fieldset>
    <div className="button-row"><button type="submit" disabled={busy || needsRefresh || missingObjective}>{frozen ? "Retry same checkpoint" : correction ? "Save checkpoint correction" : "Save checkpoint"}</button>{(needsRefresh || frozen) && <button type="button" className="secondary" disabled={busy} onClick={() => void checkSaved()}>Check saved history and keep draft</button>}<button type="button" className="secondary" disabled={busy || frozen && !missingObjective} onClick={onClose}>Discard checkpoint draft</button></div>
  </form>;
}
