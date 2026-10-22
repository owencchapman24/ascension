import * as api from "../platform/client";
import { useEffect, useState } from "react";
import type { StudyMutation } from "../study/OpportunityEditor";

export function organizedObjectives(workspace: api.Workspace) {
  const organizations = new Map(workspace.daily.objectives.map((item) => [item.objective_id, item]));
  return workspace.objectives.map((objective, index) => ({ objective, organization: organizations.get(objective.id) ?? { objective_id: objective.id, status: "active" as const, priority: index, revision: 1, updated_at: objective.updated_at } }))
    .sort((a, b) => a.organization.priority - b.organization.priority || a.objective.id.localeCompare(b.objective.id));
}
export default function ObjectiveManager({ workspace, busy, onMutation, onSelect }: { workspace: api.Workspace; busy: boolean; onMutation: StudyMutation; onSelect: (id: string) => void }) {
  const [expanded, setExpanded] = useState(false);
  const [focusId, setFocusId] = useState<string | null>(null);
  const ordered = organizedObjectives(workspace);
  const active = ordered.filter((item) => item.organization.status === "active");
  useEffect(() => {
    if (!busy && focusId) { document.getElementById(`priority-${focusId}`)?.focus(); setFocusId(null); }
  }, [busy, focusId, workspace]);
  async function changeStatus(id: string, status: api.ObjectiveStatus, message: string) {
    const saved = await onMutation(() => api.setObjectiveStatus({ expected_generation: workspace.generation, objective_id: id, status }), message);
    if (saved) setFocusId(id);
  }
  async function move(id: string, direction: -1 | 1) {
    const ids = active.map((item) => item.objective.id); const index = ids.indexOf(id); const target = index + direction;
    if (target < 0 || target >= ids.length) return;
    [ids[index], ids[target]] = [ids[target], ids[index]];
    const saved = await onMutation(() => api.reorderObjectives({ expected_generation: workspace.generation, objective_ids: ids }), "Priority order saved on this device.");
    if (saved) setFocusId(id);
  }
  return <details className="objective-manager" onToggle={(event) => { if (event.target === event.currentTarget) setExpanded(event.currentTarget.open); }}><summary>Manage objectives and priority</summary>{expanded && <>
    <p className="muted">Choose your order. Pausing or archiving organizes work and preserves its factual and reward history.</p>
    {(["active", "paused", "archived"] as const).map((status) => <section key={status} aria-label={`${status} objectives`}><h3>{status === "active" ? "Active priorities" : status === "paused" ? "Paused" : "Archived"}</h3>
      <ol>{ordered.filter((item) => item.organization.status === status).map(({ objective }) => {
        const position = active.findIndex((item) => item.objective.id === objective.id);
        return <li key={objective.id} id={`priority-${objective.id}`} tabIndex={-1}>
          <p><strong>{objective.title}</strong>{workspace.selected_objective_id === objective.id && <span className="muted"> · focused</span>}</p>
          <p className="muted">{workspace.projects.find((item) => item.id === objective.project_id)?.title}</p>
          <div className="button-row">
            {status === "active" && <><button type="button" className="secondary small" disabled={busy || workspace.selected_objective_id === objective.id} onClick={() => onSelect(objective.id)}>Focus {objective.title}</button>
              <button type="button" className="secondary small" disabled={busy || position === 0} onClick={() => void move(objective.id, -1)} aria-label={`Move ${objective.title} higher`}>Move higher</button><button type="button" className="secondary small" disabled={busy || position === active.length - 1} onClick={() => void move(objective.id, 1)} aria-label={`Move ${objective.title} lower`}>Move lower</button></>}
            <button type="button" className="secondary small" disabled={busy} onClick={() => void changeStatus(objective.id, status === "active" ? "paused" : "active", status === "active"
              ? active.length === 1 ? "Objective paused. No active priority remains; reactivate an objective or create one when ready." : "Objective paused. Active priorities remain available."
              : "Objective restored to active priorities.")}>{status === "active" ? "Pause" : status === "archived" ? "Restore" : "Reactivate"} {objective.title}</button>
            {status !== "archived" && <button type="button" className="secondary small" disabled={busy} onClick={() => void changeStatus(objective.id, "archived", "Objective archived with its history preserved.")}>Archive {objective.title}</button>}
          </div>
          <details><summary>Saved context and history</summary><p>Criterion: {objective.criterion}</p><p>Saved action: {workspace.actions.find((item) => item.objective_id === objective.id)?.description ?? "No action saved."}</p><p>Resumption note: {workspace.notes.find((item) => item.objective_id === objective.id)?.text ?? "No note saved."}</p>
            {workspace.progress.acceptances.filter((item) => item.parent_id === objective.id).map((item) => <p key={item.id}>Accomplishment ({item.status}): {item.factual_summary}</p>)}
            {workspace.daily.checkpoints.filter((item) => item.objective_id === objective.id).map((item) => <p key={item.id}>Checkpoint · {new Date(item.recorded_at).toLocaleString()}: {item.note}{item.supersedes && " (correction; original retained)"}</p>)}
          </details>
        </li>;
      })}</ol>
    </section>)}
  </>}</details>;
}
