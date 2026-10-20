import { useEffect, useState, type FormEvent } from "react";
import * as api from "../platform/client";
import { localDateTime, localTimezone, StudyField, utcDateTime } from "./fields";
import type { StudyMutation } from "./OpportunityEditor";

export const E01_ACKNOWLEDGEMENTS = [
  ["e00_assessed", "I assessed E00 using ordinary human observations and recorded the feasibility assessment below."],
  ["frozen_protocol", "I reviewed the frozen protocol, planned checkpoints, and approximate 6–8 week / 30–60 natural-opportunity planning range. These are not quotas."],
  ["schedule_epoch", "The selected schedule epoch describes the current routine. A material change requires an explicit pause and protocol decision."],
  ["assignment_and_window", "I understand balanced blocks of four, two D and two D + construction, with eligibility and meaningful-start definitions locked before assignment and a 15-minute window."],
  ["matched_support", "Both conditions retain identical work support, numeric feedback, and rewards. Construction is the only addition: a 60-second pre-cue preview and a checkpoint after the window."],
  ["as_assigned_analysis", "Analysis includes all originally eligible assigned opportunities, including missing outcomes and delivery failures. Future allocations stay hidden; comparisons wait for planned review."],
  ["voluntary_stop", "I am the owner and explicitly authorize activation when I press Activate E01. I can pause or leave at any time, without losing work or fictional progress."],
] as const;

type ManifestSummary = api.StudyManifest;
interface Props { workspace: api.Workspace; epochs: { id: string; label: string; effective_from: string }[]; manifests: ManifestSummary[]; busy: boolean; onMutation: StudyMutation; onDirtyChange: (dirty: boolean) => void }
export default function StudySettings({ workspace, epochs, manifests, busy, onMutation, onDirtyChange }: Props) {
  const [epochLabel, setEpochLabel] = useState("");
  const [epochDate, setEpochDate] = useState(localDateTime());
  const [selectedEpoch, setSelectedEpoch] = useState(epochs[epochs.length - 1]?.id ?? "");
  const [phase, setPhase] = useState<api.StudyPhase>("e00");
  const [reason, setReason] = useState("");
  const [checks, setChecks] = useState<string[]>([]);
  const [error, setError] = useState("");
  const [editingManifest, setEditingManifest] = useState<string | null>(null);
  useEffect(() => { onDirtyChange(!!epochLabel || editingManifest !== null); }, [epochLabel, editingManifest, onDirtyChange]);
  const change = () => onDirtyChange(true);
  async function addEpoch(event: FormEvent) {
    event.preventDefault();
    const effective_from = utcDateTime(epochDate);
    if (!epochLabel.trim() || !effective_from) { setError("Give the schedule epoch a label and effective date."); document.getElementById("epoch-label")?.focus(); return; }
    const saved = await onMutation(() => api.createScheduleEpoch({ expected_generation: workspace.generation, label: epochLabel, effective_from, timezone: localTimezone() }), "Schedule epoch saved. Existing opportunity histories retain their original epoch.");
    if (saved) { setEpochLabel(""); setError(""); onDirtyChange(false); }
  }
  async function transition(manifest: ManifestSummary, status: api.TransitionStudyInput["status"]) {
    const requiresAcknowledgements = manifest.phase === "e01" && (status === "ready" || status === "active");
    if (requiresAcknowledgements && (checks.length !== E01_ACKNOWLEDGEMENTS.length || !reason.trim())) { setError("Review every acknowledgement and record the E00 feasibility assessment before continuing."); return; }
    if (!reason.trim() && status !== "active") { setError("Record the reason or assessment for this lifecycle decision."); document.getElementById("study-decision")?.focus(); return; }
    const saved = await onMutation(() => api.transitionStudy({ expected_generation: workspace.generation, manifest_id: manifest.id, status,
      reason: reason || "Explicitly beginning E00 feasibility logging.", acknowledgements: requiresAcknowledgements ? checks : [],
    }), `${manifest.phase.toUpperCase()} is now ${status}. Prior observations and assignments are retained.`);
    if (saved) { setReason(""); setChecks([]); setEditingManifest(null); setError(""); onDirtyChange(false); }
  }
  return <details className="study-settings"><summary>Schedules and optional studies</summary>
    <p>No study starts automatically. Ordinary opportunity logging works without enrollment. E00 assesses logging feasibility; it does not test whether Ascension improves productivity.</p>
    {error && <p role="alert" className="field-error">{error}</p>}
    <form className="study-editor" onSubmit={(event) => void addEpoch(event)}>
      <h3>Declare a schedule epoch</h3><p className="muted">Use a short, nonconfidential label for comparable routines. A material change during E01 pauses new assignment; prior assignments stay in history.</p>
      <div className="study-grid"><StudyField id="epoch-label" label="Schedule label"><input id="epoch-label" maxLength={120} value={epochLabel} onChange={(event) => { setEpochLabel(event.target.value); change(); }} placeholder="Ordinary routine" /></StudyField>
      <StudyField id="epoch-date" label="Effective from (local time)"><input id="epoch-date" type="datetime-local" value={epochDate} onChange={(event) => { setEpochDate(event.target.value); change(); }} /></StudyField></div>
      <button type="submit" className="secondary" disabled={busy || !epochLabel.trim()}>Save schedule epoch</button>
    </form>
    {epochs.length > 0 && <section className="study-editor"><h3>Configure optional logging</h3>
      <div className="study-grid"><StudyField id="configure-epoch" label="Study schedule epoch"><select id="configure-epoch" value={selectedEpoch || epochs[epochs.length - 1].id} onChange={(event) => setSelectedEpoch(event.target.value)}>{epochs.map((epoch) => <option key={epoch.id} value={epoch.id}>{epoch.label}</option>)}</select></StudyField>
      <StudyField id="configure-phase" label="Phase"><select id="configure-phase" value={phase} onChange={(event) => setPhase(event.target.value as api.StudyPhase)}><option value="e00">E00 — feasibility logging</option><option value="e01">E01 — construction comparison readiness</option></select></StudyField></div>
      <p className="muted">E00 planning range: 5–10 ordinary workdays, with no compulsory quota. E01 configuration freezes a reproducible sequence; configuration does not activate it.</p>
      <button type="button" className="secondary" disabled={busy} onClick={() => void onMutation(() => api.configureStudy({ expected_generation: workspace.generation, phase, epoch_id: selectedEpoch || epochs[epochs.length - 1].id }), `${phase.toUpperCase()} configured. It has not been activated.`)}>Configure {phase.toUpperCase()}</button>
    </section>}
    {manifests.map((manifest) => <section key={manifest.id} data-study-manifest-id={manifest.id} className="study-editor"><div className="section-heading"><h3>{manifest.phase.toUpperCase()} · {manifest.status}</h3><span className="muted">{epochs.find((epoch) => epoch.id === manifest.epoch_id)?.label}</span></div>
      {manifest.phase === "e01" && <p>Future allocations are hidden. E01 needs actual E00 assessment and separate explicit owner authorization. Engineering tests do not establish human feasibility.</p>}
      <details><summary>Frozen protocol and decision history</summary>
        <p>Protocol {manifest.protocol_version} · algorithm {manifest.algorithm} · block size {manifest.block_size} · {manifest.window_seconds}-second observation window.</p>
        <p className="preserve-text">Allocation hash: {manifest.allocation_hash}</p><p>Presentation policy: {manifest.presentation_policy}. {manifest.reveal_rule}</p>
        <p className="preserve-text">Analysis: {manifest.analysis_definition}. Planned review checkpoints: {manifest.review_after.join(", ")} assignments.</p>
        <p className="muted">The complete reproducible seed and allocation are preserved in export. Future conditions remain hidden here. There is no background cue delivery when Ascension is closed.</p>
        {workspace.study.protocol_events.filter((item) => item.manifest_id === manifest.id).sort((a, b) => a.sequence - b.sequence).map((item) => <p key={item.id}>{item.sequence}. {item.status}: {item.reason}</p>)}
      </details>
      {!(["completed", "abandoned"] as api.StudyStatus[]).includes(manifest.status) && editingManifest !== manifest.id && <button type="button" className="secondary" disabled={busy} onClick={() => { setEditingManifest(manifest.id); setReason(""); setChecks([]); change(); }}>Review {manifest.phase.toUpperCase()} lifecycle</button>}
      {editingManifest === manifest.id && <>
        <StudyField id="study-decision" label={manifest.phase === "e01" ? "E00 feasibility assessment and protocol decision" : "Feasibility assessment or decision (optional when beginning)"}><textarea id="study-decision" rows={3} maxLength={2000} value={reason} onChange={(event) => { setReason(event.target.value); change(); }} /></StudyField>
        {manifest.phase === "e01" && ["configured", "ready", "paused"].includes(manifest.status) && <fieldset className="e01-acknowledgements"><legend>Owner review before readiness or activation</legend>{E01_ACKNOWLEDGEMENTS.map(([key, label]) => <label className="check-label" key={key}><input type="checkbox" checked={checks.includes(key)} onChange={(event) => { setChecks(event.target.checked ? [...checks, key] : checks.filter((item) => item !== key)); change(); }} />{label}</label>)}</fieldset>}
        <div className="button-row">
          {manifest.phase === "e01" && manifest.status === "configured" && <button type="button" disabled={busy || checks.length !== 7 || !reason.trim()} onClick={() => void transition(manifest, "ready")}>Confirm E01 readiness</button>}
          {(manifest.phase === "e00" && ["configured", "ready", "paused"].includes(manifest.status) || manifest.phase === "e01" && ["ready", "paused"].includes(manifest.status)) && <button type="button" disabled={busy || (manifest.phase === "e01" && (checks.length !== 7 || !reason.trim()))} onClick={() => void transition(manifest, "active")}>{manifest.phase === "e01" ? "Activate E01 with owner approval" : "Begin E00 feasibility logging"}</button>}
          {manifest.status === "active" && <button type="button" className="secondary" disabled={busy} onClick={() => void transition(manifest, "paused")}>Pause new assignment</button>}
          {["active", "paused"].includes(manifest.status) && <button type="button" className="secondary" disabled={busy} onClick={() => void transition(manifest, "completed")}>Complete this phase</button>}
          <button type="button" className="secondary" disabled={busy} onClick={() => void transition(manifest, "abandoned")}>Leave this study</button>
          <button type="button" className="secondary" onClick={() => { setEditingManifest(null); setReason(""); setChecks([]); onDirtyChange(false); }}>Discard lifecycle draft</button>
        </div>
      </>}
    </section>)}
  </details>;
}
