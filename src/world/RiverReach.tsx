import { useEffect, useMemo, useRef, useState, type ReactNode } from "react";
import * as api from "../platform/client";
import type { Workspace } from "../platform/client";
import { BuildingIcon } from "./BuildingArt";
import { ProvinceMap } from "./ProvinceMap";
import type { BuildingType, Capabilities, PlacementInput, Site, WorldCommandInput, WorldOperation, WorldPreview, WorldView } from "./types";
import "./RiverReach.css";

const CAPABILITIES = ["engineering", "knowledge", "diplomacy"] as const;
const title = (value: string) => value[0].toUpperCase() + value.slice(1);
const footprint = (placements: PlacementInput[]) => placements.reduce((total, item) => total + item.parcels.length, 0);
function CapabilityList({ values, label }: { values: Capabilities; label?: string }) {
  return <dl className="rr-capabilities" aria-label={label ?? "Fictional capabilities"}>{CAPABILITIES.map(key => <div key={key} className={`rr-capability-${key}`}><dt>{title(key)}</dt><dd>{values[key]}</dd></div>)}</dl>;
}
function Confirmation({ heading, children, pending, error, onCancel, onConfirm, label }: { heading: string; children: ReactNode; pending: boolean; error: string; onCancel: () => void; onConfirm: () => void; label: string }) {
  const ref = useRef<HTMLDivElement>(null);
  useEffect(() => { const previous = document.activeElement as HTMLElement | null; ref.current?.querySelector<HTMLButtonElement>("button")?.focus(); return () => { if (previous?.isConnected) previous.focus(); }; }, []);
  return <div className="modal-backdrop"><div className="modal rr-confirm" ref={ref} role="dialog" aria-modal="true" aria-label={heading} onKeyDown={event => {
    if (event.key === "Escape" && !pending) { event.preventDefault(); onCancel(); }
    if (event.key === "Tab") { const controls = Array.from(ref.current?.querySelectorAll<HTMLButtonElement>("button:not(:disabled)") ?? []); const first = controls[0], last = controls[controls.length - 1]; if (!first) event.preventDefault(); else if (event.shiftKey && document.activeElement === first) { event.preventDefault(); last.focus(); } else if (!event.shiftKey && document.activeElement === last) { event.preventDefault(); first.focus(); } }
  }}><h2>{heading}</h2>{children}{error && <p className="field-error" role="alert">{error}</p>}<div className="button-row"><button type="button" className="secondary" disabled={pending} onClick={onCancel}>Keep planning</button><button type="button" disabled={pending} onClick={onConfirm}>{pending ? "Saving…" : label}</button></div></div></div>;
}

interface Props { workspace: Workspace; active: boolean; chronicle: boolean; busy: boolean; onWorkspace: (workspace: Workspace) => void; onDirtyChange: (dirty: boolean) => void; onPendingChange: (pending: boolean) => void; onModalChange: (open: boolean) => void; onReturnToWork: () => void; onChronicle: () => void }
export default function RiverReach({ workspace, active, chronicle, busy, onWorkspace, onDirtyChange, onPendingChange, onModalChange, onReturnToWork, onChronicle }: Props) {
  const [world, setWorld] = useState<WorldView | null>(null);
  const [error, setError] = useState("");
  const [notice, setNotice] = useState("");
  const [pending, setPending] = useState(false);
  const lock = useRef(false);
  const readSequence = useRef(0);
  const [selectedType, setSelectedType] = useState<BuildingType>("survey_lodge");
  const [selectedSites, setSelectedSites] = useState<Site[]>([]);
  const [reserveAvailable, setReserveAvailable] = useState(true);
  const [draft, setDraft] = useState<PlacementInput[] | null>(null);
  const [editingIndex, setEditingIndex] = useState<number | null>(null);
  const [preview, setPreview] = useState<WorldPreview | null>(null);
  const [previewError, setPreviewError] = useState("");
  const [previewPending, setPreviewPending] = useState(false);
  const [confirmation, setConfirmation] = useState<"construction" | "ending" | null>(null);
  const [failedIntent, setFailedIntent] = useState<WorldCommandInput | null>(null);
  const chapter = world?.chapter;
  const definition = world?.buildings.find(item => item.building_type === selectedType);
  const names = Object.fromEntries(world?.buildings.map(item => [item.building_type, item.name]) ?? []);
  const disabled = pending || busy;
  const candidateReady = selectedSites.length === definition?.footprint;
  const draftDirty = draft !== null && JSON.stringify(draft) !== JSON.stringify(chapter?.layout ?? []);
  const currentLayout = chapter?.layout ?? [];
  const previewLayout = useMemo(() => draft ?? (candidateReady ? [...currentLayout, { id: null, building_type: selectedType, parcels: selectedSites }] : currentLayout), [draft, currentLayout, candidateReady, selectedType, selectedSites]);
  const previewKey = JSON.stringify(previewLayout);
  const queuedDefinition = world?.buildings.find(item => item.building_type === chapter?.queue?.building_type);
  const queueCost = chapter?.queue ? Math.max(0, footprint(currentLayout) + (queuedDefinition?.footprint ?? 0) - chapter.funded_site_count) * 4 : 0;
  const reusableQueueSites = chapter?.queue ? Math.min(queuedDefinition?.footprint ?? 0, Math.max(0, chapter.funded_site_count - footprint(currentLayout))) : 0;
  const queueFundingShortfall = chapter?.queue ? Math.max(0, queueCost - chapter.queue.reserved_bc - (world?.bank.available_bc ?? 0)) : 0;
  const selectedRoute = world?.routes.find(route => route.route === chapter?.selected_route);
  const achieved = !!selectedRoute && CAPABILITIES.every(key => (world?.capabilities[key] ?? 0) >= selectedRoute.requirements[key]);
  useEffect(() => { onDirtyChange(draftDirty); }, [draftDirty, onDirtyChange]);
  useEffect(() => { onPendingChange(pending); }, [pending, onPendingChange]);
  useEffect(() => { onModalChange(confirmation !== null); }, [confirmation, onModalChange]);
  useEffect(() => () => { onPendingChange(false); onDirtyChange(false); onModalChange(false); }, [onPendingChange, onDirtyChange, onModalChange]);
  async function handleFailure(cause: unknown) {
    const failure = api.normalizeError(cause);
    setError(failure.message);
    if (failure.code === "study_active") {
      ++readSequence.current; setWorld(null); setPreview(null); setConfirmation(null); setFailedIntent(null);
      try { onWorkspace(await api.loadWorkspace()); } catch { /* Keep all strategic content withheld even if the Work refresh fails. */ }
    }
  }
  useEffect(() => {
    if (!active) return;
    let cancelled = false;
    const refreshVisible = () => {
      if (document.visibilityState === "hidden") return;
      const sequence = ++readSequence.current;
      void api.loadWorld().then(value => { if (!cancelled && sequence === readSequence.current) setWorld(value); }, cause => { if (!cancelled && sequence === readSequence.current) void handleFailure(cause); });
    };
    refreshVisible();
    window.addEventListener("focus", refreshVisible); document.addEventListener("visibilitychange", refreshVisible);
    return () => { cancelled = true; window.removeEventListener("focus", refreshVisible); document.removeEventListener("visibilitychange", refreshVisible); };
  }, [active, workspace.generation]);
  useEffect(() => {
    if (!chapter || !active) return;
    let cancelled = false;
    setPreviewPending(true); setPreview(null); setPreviewError("");
    void api.previewWorld({ chapter_id: chapter.id, content_version: "river-reach-v1", placements: JSON.parse(previewKey) as PlacementInput[] }).then(value => { if (!cancelled) setPreview(value); }, cause => { if (!cancelled) { setPreviewError(api.normalizeError(cause).message); void handleFailure(cause); } }).finally(() => { if (!cancelled) setPreviewPending(false); });
    return () => { cancelled = true; };
  }, [chapter?.id, chapter?.revision, active, previewKey]);

  async function refresh() {
    setError("");
    const sequence = ++readSequence.current;
    try { const value = await api.loadWorld(); if (sequence === readSequence.current) { setWorld(value); setNotice("Saved province reloaded. Your layout draft is still available."); } } catch (cause) { if (sequence === readSequence.current) await handleFailure(cause); }
  }
  async function run(operation: WorldOperation, exact?: WorldCommandInput) {
    if (!world || lock.current || busy) return false;
    const input: WorldCommandInput = exact ?? (failedIntent && JSON.stringify(failedIntent.operation) === JSON.stringify(operation) ? failedIntent : {
      command_id: crypto.randomUUID(), schema_version: 1, content_version: "river-reach-v1", chapter_id: chapter?.id ?? null, expected_world_revision: chapter?.revision ?? 0, operation,
    });
    const sequence = ++readSequence.current;
    lock.current = true; setPending(true); setError(""); setNotice("");
    try {
      const response = await api.worldCommand(input);
      onWorkspace(response.workspace); setFailedIntent(null);
      const refreshed = await api.loadWorld();
      const acceptedRead = sequence === readSequence.current;
      if (acceptedRead) setWorld(refreshed);
      if (["reconfigure", "undo", "complete_construction", "complete_chapter"].includes(operation.kind)) { setDraft(null); setEditingIndex(null); setSelectedSites([]); }
      if (operation.kind === "queue" || operation.kind === "replace_queue") setSelectedSites([]);
      setConfirmation(null);
      if (acceptedRead) setNotice(operation.kind === "complete_construction" ? "Construction is complete. Funding and province are saved." : operation.kind === "reconfigure" ? "Layout saved. Your invested credits and funded capacity are unchanged." : operation.kind === "complete_chapter" ? "River Reach's first ending is preserved in the Chronicle." : operation.kind === "cancel_queue" ? "Blueprint cancelled. Reserved credits are available again." : operation.kind === "reserve" ? "Available funding reserved for this blueprint." : "Province saved on this device.");
      return true;
    } catch (cause) { const failure = api.normalizeError(cause); setFailedIntent(failure.code === "conflict" || failure.code === "study_active" ? null : input); await handleFailure(cause); return false; }
    finally { lock.current = false; setPending(false); }
  }
  function chooseBuilding(kind: BuildingType) { setSelectedType(kind); setSelectedSites([]); setEditingIndex(null); }
  function chooseParcel(site: string) {
    const value = site as Site;
    setSelectedSites(previous => previous.includes(value) ? previous.filter(item => item !== value) : previous.length >= (definition?.footprint ?? 1) ? [value] : [...previous, value]);
  }
  function addDraft() {
    if (!draft || !candidateReady) return;
    const old = editingIndex === null ? null : draft[editingIndex];
    const placement: PlacementInput = { id: old?.building_type === selectedType ? old.id : null, building_type: selectedType, parcels: [...selectedSites] };
    setDraft(editingIndex === null ? [...draft, placement] : draft.map((item, index) => index === editingIndex ? placement : item));
    setEditingIndex(null); setSelectedSites([]);
  }
  function startLayout() { setDraft(currentLayout.map(item => ({ ...item, parcels: [...item.parcels] }))); setSelectedSites([]); setEditingIndex(null); }

  // Retain local drafts while Work is visible, without rebuilding the scene on
  // ordinary Work text entry. All hooks above remain in the same order.
  if (!active) return null;
  const errorPanel = error && <div className="error-banner"><p role="alert">{error}</p><div className="button-row">{failedIntent && <button type="button" className="secondary" disabled={disabled} onClick={() => void run(failedIntent.operation, failedIntent)}>Retry exact request</button>}<button type="button" className="secondary" disabled={disabled} onClick={() => void refresh()}>Reload province</button></div></div>;
  if (!world) return <section className="river-reach" aria-label="River Reach"><h2>River Reach</h2>{errorPanel || <p role="status">Reading your saved province…</p>}<button type="button" className="secondary" onClick={onReturnToWork}>Return to Work</button></section>;
  const ending = chapter?.completion;
  if (chronicle) return <section className="river-reach rr-chronicle" aria-labelledby="chronicle-heading"><p className="eyebrow">The Beacon Commonwealth</p><h2 id="chronicle-heading">Chronicle</h2>{errorPanel}{ending ? <>
    <p className="rr-chapter-marker">Chapter I · River Reach · Original ending</p><h3>{world.routes.find(route => route.route === ending.route)?.name}</h3><p className="rr-ending-narrative">{ending.narrative}</p>
    <ProvinceMap placements={ending.layout} fundedSites={ending.funded_site_count} names={names} completed endingRoute={ending.route} />
    <CapabilityList values={ending.capabilities} /><p>{ending.spent_bc} BC invested · {ending.funded_site_count} funded sites · completed {new Date(ending.completed_at).toLocaleDateString()}.</p>
    <p>{chapter.sandbox_alternatives > 0 ? `${chapter.sandbox_alternatives} later sandbox layout changes. The original ending above is preserved.` : "This is the first committed ending. Later sandbox layouts leave this record intact."}</p>
    <details><summary>Chapter record</summary><p className="preserve-text">Completion {ending.id}<br />Content {ending.content_version}<br />Chapter {ending.chapter_id}</p></details>
  </> : <div className="rr-empty"><p className="rr-chapter-marker">Chapter I awaits</p><p>River Reach's first committed route will be preserved here, together with its exact province layout.</p></div>}<button type="button" className="secondary" onClick={onReturnToWork}>Return to Work</button></section>;

  return <section className="river-reach" aria-labelledby="river-reach-heading" data-world-revision={chapter?.revision ?? 0}>
    <header className="rr-heading"><div><p className="eyebrow">The Beacon Commonwealth · Chapter I</p><h2 id="river-reach-heading">River Reach</h2><p>A crossing. Six parcels. A province of your choosing.</p></div><button type="button" className="secondary" onClick={onReturnToWork}>Return to Work</button></header>
    {notice && <p className="notice" role="status">{notice}</p>}{!confirmation && errorPanel}
    {!chapter ? <>
      <ProvinceMap placements={[]} fundedSites={0} names={names} />
      <div className="rr-introduction"><h3>A small camp beside an unlit beacon</h3><p>Build on the connected river terrace, combine fictional capabilities, and choose how River Reach will connect its settlements. Construction uses credits already issued through accepted accomplishments. Nothing decays while you are away.</p><button type="button" disabled={disabled} onClick={() => void run({ kind: "initialize" })}>Enter River Reach</button><p className="muted">Entering is free. The camp and beacon are scenery.</p></div>
    </> : <>
      {ending && <div className="rr-completion"><p className="rr-chapter-marker">River Reach is complete</p><h3>{world.routes.find(route => route.route === ending.route)?.name}</h3><p>{ending.narrative}</p><div className="button-row"><button type="button" onClick={onChronicle}>View original Chronicle ending</button><button type="button" className="secondary" disabled={disabled || !!draft} onClick={startLayout}>Explore sandbox layouts</button></div><p className="muted">Alternate layouts use existing capacity. Your original ending and invested credits remain intact.</p></div>}
      <dl className="rr-bank" aria-label="Build Credit accounting">{[["Issued", world.bank.issued_bc], ["Available", world.bank.available_bc], ["Reserved", world.bank.reserved_bc], ["Invested", world.bank.spent_bc]].map(([name, value]) => <div key={name}><dt>{name} BC</dt><dd>{value}</dd></div>)}</dl>
      <div className="rr-capacity"><span><strong>{chapter.funded_site_count} / 6</strong> funded sites</span><span><strong>{footprint(currentLayout)}</strong> occupied</span><span><strong>{chapter.funded_site_count - footprint(currentLayout)}</strong> funded sites free</span><span><strong>{6 - chapter.funded_site_count}</strong> unfunded</span><span>Chapter investment: <strong>{world.bank.spent_bc} / 24 BC</strong></span></div>
      <p className="rr-small">Funded sites are reusable capacity. A layout may use any exact parcel within that capacity; moving or replacing buildings is free.</p>
      {world.retained_credit_bc > 0 && <p className="rr-retained">Construction includes {world.retained_credit_bc} BC of fictional courtesy credit retained after factual corrections. Existing construction stays in place.</p>}
      <div className="rr-province-layout"><div className="rr-province">
        <div className="rr-map-heading"><h3>{draft ? ending ? "Sandbox layout preview" : "Layout preview" : ending ? chapter.sandbox_alternatives > 0 ? "Saved sandbox province" : "Completed province" : "Your province"}</h3>{draft && <span className="draft-label">Unsaved layout</span>}</div>
        {ending && chapter.sandbox_alternatives > 0 && <p className="rr-small">This saved sandbox layout is separate from the original {world.routes.find(route => route.route === ending.route)?.name} ending in the Chronicle.</p>}
        <ProvinceMap placements={draft ?? currentLayout} selected={selectedSites} queued={chapter.queue?.parcels} fundedSites={chapter.funded_site_count} names={names} draft={draft !== null} completed={!!ending} endingRoute={ending?.route} onSelect={disabled ? undefined : chooseParcel} />
        <CapabilityList values={draft && preview ? preview.capabilities : world.capabilities} label={draft ? "Preview capabilities" : "Current capabilities"} />
        <p className="rr-small">Capabilities belong to this fictional layout. They are reusable properties, never currencies or measures of your abilities.</p>
      </div><aside className="rr-planning" aria-label="Province planning">
        <div className="rr-mode-buttons"><button type="button" className={draft ? "secondary" : ""} disabled={disabled || !!ending} onClick={() => { setDraft(null); setSelectedSites([]); setEditingIndex(null); }}>Construction</button><button type="button" className={draft ? "" : "secondary"} disabled={disabled || !!draft || !!chapter.queue} onClick={startLayout}>{ending ? "Sandbox" : "Reconfigure"}</button></div>
        <h3>{draft ? "Arrange funded capacity" : "Choose your next building"}</h3>
        {chapter.queue && !draft && <p className="rr-small">One blueprint is queued. Replacing it releases its reservation before reserving the new blueprint. Cancel it before reconfiguring.</p>}
        <label htmlFor="rr-building">Building</label><select id="rr-building" value={selectedType} disabled={disabled} onChange={event => chooseBuilding(event.target.value as BuildingType)}>{world.buildings.map(item => <option key={item.building_type} value={item.building_type}>{item.name}</option>)}</select>
        {definition && <div className="rr-selected-building"><BuildingIcon kind={selectedType} /><div><strong>{definition.name}</strong><p>{definition.footprint} {definition.footprint === 1 ? "parcel" : "adjacent parcels"} · {definition.cost_bc} BC capacity</p></div><p>{definition.description}</p><p className="rr-small">{CAPABILITIES.filter(key => definition.capabilities[key] > 0).map(key => `${title(key)} +${definition.capabilities[key]}`).join(" · ")} · max {definition.copy_limit} copies</p></div>}
        <p id="rr-parcel-help">Select {definition?.footprint === 2 ? "two orthogonally adjacent parcels" : "one parcel"} on the map or below. Select again to clear.</p>
        <div className="rr-parcel-choices" aria-describedby="rr-parcel-help">{world.parcels.map(parcel => <button key={parcel.id} type="button" className="secondary" disabled={disabled} aria-pressed={selectedSites.includes(parcel.id)} onClick={() => chooseParcel(parcel.id)}>{parcel.label}</button>)}</div>
        <p className="rr-selection">Selected: {selectedSites.map(site => site.toUpperCase()).join(" + ") || "none"}</p>
        {draft ? <><button type="button" disabled={disabled || !candidateReady || draft.length >= 6 && editingIndex === null} onClick={addDraft}>{editingIndex === null ? "Add to layout preview" : "Update draft placement"}</button><p className="rr-small">Changes stay in the preview until you save the entire legal layout.</p></>
          : !ending && <><label className="check-label"><input type="checkbox" checked={reserveAvailable} disabled={disabled} onChange={event => setReserveAvailable(event.target.checked)} />Reserve available credits when queued</label><button type="button" disabled={disabled || !candidateReady || previewPending || !preview?.placement_legal} onClick={() => void run({ kind: chapter.queue ? "replace_queue" : "queue", building_type: selectedType, parcels: selectedSites, reserve_available: reserveAvailable })}>{chapter.queue ? "Replace queued blueprint" : "Queue construction"}</button></>}
      </aside></div>
      {(draft || candidateReady) && <section className="rr-preview" aria-labelledby="rr-preview-heading"><h3 id="rr-preview-heading">{draft ? "Authoritative layout preview" : "Placement preview"}</h3>{previewPending && <p role="status">Checking the proposed layout…</p>}{previewError && <p role="alert" className="field-error">{previewError}</p>}{preview && <><p className={preview.placement_legal ? "rr-valid" : "field-error"}>{preview.placement_legal ? "The spatial layout is legal." : "This layout cannot be committed."}</p>{preview.errors.length > 0 && <ul>{preview.errors.map((item, index) => <li key={index}>{item}</li>)}</ul>}<p>{preview.occupied_sites} occupied sites · {preview.funded_site_count} funded · {preview.additional_sites_needed} additional sites needed{preview.additional_sites_needed > 0 ? ` (${preview.additional_sites_needed * 4} BC through construction)` : ""}.</p><CapabilityList values={preview.capabilities} label="Projected capabilities" />{draft && preview.additional_sites_needed > 0 && <p className="field-error">This preview exceeds funded capacity. Save a smaller layout and use construction for additional funding.</p>}</>}
        {draft && <><ol className="rr-draft-list">{draft.map((item, index) => <li key={item.id ?? `draft-${index}`}><span><strong>{names[item.building_type]}</strong> · {item.parcels.map(site => site.toUpperCase()).join(" + ")}</span><div className="button-row"><button type="button" className="secondary small" disabled={disabled} onClick={() => { setSelectedType(item.building_type); setSelectedSites(item.parcels); setEditingIndex(index); }}>Move or turn {names[item.building_type]} {index + 1}</button><button type="button" className="secondary small" disabled={disabled} onClick={() => { setDraft(draft.filter((_, i) => i !== index)); setEditingIndex(null); setSelectedSites([]); }}>Remove {names[item.building_type]} {index + 1}</button></div></li>)}</ol><div className="button-row"><button type="button" disabled={disabled || previewPending || !preview?.legal || preview.additional_sites_needed > 0 || !draftDirty} onClick={() => void run({ kind: "reconfigure", placements: draft })}>Save free layout</button><button type="button" className="secondary" disabled={disabled} onClick={() => { setDraft(null); setEditingIndex(null); setSelectedSites([]); }}>Discard layout preview</button></div></>}
      </section>}
      {chapter.queue && <section className="rr-queue" aria-labelledby="rr-queue-heading"><div><p className="eyebrow">Construction queue · one blueprint</p><h3 id="rr-queue-heading">{queuedDefinition?.name}</h3><p>Parcels {chapter.queue.parcels.map(site => site.toUpperCase()).join(" + ")} · {chapter.queue.reserved_bc} / {queueCost} BC reserved</p>
        <dl className="rr-blueprint-details"><div><dt>Required footprint</dt><dd>{queuedDefinition?.footprint} {queuedDefinition?.footprint === 1 ? "parcel" : "orthogonally adjacent parcels"}</dd></div><div><dt>Total building capacity</dt><dd>{queuedDefinition?.cost_bc} BC</dd></div><div><dt>Previously funded capacity usable</dt><dd>{reusableQueueSites} {reusableQueueSites === 1 ? "site" : "sites"}</dd></div><div><dt>Additional funded capacity needed</dt><dd>{queueCost / 4} {queueCost === 4 ? "site" : "sites"} · {queueCost} BC</dd></div><div><dt>Placement status</dt><dd>Legal saved blueprint</dd></div><div><dt>Current affordability</dt><dd>{queueFundingShortfall > 0 ? `${queueFundingShortfall} more issued BC needed` : chapter.queue.reserved_bc >= queueCost ? "Fully funded — ready to complete" : "Enough available BC — reserve to complete"}</dd></div></dl>
        <progress value={chapter.queue.reserved_bc} max={Math.max(1, queueCost)} aria-label="Reserved construction funding" /><p>{Math.max(0, queueCost - chapter.queue.reserved_bc)} BC still to reserve. Reserving funding creates no capability until construction is completed.</p></div><div className="button-row"><button type="button" className="secondary" disabled={disabled || world.bank.available_bc === 0 || chapter.queue.reserved_bc >= queueCost} onClick={() => void run({ kind: "reserve" })}>Reserve available BC</button><button type="button" disabled={disabled || chapter.queue.reserved_bc < queueCost} onClick={() => { setError(""); setConfirmation("construction"); }}>Complete construction</button><button type="button" className="secondary" disabled={disabled} onClick={() => void run({ kind: "cancel_queue" })}>Cancel blueprint</button></div><p className="rr-small">Choose another building and parcels, then use Replace queued blueprint to release and reassign this reservation together. Cancel blueprint releases the unspent reservation without a replacement.</p></section>}
      <section className="rr-routes" aria-labelledby="rr-routes-heading"><div className="section-heading"><div><p className="eyebrow">Three ways to connect the province</p><h3 id="rr-routes-heading">Choose a development route</h3></div><span className="rr-small">All endings have equal standing</span></div><div className="rr-route-grid">{world.routes.map(route => { const totals = draft && preview ? preview.capabilities : world.capabilities; const satisfied = draft && preview ? preview.placement_legal && (preview.routes.find(item => item.route === route.route)?.satisfied ?? false) : CAPABILITIES.every(key => totals[key] >= route.requirements[key]); return <article className={`rr-route ${chapter.selected_route === route.route ? "rr-route-selected" : ""}`} key={route.route}><span className="rr-route-symbol" aria-hidden="true">{route.route === "stone_causeway" ? "⌒" : route.route === "river_compact" ? "≈" : "✧"}</span><h4>{route.name}</h4><p>{route.description}</p><ul>{CAPABILITIES.filter(key => route.requirements[key] > 0).map(key => <li key={key}><span>{title(key)}</span><strong>{totals[key]} / {route.requirements[key]}</strong><small>{Math.max(0, route.requirements[key] - totals[key])} missing</small></li>)}</ul><p className={satisfied ? "rr-valid" : "rr-small"}>{satisfied ? draft ? "Satisfied in preview" : "Requirements satisfied" : "Build or reconfigure toward this route"}</p>{!ending && <button type="button" className="secondary" disabled={disabled || chapter.selected_route === route.route} onClick={() => void run({ kind: "select_route", route: route.route })}>{chapter.selected_route === route.route ? "Selected route" : `Choose ${route.name}`}</button>}</article>; })}</div>
        {!ending && <div className="rr-route-action"><p>{selectedRoute ? `Selected: ${selectedRoute.name}. The ending uses your saved province.` : "Choose any route. You can change your choice before committing the ending."}</p><button type="button" disabled={disabled || !achieved || draft !== null || !!chapter.queue} onClick={() => { setError(""); setConfirmation("ending"); }}>Review chapter ending</button></div>}
      </section>
      <section className="rr-building-catalog" aria-labelledby="rr-buildings-heading"><h3 id="rr-buildings-heading">Buildings of River Reach</h3><div className="rr-building-grid">{world.buildings.map(building => <article key={building.building_type}><BuildingIcon kind={building.building_type} /><h4>{building.name}</h4><p>{building.description}</p><p><strong>{building.footprint} {building.footprint === 1 ? "parcel" : "adjacent parcels"}</strong> · {building.cost_bc} BC capacity</p><p>{CAPABILITIES.filter(key => building.capabilities[key] > 0).map(key => `${title(key)} +${building.capabilities[key]}`).join(" · ")}</p><button type="button" className="secondary small" disabled={disabled} onClick={() => { chooseBuilding(building.building_type); document.getElementById("rr-building")?.focus(); }}>Inspect {building.name}</button></article>)}</div></section>
      <div className="rr-footer"><button type="button" className="secondary" disabled={disabled || !chapter.previous_layout || !!chapter.queue || draft !== null} onClick={() => void run({ kind: "undo" })}>Undo last layout</button><button type="button" className="text-button" disabled={disabled} onClick={() => void refresh()}>Reload province</button><p className="rr-small">Undo restores a prior layout. It never refunds invested credits. Content {world.content_version}.</p></div>
    </>}
    {confirmation === "construction" && chapter?.queue && <Confirmation heading="Complete this construction?" label="Confirm construction and spend" pending={pending} error={error} onCancel={() => setConfirmation(null)} onConfirm={() => void run({ kind: "complete_construction" })}><p><strong>{queuedDefinition?.name}</strong> will occupy {chapter.queue.parcels.map(site => site.toUpperCase()).join(" + ")}.</p><p>This commits <strong>{queueCost} BC</strong> and adds {queueCost / 4} funded sites. The completed layout can be reconfigured freely; committed spending is never refunded.</p><p>Chapter investment after completion: {world.bank.spent_bc + queueCost} / 24 BC.</p></Confirmation>}
    {confirmation === "ending" && chapter && selectedRoute && <Confirmation heading="Preserve this chapter ending?" label="Commit chapter ending" pending={pending} error={error} onCancel={() => setConfirmation(null)} onConfirm={() => void run({ kind: "complete_chapter" })}><h3>{selectedRoute.name}</h3><p>{selectedRoute.ending}</p><CapabilityList values={world.capabilities} /><p>{chapter.funded_site_count} funded sites · {world.bank.spent_bc} BC invested.</p><ul>{chapter.layout.map(item => <li key={item.id}>{names[item.building_type]} · {item.parcels.map(site => site.toUpperCase()).join(" + ")}</li>)}</ul><p>This preserves the first ending and exact layout in the Chronicle. You may explore alternatives afterward using the same capacity. No extra credits are awarded.</p></Confirmation>}
  </section>;
}
