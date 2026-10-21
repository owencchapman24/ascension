import { useEffect, useRef, useState, type FormEvent } from "react";
import * as api from "../platform/client";
import { localDateTime, localTimezone, optionalText, StudyField, utcDateTime } from "./fields";

export type StudyMutation = (task: () => Promise<api.Workspace>, message: string) => Promise<boolean>;
interface Props {
  workspace: api.Workspace;
  objective: api.Objective;
  epochs: { id: string; label: string }[];
  manifests: { id: string; phase: api.StudyPhase; status: api.StudyStatus; epoch_id: string }[];
  busy: boolean;
  onMutation: StudyMutation;
  onClose: () => void;
}

export default function OpportunityEditor({ workspace, objective, epochs, manifests, busy, onMutation, onClose }: Props) {
  const action = workspace.actions.find((item) => item.objective_id === objective.id);
  const active = manifests.find((item) => item.status === "active");
  const [advanced, setAdvanced] = useState(false);
  const [draft, setDraft] = useState({ meaningful_start: action?.description ?? "", cue: localDateTime(new Date(Date.now() + 30 * 60_000)),
    epoch_id: active?.epoch_id ?? epochs[epochs.length - 1]?.id ?? "", manifest_id: "", domain: "general", eligible: true,
    unavailability: "", priority: "ordinary" as const as "ordinary" | "high", available_minutes: "30", deadline: "", context: "" });
  const [error, setError] = useState("");
  const ref = useRef<HTMLFormElement>(null);
  useEffect(() => { ref.current?.querySelector<HTMLElement>("textarea, input")?.focus(); }, []);
  function invalid(message: string, field: string) { setError(message); document.getElementById(field)?.focus(); }
  async function save(event: FormEvent) {
    event.preventDefault();
    const cue_at = utcDateTime(draft.cue);
    if (!draft.meaningful_start.trim()) return invalid("Describe observable work that will count as a meaningful start.", "opportunity-start");
    if (!cue_at || Date.parse(cue_at) <= Date.now()) return invalid("Choose a future work time so this opportunity is declared prospectively.", "opportunity-cue");
    if (advanced && !draft.epoch_id) return invalid("Choose a schedule epoch before planning the opportunity.", "opportunity-epoch");
    if (advanced && !draft.domain.trim()) return invalid("Name the work domain.", "opportunity-domain");
    if (!Number.isInteger(Number(draft.available_minutes)) || Number(draft.available_minutes) <= 0 || Number(draft.available_minutes) > 1440) return invalid("Available time must be a whole-minute estimate from 1 to 1440.", "opportunity-minutes");
    if (advanced && !draft.eligible && !draft.unavailability.trim()) return invalid("Record the known unavailability, such as planned rest.", "opportunity-unavailable");
    setError("");
    if (!advanced) {
      const saved = await onMutation(() => api.planOrdinaryOpportunity({ expected_generation: workspace.generation, objective_id: objective.id,
        meaningful_start: draft.meaningful_start, cue_at, timezone: localTimezone(), available_minutes: Number(draft.available_minutes) }), "Ordinary opportunity saved with a prospective 15-minute start window.");
      if (saved) onClose();
      return;
    }
    const saved = await onMutation(() => api.planOpportunity({ expected_generation: workspace.generation, objective_id: objective.id,
      epoch_id: draft.epoch_id, manifest_id: optionalText(draft.manifest_id), meaningful_start: draft.meaningful_start,
      cue_at, timezone: localTimezone(), window_seconds: 900, domain: draft.domain, eligible: draft.eligible,
      unavailability: draft.eligible ? null : optionalText(draft.unavailability), priority: draft.priority,
      available_minutes: Number(draft.available_minutes), deadline_at: utcDateTime(draft.deadline), context: optionalText(draft.context),
    }), "Opportunity saved with its action, start definition, eligibility, and 15-minute window.");
    if (saved) onClose();
  }
  return <form ref={ref} className="study-editor" data-active-draft="true" onSubmit={(event) => void save(event)} noValidate>
    <h3>Plan a work opportunity</h3><p className="draft-label">Unsaved draft</p>
    <p><strong>{objective.title}</strong></p>
    <p className="preserve-text">Saved action: {action?.description}</p>
    <p className="muted">A realistic occasion for work. Rest is always allowed; no daily opportunity is required.</p>
    {error && <p role="alert" className="field-error">{error}</p>}
    <StudyField id="opportunity-start" label="What observable first action will count as a meaningful start?" hint="Confirm task-relevant human work. Opening Ascension, clicking a timer, or writing a plan does not count by itself.">
      <textarea id="opportunity-start" rows={2} maxLength={2000} value={draft.meaningful_start} onChange={(event) => setDraft({ ...draft, meaningful_start: event.target.value })} aria-describedby="opportunity-start-hint" />
    </StudyField>
    <div className="study-grid"><StudyField id="opportunity-cue" label="Available work time" hint={`Local time: ${localTimezone()}. Observe the first 15 minutes after this cue.`}>
      <input id="opportunity-cue" type="datetime-local" value={draft.cue} onChange={(event) => setDraft({ ...draft, cue: event.target.value })} aria-describedby="opportunity-cue-hint" />
    </StudyField><StudyField id="opportunity-minutes" label="Available time (estimated minutes)"><input id="opportunity-minutes" type="number" min={1} max={1440} value={draft.available_minutes} onChange={(event) => setDraft({ ...draft, available_minutes: event.target.value })} /></StudyField></div>
    {!advanced && <p className="muted">Ordinary use, outside a study. Confirm an occasion when you have available time and intend to attempt this work. No schedule setup is needed.</p>}
    {advanced && <p className="muted">Logging context: {draft.manifest_id ? `${manifests.find((item) => item.id === draft.manifest_id)?.phase.toUpperCase()} protocol selected` : "Ordinary use outside a study"}. Select a study explicitly under the options below to declare a protocol opportunity.</p>}
    {advanced && <><label className="check-label"><input type="checkbox" checked={draft.eligible} onChange={(event) => setDraft({ ...draft, eligible: event.target.checked })} />I have available time and intend to attempt this work.</label>
    {!draft.eligible && <StudyField id="opportunity-unavailable" label="Known unavailability"><input id="opportunity-unavailable" value={draft.unavailability} maxLength={2000} onChange={(event) => setDraft({ ...draft, unavailability: event.target.value })} /></StudyField>}
    <details className="study-options"><summary>Schedule, study, and optional context</summary>
      <StudyField id="opportunity-epoch" label="Schedule epoch"><select id="opportunity-epoch" value={draft.epoch_id} onChange={(event) => setDraft({ ...draft, epoch_id: event.target.value })}>
        {epochs.map((epoch) => <option key={epoch.id} value={epoch.id}>{epoch.label}</option>)}
      </select></StudyField>
      <StudyField id="opportunity-study" label="Logging context"><select id="opportunity-study" value={draft.manifest_id} onChange={(event) => { const manifest = manifests.find((item) => item.id === event.target.value); setDraft({ ...draft, manifest_id: event.target.value, epoch_id: manifest?.epoch_id ?? draft.epoch_id }); }}>
        <option value="">Ordinary use — outside a study</option>{manifests.filter((item) => item.status === "active").map((item) => <option key={item.id} value={item.id}>{item.phase.toUpperCase()} · active</option>)}
      </select></StudyField>
      <StudyField id="opportunity-domain" label="Work domain"><input id="opportunity-domain" value={draft.domain} maxLength={120} onChange={(event) => setDraft({ ...draft, domain: event.target.value })} /></StudyField>
      <StudyField id="opportunity-priority" label="Declared priority"><select id="opportunity-priority" value={draft.priority} onChange={(event) => setDraft({ ...draft, priority: event.target.value as "ordinary" | "high" })}><option value="ordinary">Ordinary</option><option value="high">High</option></select></StudyField>
      <StudyField id="opportunity-deadline" label="External deadline (optional)"><input id="opportunity-deadline" type="datetime-local" value={draft.deadline} onChange={(event) => setDraft({ ...draft, deadline: event.target.value })} /></StudyField>
      <StudyField id="opportunity-context" label="Energy or support context (optional)" hint="A personal note, not a clinical measure. Low motivation does not automatically make an opportunity ineligible."><input id="opportunity-context" maxLength={2000} value={draft.context} onChange={(event) => setDraft({ ...draft, context: event.target.value })} /></StudyField>
    </details></>}
    <button type="button" className="text-button small" onClick={() => { setAdvanced(!advanced); queueMicrotask(() => document.getElementById("opportunity-start")?.focus()); }}>{advanced ? "Use ordinary planning" : "Advanced schedule and study options"}</button>
    <div className="button-row"><button type="submit" disabled={busy}>Confirm opportunity</button><button type="button" className="secondary" disabled={busy} onClick={onClose}>Discard opportunity draft</button></div>
  </form>;
}
