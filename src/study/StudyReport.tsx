import type { StudyView, StudyManifest } from "../platform/client";
import { useState } from "react";
import { assignedSummary, differenceBounds, lowPressure, percent, reviewScope, summarize } from "./reporting";

function Summary({ value }: { value: ReturnType<typeof summarize> }) {
  return <><dl className="study-statistics">
    <div><dt>Planned / eligible / ineligible</dt><dd>{value.planned} / {value.eligible} / {value.ineligible}</dd></div>
    <div><dt>Ascertainable outcomes</dt><dd>{value.ascertainable} · {percent(value.completeness)} of all eligible planned</dd></div>
    <div><dt>Starts / nonstarts / missing / pending</dt><dd>{value.starts} / {value.nonstarts} / {value.missing} / {value.pending}</dd></div>
    <div><dt>Late starts / technical problems / corrections</dt><dd>{value.late} / {value.technical} / {value.corrections}</dd></div>
    <div><dt>Prospectively declared start definitions</dt><dd>{value.prospective} / {value.eligible}</dd></div>
    <div><dt>Median recorded capture estimate</dt><dd>{value.medianCaptureMinutes === null ? "No estimates" : `${value.medianCaptureMinutes} min`} · {value.captureEstimates} opportunities with planning/logging estimates</dd></div>
    <div><dt>Estimated administration / useful work</dt><dd>{value.managementMinutes} / {value.usefulWorkMinutes} minutes, recorded separately</dd></div>
    <div><dt>Priority advanced / reported</dt><dd>{value.priorityAdvanced} / {value.priorityReported} personal observations</dd></div>
    <div><dt>Domain quality notes / linked accomplishments</dt><dd>{value.qualityNotes} / {value.linkedAcceptanceIds.length} (current acceptance status remains in Progress and evidence)</dd></div>
  </dl><p className="muted">Capture estimates sum recorded planning and logging minutes per opportunity. Unreported categories are unknown; the median may understate burden. These are personal estimates, not activity monitoring.</p></>;
}
function ArmComparison({ study, manifest, epochId, filter }: { study: StudyView; manifest: StudyManifest; epochId: string; filter?: Parameters<typeof assignedSummary>[5] }) {
  const scope = reviewScope(study, manifest);
  const d = assignedSummary(study, manifest, "d", epochId, scope, filter);
  const b = assignedSummary(study, manifest, "d_plus_b", epochId, scope, filter);
  const bounds = differenceBounds(b, d);
  return <><div className="study-table-scroll"><table><caption>As-assigned descriptive outcomes</caption><thead><tr><th scope="col">Recorded measure</th><th scope="col">D</th><th scope="col">D + construction</th></tr></thead><tbody>
    <tr><th scope="row">All originally eligible assigned (N)</th><td>{d.eligible}</td><td>{b.eligible}</td></tr>
    <tr><th scope="row">Observed starts (S)</th><td>{d.starts}</td><td>{b.starts}</td></tr>
    <tr><th scope="row">Observed nonstarts</th><td>{d.nonstarts}</td><td>{b.nonstarts}</td></tr>
    <tr><th scope="row">Missing (M) / pending</th><td>{d.missing} / {d.pending}</td><td>{b.missing} / {b.pending}</td></tr>
    <tr><th scope="row">Possible start rate: S/N to (S+M)/N</th><td>{percent(d.lower)} to {percent(d.upper)}</td><td>{percent(b.lower)} to {percent(b.upper)}</td></tr>
    <tr><th scope="row">Late starts / technical problems</th><td>{d.late} / {d.technical}</td><td>{b.late} / {b.technical}</td></tr>
    <tr><th scope="row">Overrides or deviations</th><td>{d.deviations}</td><td>{b.deviations}</td></tr>
    <tr><th scope="row">Cue delivery recorded / unrecorded</th><td>{d.delivered} / {d.deliveryUnrecorded}</td><td>{b.delivered} / {b.deliveryUnrecorded}</td></tr>
    <tr><th scope="row">Presentation noticed (self-reported)</th><td>{d.seen}</td><td>{b.seen}</td></tr>
    <tr><th scope="row">Median exact cue-to-start latency (seconds)</th><td>{d.medianExactLatencySeconds ?? "Unknown"} ({d.exactLatencyCount} exact reports)</td><td>{b.medianExactLatencySeconds ?? "Unknown"} ({b.exactLatencyCount} exact reports)</td></tr>
    <tr><th scope="row">Priority advanced / reported</th><td>{d.priorityAdvanced} / {d.priorityReported}</td><td>{b.priorityAdvanced} / {b.priorityReported}</td></tr>
    <tr><th scope="row">Domain quality notes / linked accomplishments</th><td>{d.qualityNotes} / {d.linkedAcceptanceIds.length}</td><td>{b.qualityNotes} / {b.linkedAcceptanceIds.length}</td></tr>
    <tr><th scope="row">Estimated administration / useful work (minutes)</th><td>{d.managementMinutes} / {d.usefulWorkMinutes}</td><td>{b.managementMinutes} / {b.usefulWorkMinutes}</td></tr>
  </tbody></table></div><p>{bounds ? `D + construction minus D possible difference: ${(bounds.lower * 100).toFixed(1)} to ${(bounds.upper * 100).toFixed(1)} percentage points.` : "Difference bounds require mature observations and at least one assigned opportunity in each arm."}</p></>;
}
export default function StudyReport({ study }: { study: StudyView }) {
  const [expanded, setExpanded] = useState(false);
  return <details className="study-reports" onToggle={(event) => { if (event.target === event.currentTarget) setExpanded(event.currentTarget.open); }}><summary>Descriptive reports</summary>{expanded && <>
    <p>Missing observations remain unknown. Nonstarts are censored at the declared window; missing latency is never substituted with 900 seconds. No universal productivity score or effectiveness claim is calculated.</p>
    {study.manifests.length === 0 && <p>No study configured. Ordinary opportunities remain available in history.</p>}
    {study.manifests.map((manifest) => {
      const opportunities = study.opportunities.filter((item) => item.manifest_id === manifest.id);
      const scope = reviewScope(study, manifest);
      return <section className="study-report" key={manifest.id}><h3>{manifest.phase.toUpperCase()} · {manifest.status}</h3>
        <p className="muted">Protocol {manifest.protocol_version} · {manifest.window_seconds / 60}-minute window · review checkpoints {manifest.review_after.join(" and ")} assigned opportunities.</p>
        {manifest.phase === "e00" ? <><Summary value={summarize(study, opportunities)} />
          <p>Provisional feasibility criteria: at least 90% usable eligible outcomes, median routine capture at most 2 minutes, meaningful work distinguishable from app interaction, and ordinary work/rest preserved. Actual ordinary use and owner assessment are required; this summary does not declare success.</p>
        </> : !scope.allowed ? <p>Treatment comparisons stay hidden until a planned review: {manifest.review_after.join(" or ")} matured assignments, or phase completion. {scope.totalAssigned} eligible assignments are retained so far.</p> : <>
          <p>Review includes the first {scope.count} eligible assignments of {scope.totalAssigned} currently recorded, including missing outcomes and delivery failures. Later assignments wait for the next frozen checkpoint.</p>
          {study.epochs.filter((epoch) => opportunities.some((item) => item.epoch_id === epoch.id)).map((epoch) => <div key={epoch.id}><h4>Schedule epoch: {epoch.label}</h4>
            <ArmComparison study={study} manifest={manifest} epochId={epoch.id} />
            <details><summary>Deadline and early/later block descriptions</summary>
              <h4>No externally fixed deadline within 48 hours</h4><ArmComparison study={study} manifest={manifest} epochId={epoch.id} filter={lowPressure} />
              <h4>External deadline within 48 hours or already due</h4><ArmComparison study={study} manifest={manifest} epochId={epoch.id} filter={(item) => !lowPressure(item)} />
              <h4>Earlier blocks in this review</h4><ArmComparison study={study} manifest={manifest} epochId={epoch.id} filter={(_item, position) => Math.floor(position / 4) < Math.ceil(scope.count / 8)} />
              <h4>Later blocks in this review</h4><ArmComparison study={study} manifest={manifest} epochId={epoch.id} filter={(_item, position) => Math.floor(position / 4) >= Math.ceil(scope.count / 8)} />
            </details>
          </div>)}
          <p className="muted">Epochs are kept separate. Exact latency summaries exclude unknown and approximate times and are secondary descriptions. Unrecorded delivery is not proof that a cue was seen. Small personal comparisons do not establish population-level causal benefit.</p>
        </>}
      </section>;
    })}
  </>}</details>;
}
