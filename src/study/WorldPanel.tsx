import { useEffect, useRef, useState } from "react";
import type { Workspace, ExposureKind } from "../platform/client";
import * as api from "../platform/client";
import { HorizonScene } from "../world/HorizonScene";
import type { StudyMutation } from "./OpportunityEditor";
import { useVisible } from "./useVisible";

export default function WorldPanel({ workspace, busy, onMutation }: { workspace: Workspace; busy: boolean; onMutation: StudyMutation }) {
  const [open, setOpen] = useState(false);
  const [voluntarySession, setVoluntarySession] = useState(0);
  const [failedDelivery, setFailedDelivery] = useState("");
  const rendered = useRef<HTMLDivElement>(null);
  const attempted = useRef(new Set<string>());
  const voluntaryBaseline = useRef(new Set<string>());
  const study = workspace.study;
  const active = study.manifests.find((item) => item.status === "active");
  const experimental = active?.phase === "e01";
  const presentation = study.presentation;
  const disabled = !presentation.enabled;
  const assignedVisible = experimental && presentation.world_allowed && presentation.boundary !== "none" && !disabled;
  const voluntaryVisible = !experimental && open;
  const showScene = assignedVisible || voluntaryVisible;
  const visible = useVisible(rendered, showScene);
  const kind: ExposureKind = experimental ? presentation.boundary === "preview" ? "preview_delivered" : "checkpoint_delivered" : "voluntary_world";
  const alreadyDelivered = experimental ? study.exposures.some((item) => item.kind === kind && item.opportunity_id === presentation.opportunity_id)
    : study.exposures.some((item) => item.kind === "voluntary_world" && !voluntaryBaseline.current.has(item.id));
  useEffect(() => {
    if (!showScene || !visible || busy || alreadyDelivered) return;
    const key = experimental ? `${presentation.opportunity_id}:${kind}` : `voluntary:${voluntarySession}`;
    if (attempted.current.has(key) || failedDelivery === `${key}:${workspace.generation}`) return;
    attempted.current.add(key);
    // Effects run after the scene is committed to the DOM. Rendering is delivery,
    // never evidence that a person noticed the presentation.
    void onMutation(() => api.recordStudyExposure({ expected_generation: workspace.generation,
      opportunity_id: experimental ? presentation.opportunity_id : null, kind,
      note: experimental ? null : active?.phase === "e00" ? "User opened an optional nonexperimental construction view during E00." : "User opened optional construction outside an experiment.",
    }), "Construction delivery recorded locally; noticing it is a separate self-report.").then((saved) => { if (!saved) { attempted.current.delete(key); setFailedDelivery(`${key}:${workspace.generation}`); } else setFailedDelivery(""); });
  }, [showScene, visible, busy, alreadyDelivered, experimental, presentation.opportunity_id, kind, voluntarySession, workspace.generation, active?.phase, onMutation, failedDelivery]);
  return <section className="world-panel" aria-labelledby="world-heading">
    <div className="section-heading"><div><p className="eyebrow">The Beacon Commonwealth</p><h2 id="world-heading">Optional construction</h2></div>
      {!experimental ? <button type="button" className="secondary" onClick={() => { setOpen(!open); if (!open) { voluntaryBaseline.current = new Set(study.exposures.map((item) => item.id)); setVoluntarySession((value) => value + 1); } }}>{open ? "Hide construction" : "View construction"}</button>
        : <button type="button" className="secondary" onClick={() => void onMutation(() => api.setConstructionEnabled({ expected_generation: workspace.generation, enabled: disabled }), disabled ? "Assigned construction enabled. Declared timing still applies." : "Optional construction disabled on this device. Your preference is saved.")}>{disabled ? "Allow assigned construction" : "Hide optional construction"}</button>}
    </div>
    {(experimental || open) && <><p className="numeric-progress">{workspace.progress.acceptances.filter((item) => item.status === "accepted").length} currently accepted accomplishments · {workspace.progress.wallet_bc} issued Build Credits · {Math.min(64, workspace.progress.wallet_bc)} / 64 restoration units.</p>
    <p className="muted">The same numeric feedback is available in both conditions. Construction reflects committed issuance, including retained fictional credit after corrections. It never spends credits or grants rewards.</p></>}
    {experimental && <p>{disabled ? "Construction is hidden by your choice. Work and numeric feedback remain available." : presentation.boundary === "none" ? "Construction stays hidden outside the declared presentation boundaries." : presentation.condition === "d" ? "This assigned opportunity uses conventional support with numeric feedback." : presentation.boundary === "preview" ? "Assigned brief construction preview before the cue." : "Assigned construction at the normal progress checkpoint."}</p>}
    {active?.phase === "e00" && open && <p className="study-context-label">Nonexperimental exposure during E00. This optional view is recorded separately from feasibility logging.</p>}
    {showScene && <div ref={rendered}><HorizonScene issuedBC={workspace.progress.wallet_bc} preview={experimental && presentation.boundary === "preview"} />
      {failedDelivery && !alreadyDelivered && <div className="error-banner"><p>Saving presentation delivery has not been confirmed. Your work remains available.</p><button type="button" className="secondary" disabled={busy} onClick={() => void onMutation(api.loadWorkspace, "Saved delivery records checked.").then((loaded) => { if (loaded) setFailedDelivery(""); })}>Check and retry delivery record</button></div>}
      {experimental && <button type="button" className="secondary small" disabled={busy || study.exposures.some((item) => item.kind === "seen" && item.opportunity_id === presentation.opportunity_id)} onClick={() => void onMutation(() => api.recordStudyExposure({ expected_generation: workspace.generation, opportunity_id: presentation.opportunity_id, kind: "seen", note: "User reports noticing the assigned presentation." }), "Presentation noticing recorded as your self-report.")}>I noticed this presentation</button>}
    </div>}
    {experimental && disabled && presentation.condition === "d_plus_b" && presentation.boundary !== "none" && !study.exposures.some((item) => item.opportunity_id === presentation.opportunity_id && item.kind === "deviation") && <button type="button" className="secondary small" disabled={busy} onClick={() => void onMutation(() => api.recordStudyExposure({ expected_generation: workspace.generation, opportunity_id: presentation.opportunity_id, kind: "deviation", note: "User chose to hide the assigned optional construction presentation." }), "Presentation deviation recorded. The original assignment is retained.")}>Record hidden-presentation deviation</button>}
  </section>;
}
