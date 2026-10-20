import { useEffect, useRef, useState, type FormEvent } from "react";
import * as api from "../platform/client";
import type { Assistance, Criterion, EvidenceKind, EvidencePolicy, EvidenceStatus, Objective, Workspace } from "../platform/client";

interface Props {
  workspace: Workspace;
  objective: Objective;
  busy: boolean;
  onMutation: (task: () => Promise<Workspace>, message: string) => Promise<boolean>;
  onDirtyChange: (dirty: boolean) => void;
}
type Mode = "criterion" | "evidence" | "review" | "correction" | null;
const policyNames: Record<EvidencePolicy, string> = {
  user_attestation: "Personal attestation", referenced_evidence: "Referenced evidence", trusted_validation: "Trusted validation",
};
const kindNames: Record<EvidenceKind, string> = {
  user_attestation: "Personal attestation", self_report: "Self-report", artifact_reference: "Artifact reference",
  imported_metadata: "Imported metadata", registered_validator: "Registered validator", third_party_claim: "Third-party claim",
};
const assistanceNames: Record<Assistance, string> = { none: "Independent work", assisted: "Assisted work", generated: "Generated work" };
const nullable = (value: string) => value.trim() || null;
const emptyCriterion = {
  criterion_lineage_id: null as string | null, description: "", allocation_bc: 1, evidence_policy: "user_attestation" as EvidencePolicy,
  scope: "parent outcome", artifact_id: "", artifact_version: "", requires_independent_work: false, required_validators: [] as string[],
};
const emptyEvidence = {
  kind: "user_attestation" as EvidenceKind, description: "", reference: "", observed_at: "", status: "supports" as EvidenceStatus,
  assistance: "none" as Assistance, uncertainty: "", supersedes: "",
};

function AssistanceField({ id, value, onChange }: { id: string; value: Assistance; onChange: (value: Assistance) => void }) {
  return <div className="field"><label htmlFor={id}>How was the work produced?</label>
    <select id={id} value={value} onChange={(event) => onChange(event.target.value as Assistance)}>
      {Object.entries(assistanceNames).map(([key, label]) => <option key={key} value={key}>{label}</option>)}
    </select></div>;
}

export default function ProgressPanel({ workspace, objective, busy, onMutation, onDirtyChange }: Props) {
  const progress = workspace.progress;
  const criteria = progress.criteria.filter((item) => item.parent_id === objective.id);
  const latest = criteria.filter((item) => !criteria.some((other) => other.criterion_lineage_id === item.criterion_lineage_id && other.criterion_version > item.criterion_version));
  const [lineage, setLineage] = useState("");
  const criterion = latest.find((item) => item.criterion_lineage_id === lineage) ?? latest[0];
  const slot = progress.slots.find((item) => item.id === criterion?.reward_slot_id);
  const availableAllocation = 4 - progress.slots.filter((item) => item.parent_id === objective.id).reduce((total, item) => total + item.allocation_bc, 0);
  const [showRewards, setShowRewards] = useState(false);
  const [mode, setMode] = useState<Mode>(null);
  const [criterionDraft, setCriterionDraft] = useState(emptyCriterion);
  const [evidenceDraft, setEvidenceDraft] = useState(emptyEvidence);
  const [summary, setSummary] = useState("");
  const [assistance, setAssistance] = useState<Assistance>("none");
  const [references, setReferences] = useState<string[]>([]);
  const [correction, setCorrection] = useState({ acceptance_id: "", kind: "incorrect_acceptance" as api.CorrectionKind, reason: "" });
  const [rejectReason, setRejectReason] = useState("");
  const [selectedSnapshotId, setSelectedSnapshotId] = useState("");
  const [lockConfirmed, setLockConfirmed] = useState(false);
  const [formError, setFormError] = useState("");
  const [acceptanceFeedback, setAcceptanceFeedback] = useState("");
  const formRef = useRef<HTMLFormElement>(null);
  const intent = useRef<api.AcceptProgressInput | null>(null);
  const evidence = progress.evidence.filter((item) => item.criterion_id === criterion?.id);
  const currentEvidence = evidence.filter((item) => !evidence.some((other) => other.supersedes === item.id));
  const snapshots = progress.snapshots.filter((item) => item.criterion_id === criterion?.id);
  const snapshot = snapshots.find((item) => item.id === selectedSnapshotId) ?? snapshots.sort((a, b) =>
    a.registry_revision - b.registry_revision || Number(a.disposition === "pending") - Number(b.disposition === "pending")
    || a.reviewed_at.localeCompare(b.reviewed_at) || a.id.localeCompare(b.id)).slice(-1)[0];
  const stale = !!snapshot && snapshot.registry_revision !== progress.registry_revision;
  const acceptances = progress.acceptances.filter((item) => item.parent_id === objective.id);
  const dirty = mode !== null || !!rejectReason;

  useEffect(() => { onDirtyChange(dirty); }, [dirty, onDirtyChange]);
  useEffect(() => {
    if (mode) formRef.current?.querySelector<HTMLElement>("input, textarea, select")?.focus();
  }, [mode]);
  useEffect(() => { if (formError) document.getElementById("progress-form-error")?.scrollIntoView?.({ block: "nearest" }); }, [formError]);

  function closeDraft() { setMode(null); setFormError(""); }
  function editCriterion(item?: Criterion) {
    setAcceptanceFeedback("");
    setCriterionDraft(item ? {
      criterion_lineage_id: item.criterion_lineage_id, description: item.description,
      allocation_bc: progress.slots.find((candidate) => candidate.id === item.reward_slot_id)?.allocation_bc ?? 0,
      evidence_policy: item.evidence_policy, scope: item.scope, artifact_id: item.artifact_id ?? "", artifact_version: item.artifact_version ?? "",
      requires_independent_work: item.requires_independent_work, required_validators: item.required_validators,
    } : { ...emptyCriterion, description: objective.criterion, allocation_bc: Math.max(1, availableAllocation) });
    setMode("criterion"); setFormError("");
  }
  function startReview() {
    setAcceptanceFeedback("");
    setSummary(""); setAssistance("none"); setReferences(currentEvidence.map((item) => item.id)); setMode("review"); setFormError("");
  }
  async function saveDraft(event: FormEvent) {
    event.preventDefault();
    if (busy || !mode) return;
    setAcceptanceFeedback("");
    setFormError("");
    const expected_generation = workspace.generation;
    if (mode === "criterion") {
      if (!criterionDraft.description.trim() || !criterionDraft.scope.trim()) { setFormError("Describe the criterion and its applicable scope before saving."); return; }
      if (!Number.isInteger(criterionDraft.allocation_bc) || criterionDraft.allocation_bc < 1 || criterionDraft.allocation_bc > 4) { setFormError("The declared allocation must be a whole number from one through four."); return; }
      const saved = await onMutation(() => api.saveCriterion({ ...criterionDraft, expected_generation, parent_id: objective.id,
        artifact_id: nullable(criterionDraft.artifact_id), artifact_version: nullable(criterionDraft.artifact_version) }), "Criterion draft saved. It is not eligible for acceptance until locked.");
      if (saved) closeDraft();
    } else if (mode === "evidence" && criterion) {
      if (!evidenceDraft.description.trim()) { setFormError("Describe the observed progress or evidence before saving."); return; }
      const saved = await onMutation(() => api.recordEvidence({ ...evidenceDraft, expected_generation, parent_id: objective.id,
        criterion_lineage_id: criterion.criterion_lineage_id, criterion_version: criterion.criterion_version,
        reference: nullable(evidenceDraft.reference), observed_at: nullable(evidenceDraft.observed_at), uncertainty: nullable(evidenceDraft.uncertainty), supersedes: nullable(evidenceDraft.supersedes),
      }), "Evidence saved with its source and assistance information. Saving evidence does not accept progress.");
      if (saved) { setEvidenceDraft(emptyEvidence); closeDraft(); }
    } else if (mode === "review" && criterion) {
      if (!summary.trim()) { setFormError("Summarize the factual progress you want to review."); return; }
      let createdSnapshotId = "";
      const saved = await onMutation(async () => {
        const result = await api.reviewProgress({ expected_generation, parent_id: objective.id,
          criterion_lineage_id: criterion.criterion_lineage_id, criterion_version: criterion.criterion_version,
          evidence_refs: references, factual_summary: summary, assistance });
        createdSnapshotId = result.snapshot.id;
        return result.workspace;
      }, "Review snapshot saved. Check all applicable evidence before deciding.");
      if (saved) { setSelectedSnapshotId(createdSnapshotId); intent.current = null; closeDraft(); }
    } else if (mode === "correction") {
      if (!correction.reason.trim()) { setFormError("Explain the factual correction before saving."); return; }
      const saved = await onMutation(() => api.correctAcceptance({ expected_generation, ...correction }), "Factual correction saved. Any existing fictional credit is retained; its original reward cannot be issued again.");
      if (saved) closeDraft();
    }
  }
  async function accept() {
    if (!snapshot || !criterion || busy) return;
    setAcceptanceFeedback("");
    if (!intent.current || intent.current.reviewed_snapshot_id !== snapshot.id) {
      intent.current = { schema_version: 1, command_id: crypto.randomUUID(), expected_generation: workspace.generation,
        parent_id: objective.id, criterion_lineage_id: criterion.criterion_lineage_id, criterion_version: criterion.criterion_version,
        expected_parent_revision: objective.revision, reviewed_snapshot_id: snapshot.id, reviewed_snapshot_digest: snapshot.digest,
        evidence_refs: snapshot.evidence_refs, factual_summary: snapshot.factual_summary, assistance: snapshot.assistance };
    }
    const input = intent.current;
    let currentStatus: api.Acceptance["status"] | undefined;
    const saved = await onMutation(async () => {
      const response = await api.acceptProgress(input);
      currentStatus = response.workspace.progress.acceptances.find((item) => item.id === response.result.acceptance_id)?.status;
      return response.workspace;
    }, "Acceptance request resolved. Saved history refreshed.");
    if (saved) {
      intent.current = null;
      setAcceptanceFeedback(currentStatus === "accepted" ? "Progress accepted and saved on this device."
        : currentStatus === "retracted" ? "The original request was already recorded. Its acceptance is now retracted; the correction remains in effect."
          : currentStatus === "historical" ? "The original request was already recorded. Its historical accomplishment is now reopened; the correction remains in effect."
            : "The request was resolved. Review the saved accomplishment history for its current status.");
    }
  }

  return <section className="progress-panel" aria-labelledby="progress-heading">
    <details>
      <summary id="progress-heading">Progress and evidence</summary>
      {acceptanceFeedback && <p role="status" className="notice">{acceptanceFeedback}</p>}
      <p>Record what happened, review the declared criterion, and decide whether the evidence supports acceptance. Your next action remains independent of this review.</p>
      <label className="check-label"><input type="checkbox" checked={showRewards} onChange={(event) => setShowRewards(event.target.checked)} />Show fictional rewards</label>
      {showRewards && <div className="reward-summary"><p><strong>{progress.wallet_bc} Build Credits</strong> from the saved ledger.</p>
        <p>Each parent objective has a lifetime limit of four BC. Corrections retain fictional credit; retained credit is not proof of accepted work.</p></div>}
      {criterion ? <>
        <div className="field"><label htmlFor="progress-criterion">Criterion to review</label>
          <select id="progress-criterion" value={criterion.criterion_lineage_id} disabled={!!mode || !!rejectReason} onChange={(event) => { setLineage(event.target.value); setLockConfirmed(false); setSelectedSnapshotId(""); intent.current = null; }}>
            {latest.map((item, index) => <option key={item.id} value={item.criterion_lineage_id}>{index + 1}. {item.description.slice(0, 70)} — version {item.criterion_version}</option>)}
          </select></div>
        <div className="criterion-summary">
          <h3>{criterion.state === "locked" ? "Locked criterion" : "Criterion draft"} · version {criterion.criterion_version}</h3>
          <p className="preserve-text">{criterion.description}</p>
          <p>{policyNames[criterion.evidence_policy]} · Scope: {criterion.scope}</p>
          {criterion.artifact_id && <p>Artifact: {criterion.artifact_id} · version {criterion.artifact_version ?? "unspecified"}</p>}
          {criterion.requires_independent_work && <p>This criterion requires independent work.</p>}
          {showRewards && <p>Declared allocation: {slot?.allocation_bc ?? 0} BC. {slot?.allocation_locked ? "This allocation is locked for the existing reward identity." : "The allocation remains a draft until locking."}</p>}
          {criterion.evidence_policy === "trusted_validation" && <p>No external validator connector runs in this stage. Required trusted evidence that is unavailable keeps the review pending.</p>}
          {criterion.state === "draft" && <p>Existing objective text is a draft, not evidence of prior accomplishment. Lock the criterion before recording eligible work.</p>}
          {criterion.state === "draft" && !mode && <>
            <label className="check-label"><input type="checkbox" checked={lockConfirmed} onChange={(event) => setLockConfirmed(event.target.checked)} />I have reviewed this criterion and am declaring it before eligible work.</label>
            <button type="button" disabled={!lockConfirmed || busy} onClick={() => void onMutation(() => api.lockCriterion({ expected_generation: workspace.generation, parent_id: objective.id,
              criterion_lineage_id: criterion.criterion_lineage_id, criterion_version: criterion.criterion_version }), "Criterion locked. Later changes require a new version; historical evidence is retained.").then((saved) => { if (saved) setLockConfirmed(false); })}>Lock criterion</button>
          </>}
        </div>
        {!mode && <div className="button-row">
          <button type="button" className="secondary" disabled={!!rejectReason} onClick={() => editCriterion(criterion)}>{criterion.state === "locked" ? "Prepare next criterion version" : "Edit criterion draft"}</button>
          <button type="button" className="secondary" disabled={availableAllocation < 1 || !!rejectReason} onClick={() => editCriterion()}>Add meaningful criterion</button>
          {criterion.state === "locked" && <>
            <button type="button" disabled={!!rejectReason} onClick={() => { setEvidenceDraft(emptyEvidence); setMode("evidence"); setFormError(""); }}>Record evidence</button>
            <button type="button" className="secondary" disabled={!!rejectReason} onClick={startReview}>Review progress</button>
          </>}
        </div>}
        {showRewards && !mode && availableAllocation < 1 && <p className="muted">The four-BC budget is fully allocated. To split it across meaningful criteria, reduce an existing draft allocation before locking. Locked allocations cannot be reassigned.</p>}
      </> : <><p>No criterion version has been declared for this objective.</p><button type="button" onClick={() => editCriterion()}>Prepare criterion</button></>}

      {mode && <form className="progress-editor" onSubmit={(event) => void saveDraft(event)} ref={formRef} noValidate>
        <h3>{mode === "criterion" ? "Prepare criterion draft" : mode === "evidence" ? "Record observed evidence" : mode === "review" ? "Prepare progress review" : "Correct an accomplishment"}</h3>
        <p className="draft-label">Unsaved draft</p>
        {formError && <p id="progress-form-error" role="alert" className="field-error">{formError}</p>}
        {mode === "criterion" && <>
          <p>Changed locked criteria become a new version under the same reward identity. They do not create a fresh reward or replace historical acceptance.</p>
          <div className="field"><label htmlFor="progress-description">Acceptance criterion</label><textarea id="progress-description" rows={3} value={criterionDraft.description} onChange={(event) => setCriterionDraft({ ...criterionDraft, description: event.target.value })} /></div>
          <div className="field"><label htmlFor="progress-policy">Required evidence policy</label><select id="progress-policy" value={criterionDraft.evidence_policy} onChange={(event) => setCriterionDraft({ ...criterionDraft, evidence_policy: event.target.value as EvidencePolicy })}>
            {Object.entries(policyNames).map(([key, label]) => <option key={key} value={key}>{label}</option>)}
          </select></div>
          <div className="field"><label htmlFor="progress-scope">Applicable scope or environment</label><input id="progress-scope" value={criterionDraft.scope} onChange={(event) => setCriterionDraft({ ...criterionDraft, scope: event.target.value })} /></div>
          <div className="field"><label htmlFor="progress-artifact">Artifact identity (optional)</label><input id="progress-artifact" value={criterionDraft.artifact_id} onChange={(event) => setCriterionDraft({ ...criterionDraft, artifact_id: event.target.value })} /></div>
          <div className="field"><label htmlFor="progress-artifact-version">Artifact version (optional)</label><input id="progress-artifact-version" value={criterionDraft.artifact_version} onChange={(event) => setCriterionDraft({ ...criterionDraft, artifact_version: event.target.value })} /></div>
          <label className="check-label"><input type="checkbox" checked={criterionDraft.requires_independent_work} onChange={(event) => setCriterionDraft({ ...criterionDraft, requires_independent_work: event.target.checked })} />Independent work is required</label>
          {showRewards && <div className="field"><label htmlFor="progress-allocation">Predeclared Build Credit allocation</label><input id="progress-allocation" type="number" min={1} max={4} step={1} value={criterionDraft.allocation_bc}
            disabled={!!criterionDraft.criterion_lineage_id && progress.slots.some((item) => item.criterion_lineage_id === criterionDraft.criterion_lineage_id && item.allocation_locked)}
            onChange={(event) => setCriterionDraft({ ...criterionDraft, allocation_bc: Number(event.target.value) })} /><p className="muted">All criteria share the parent's four-BC lifetime budget. A consumed allocation cannot mint again.</p></div>}
        </>}
        {mode === "evidence" && <>
          <p>These are user-recorded claims. References and imported metadata are not automatically trusted validation. No files are opened or executed.</p>
          <div className="field"><label htmlFor="evidence-kind">Evidence source</label><select id="evidence-kind" value={evidenceDraft.kind} onChange={(event) => setEvidenceDraft({ ...evidenceDraft, kind: event.target.value as EvidenceKind })}>
            {Object.entries(kindNames).filter(([key]) => key !== "registered_validator").map(([key, label]) => <option key={key} value={key}>{label}</option>)}
          </select></div>
          <div className="field"><label htmlFor="evidence-description">What happened or was observed?</label><textarea id="evidence-description" rows={3} value={evidenceDraft.description} onChange={(event) => setEvidenceDraft({ ...evidenceDraft, description: event.target.value })} /></div>
          <div className="field"><label htmlFor="evidence-reference">Reference or source description (optional)</label><input id="evidence-reference" value={evidenceDraft.reference} onChange={(event) => setEvidenceDraft({ ...evidenceDraft, reference: event.target.value })} /></div>
          <div className="field"><label htmlFor="evidence-status">Relationship to the criterion</label><select id="evidence-status" value={evidenceDraft.status} onChange={(event) => setEvidenceDraft({ ...evidenceDraft, status: event.target.value as EvidenceStatus })}>
            <option value="supports">Supports</option><option value="contradicts">Contradicts</option><option value="uncertain">Uncertain</option>
          </select></div>
          <AssistanceField id="evidence-assistance" value={evidenceDraft.assistance} onChange={(value) => setEvidenceDraft({ ...evidenceDraft, assistance: value })} />
          <div className="field"><label htmlFor="evidence-uncertainty">Uncertainty or limitations (optional)</label><textarea id="evidence-uncertainty" rows={2} value={evidenceDraft.uncertainty} onChange={(event) => setEvidenceDraft({ ...evidenceDraft, uncertainty: event.target.value })} /></div>
          <div className="field"><label htmlFor="evidence-observed">Observation time (optional ISO 8601 timestamp)</label><input id="evidence-observed" placeholder="Leave blank for now" value={evidenceDraft.observed_at} onChange={(event) => setEvidenceDraft({ ...evidenceDraft, observed_at: event.target.value })} /></div>
          {currentEvidence.length > 0 && <div className="field"><label htmlFor="evidence-supersedes">Explicitly supersedes earlier evidence</label><select id="evidence-supersedes" value={evidenceDraft.supersedes} onChange={(event) => setEvidenceDraft({ ...evidenceDraft, supersedes: event.target.value })}>
            <option value="">None — add a separate observation</option>{currentEvidence.map((item) => <option key={item.id} value={item.id}>{item.description.slice(0, 90)}</option>)}
          </select><p className="muted">Earlier records remain in history. A newer timestamp alone does not resolve conflicting evidence.</p></div>}
        </>}
        {mode === "review" && <>
          <div className="field"><label htmlFor="review-summary">Factual progress to review</label><textarea id="review-summary" rows={3} value={summary} onChange={(event) => setSummary(event.target.value)} /></div>
          <AssistanceField id="review-assistance" value={assistance} onChange={setAssistance} />
          <fieldset className="evidence-selection"><legend>Evidence you identify</legend>
            {currentEvidence.length ? currentEvidence.map((item) => <label key={item.id} className="check-label"><input type="checkbox" checked={references.includes(item.id)} onChange={(event) => setReferences(event.target.checked ? [...references, item.id] : references.filter((id) => id !== item.id))} />{item.description} ({item.status})</label>) : <p>No evidence has been recorded for this version.</p>}
          </fieldset>
          <p>The backend also checks all applicable registered evidence and validators. Omitting a contradictory record cannot make the review eligible.</p>
        </>}
        {mode === "correction" && <>
          <div className="field"><label htmlFor="correction-kind">What needs correcting?</label><select id="correction-kind" value={correction.kind} onChange={(event) => setCorrection({ ...correction, kind: event.target.value as api.CorrectionKind })}>
            <option value="incorrect_acceptance">The original acceptance was incorrect</option><option value="historical_reopened">It was accurate then; the work is now reopened</option>
          </select></div>
          <div className="field"><label htmlFor="correction-reason">Factual correction reason</label><textarea id="correction-reason" rows={3} value={correction.reason} onChange={(event) => setCorrection({ ...correction, reason: event.target.value })} /></div>
          <p>Correction changes the factual record. Reacceptance requires a fresh review and cannot issue the original reward again.</p>
        </>}
        <div className="button-row"><button type="submit" disabled={busy}>{busy ? "Saving progress..." : mode === "criterion" ? "Save criterion draft" : mode === "evidence" ? "Save evidence" : mode === "review" ? "Save review snapshot" : "Save factual correction"}</button>
          <button type="button" className="secondary" disabled={busy} onClick={closeDraft}>Discard progress draft</button></div>
      </form>}

      {snapshot && !mode && <section className="review-snapshot" aria-labelledby="snapshot-heading">
        <h3 id="snapshot-heading">Saved evidence review</h3><p className="preserve-text">{snapshot.factual_summary}</p>
        <p>Version {snapshot.criterion_version} · {assistanceNames[snapshot.assistance]} · {snapshot.disposition}</p>
        {stale && <p className="field-error">Evidence or criteria changed after this review. Prepare a fresh review before acceptance.</p>}
        <p>{snapshot.eligible ? "The saved snapshot meets the declared evidence policy." : "This review is pending: the evidence does not yet satisfy the declared policy."}</p>
        {snapshot.missing.length > 0 && <div><h4>Missing evidence</h4><ul>{snapshot.missing.map((item, index) => <li key={index}>{item}</li>)}</ul></div>}
        {snapshot.conflicts.length > 0 && <div><h4>Conflicts</h4><ul>{snapshot.conflicts.map((item, index) => <li key={index}>{item}</li>)}</ul></div>}
        <h4>All applicable evidence</h4>
        {snapshot.applicable_evidence.length ? <ul>{snapshot.applicable_evidence.map((item) => <li key={item.id}><strong>{kindNames[item.kind]}: {item.status}</strong><p className="preserve-text">{item.description}</p>
          <p className="muted">Source: {item.source_provenance} · {assistanceNames[item.assistance]} · observed {item.observed_at}</p>{item.reference && <p className="preserve-text">Reference: {item.reference}</p>}{item.uncertainty && <p>Uncertainty: {item.uncertainty}</p>}</li>)}</ul> : <p>No applicable evidence.</p>}
        {snapshot.applicable_validators.length > 0 && <><h4>Applicable registered validators</h4><ul>{snapshot.applicable_validators.map((item) => <li key={item.id}>{item.id} · {item.source_provenance} · {item.available ? "available" : "missing"}</li>)}</ul></>}
        {snapshot.disposition === "pending" && <>
          <button type="button" disabled={!snapshot.eligible || stale || busy || !!rejectReason} onClick={() => void accept()}>Accept qualifying progress</button>
          <div className="field"><label htmlFor="review-reject-reason">Reason to reject this review (optional)</label><input id="review-reject-reason" value={rejectReason} onChange={(event) => setRejectReason(event.target.value)} /></div>
          <div className="button-row"><button type="button" className="secondary" disabled={!rejectReason.trim() || busy} onClick={() => void onMutation(() => api.rejectReview({ expected_generation: workspace.generation, snapshot_id: snapshot.id, reason: rejectReason }), "Review rejected. No accomplishment or reward was created.").then((saved) => { if (saved) setRejectReason(""); })}>Reject review</button>
            <button type="button" className="secondary" disabled={busy} onClick={() => setRejectReason("")}>Leave pending</button></div>
        </>}
        {snapshot.rejection_reason && <p>Rejection reason: {snapshot.rejection_reason}</p>}
      </section>}
      {acceptances.length > 0 && <section className="accomplishments" aria-labelledby="accomplishments-heading"><h3 id="accomplishments-heading">Accomplishment history</h3>
        <ul>{acceptances.map((item) => {
          const correctionRecord = progress.corrections.find((candidate) => candidate.id === item.correction_id);
          const grant = progress.grants.find((candidate) => candidate.reward_slot_id === item.reward_slot_id);
          return <li key={item.id}><p className="preserve-text"><strong>{item.status === "accepted" ? "Accepted" : item.status === "historical" ? "Historically accepted; reopened" : "Acceptance retracted"}</strong>: {item.factual_summary}</p>
            <p className="muted">Criterion version {item.criterion_version} · {assistanceNames[item.assistance]}</p>
            {correctionRecord && <p className="preserve-text">Correction: {correctionRecord.reason}</p>}
            {showRewards && grant && <p>{grant.amount_bc} BC {item.status === "accepted" ? "issued once for this reward identity" : "retained as fictional credit; this is not current accepted progress"}.</p>}
            {item.status === "accepted" && !mode && <button type="button" className="secondary small" disabled={!!rejectReason} onClick={() => { setCorrection({ acceptance_id: item.id, kind: "incorrect_acceptance", reason: "" }); setMode("correction"); setFormError(""); }}>Correct this accomplishment</button>}
          </li>;
        })}</ul></section>}
      {criteria.length > latest.length && <details className="criterion-history"><summary>Criterion version history</summary><ul>{criteria.map((item) => <li key={item.id}>Version {item.criterion_version} · {item.state}: {item.description}</li>)}</ul></details>}
    </details>
  </section>;
}
