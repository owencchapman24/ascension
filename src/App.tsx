import { useEffect, useRef, useState, type FormEvent, type ReactNode } from "react";
import * as api from "./platform/client";
import type { CommandError, ImportPreview, Objective, Project, Workspace } from "./platform/client";
import "./App.css";

type Editor =
  | { kind: "project"; id: string | null; title: string }
  | { kind: "objective"; id: string | null; project_id: string; title: string; criterion: string }
  | { kind: "work"; objective_id: string; description: string; stop_condition: string; note: string };
type Guard = { action: () => void; description: string; onKeep?: () => void };
type ValidationError = { message: string; field: string };

function Modal({ title, children, onCancel }: { title: string; children: ReactNode; onCancel: () => void }) {
  const ref = useRef<HTMLDivElement>(null);
  useEffect(() => {
    const previous = document.activeElement as HTMLElement | null;
    ref.current?.querySelector<HTMLElement>("button")?.focus();
    return () => queueMicrotask(() => {
      const nextDialog = document.querySelector<HTMLElement>("[role=dialog]");
      if (nextDialog) {
        nextDialog.querySelector<HTMLElement>("button:not(:disabled)")?.focus();
      } else if (previous?.isConnected && !previous.matches(":disabled")) {
        previous.focus();
      } else {
        (document.querySelector<HTMLElement>(".editor-panel input, .editor-panel textarea")
          ?? document.getElementById("work-heading"))?.focus();
      }
    });
  }, []);
  return <div className="modal-backdrop"><div className="modal" role="dialog" aria-modal="true" aria-label={title} ref={ref} onKeyDown={(event) => {
    if (event.key === "Escape") { event.preventDefault(); onCancel(); }
    if (event.key === "Tab") {
      const controls = Array.from(ref.current?.querySelectorAll<HTMLElement>("button:not(:disabled), input, select, textarea") ?? []);
      if (!controls.length) event.preventDefault();
      const first = controls[0]; const last = controls[controls.length - 1];
      if (event.shiftKey && document.activeElement === first) { event.preventDefault(); last?.focus(); }
      if (!event.shiftKey && document.activeElement === last) { event.preventDefault(); first?.focus(); }
    }
  }}><h2>{title}</h2>{children}</div></div>;
}

function TextField({ id, label, value, limit, error, multiline = false, optional = false, onChange }: {
  id: string; label: string; value: string; limit: number; error?: string; multiline?: boolean; optional?: boolean; onChange: (value: string) => void;
}) {
  const props = {
    id, value, required: !optional,
    "aria-invalid": error ? true : undefined,
    "aria-describedby": error ? `${id}-error ${id}-help` : `${id}-help`,
    onChange: (event: React.ChangeEvent<HTMLInputElement | HTMLTextAreaElement>) => onChange(event.target.value),
  };
  return <div className="field"><label htmlFor={id}>{label}{optional && <span className="muted"> (optional)</span>}</label>
    {error && <p id={`${id}-error`} role="alert" className="field-error">{error}</p>}
    {multiline ? <textarea {...props} rows={id === "note" ? 5 : 3} /> : <input {...props} type="text" />}
    <small id={`${id}-help`} className={[...value].length > limit ? "over-limit" : "muted"}>{[...value].length.toLocaleString()} / {limit.toLocaleString()} characters</small>
  </div>;
}

function validateEditor(editor: Editor): ValidationError | null {
  const fields: Array<[string, string, string, number, boolean]> = editor.kind === "project"
    ? [["Project name", "project-title", editor.title, 120, true]]
    : editor.kind === "objective"
      ? [["Objective", "objective-title", editor.title, 500, true], ["Criterion", "criterion", editor.criterion, 2000, true]]
      : [["Next action", "description", editor.description, 2000, true], ["Stop condition", "stop-condition", editor.stop_condition, 2000, true], ["Resumption note", "note", editor.note, 8000, false]];
  for (const [label, field, value, limit, required] of fields) {
    if (required && !value.trim()) return { message: `${label} needs a nonempty value.`, field };
    if (value.includes("\0")) return { message: `${label} cannot contain a null character.`, field };
    if ([...value].length > limit) return { message: `${label} must be ${limit.toLocaleString()} characters or fewer.`, field };
  }
  return null;
}

function App() {
  const [workspace, setWorkspace] = useState<Workspace | null>(null);
  const [projectId, setProjectId] = useState("");
  const [loading, setLoading] = useState(true);
  const [busy, setBusy] = useState(false);
  const lock = useRef(false);
  const [error, setError] = useState<CommandError | null>(null);
  const [validationError, setValidationError] = useState<ValidationError | null>(null);
  const [notice, setNotice] = useState("");
  const [editor, setEditor] = useState<Editor | null>(null);
  const [baseline, setBaseline] = useState("");
  const [guard, setGuard] = useState<Guard | null>(null);
  const [preview, setPreview] = useState<ImportPreview | null>(null);
  const editorRef = useRef<HTMLElement>(null);
  const errorNoticeRef = useRef<HTMLDivElement>(null);
  const previousEditorKey = useRef("");
  const editorKey = editor ? `${editor.kind}:${"id" in editor ? editor.id : editor.objective_id}` : "";
  const dirty = editor !== null && JSON.stringify(editor) !== baseline;
  const selected = workspace?.objectives.find((item) => item.id === workspace.selected_objective_id);
  const project = workspace?.projects.find((item) => item.id === projectId);
  const action = workspace?.actions.find((item) => item.objective_id === selected?.id);
  const note = workspace?.notes.find((item) => item.objective_id === selected?.id);

  function adopt(next: Workspace, preferredProject?: string) {
    setWorkspace(next);
    setProjectId((previous) => next.objectives.find((item) => item.id === next.selected_objective_id)?.project_id
      ?? (next.projects.some((item) => item.id === preferredProject) ? preferredProject! : next.projects.some((item) => item.id === previous) ? previous : next.projects[0]?.id ?? ""));
  }

  useEffect(() => {
    let active = true;
    api.loadWorkspace().then((next) => { if (active) adopt(next); }, (cause: unknown) => { if (active) setError(api.normalizeError(cause)); })
      .finally(() => { if (active) setLoading(false); });
    return () => { active = false; };
  }, []);
  useEffect(() => {
    if (editorKey) editorRef.current?.querySelector<HTMLElement>("input, textarea")?.focus();
    else if (previousEditorKey.current) document.getElementById("work-heading")?.focus();
    previousEditorKey.current = editorKey;
  }, [editorKey]);
  useEffect(() => {
    if (validationError) document.getElementById(`${validationError.field}-error`)?.scrollIntoView?.({ block: "nearest" });
  }, [validationError]);
  useEffect(() => {
    if (error && !editor && !preview) errorNoticeRef.current?.scrollIntoView?.({ block: "nearest" });
  }, [error, editor, preview]);

  async function perform(task: () => Promise<void>) {
    if (lock.current) return;
    lock.current = true; setBusy(true); setError(null); setNotice("");
    try { await task(); } catch (cause: unknown) { setError(api.normalizeError(cause)); }
    finally { lock.current = false; setBusy(false); }
  }

  function protectedAction(description: string, task: () => void, onKeep?: () => void) {
    if (dirty) setGuard({ description, action: task, onKeep });
    else task();
  }
  function openEditor(next: Editor) {
    protectedAction("Opening another editor will discard this unsaved draft.", () => {
      setEditor(next); setBaseline(JSON.stringify(next)); setError(null); setValidationError(null); setNotice("");
    });
  }
  function updateEditor(next: Editor) {
    setEditor(next);
    setValidationError(null);
  }
  function fieldError(id: string) { return validationError?.field === id ? validationError.message : undefined; }
  function editProject(item?: Project) { openEditor({ kind: "project", id: item?.id ?? null, title: item?.title ?? "" }); }
  function editObjective(item?: Objective) { openEditor({ kind: "objective", id: item?.id ?? null, project_id: item?.project_id ?? projectId, title: item?.title ?? "", criterion: item?.criterion ?? "" }); }
  function editWork() {
    if (!selected) return;
    openEditor({ kind: "work", objective_id: selected.id, description: action?.description ?? "", stop_condition: action?.stop_condition ?? "", note: note?.text ?? "" });
  }
  function reload() {
    protectedAction("Reloading saved records will discard this unsaved draft.", () => void perform(async () => {
      const next = await api.loadWorkspace(); adopt(next); setEditor(null); setPreview(null); setNotice("Saved records reloaded.");
    }));
  }
  function changeSelection(id: string | null, preferredProject?: string) {
    if (!workspace) return;
    protectedAction("Switching objectives will discard this unsaved draft.", () => void perform(async () => {
      adopt(await api.selectObjective({ expected_generation: workspace.generation, id }), preferredProject);
      setEditor(null);
    }));
  }

  async function submit(event: FormEvent) {
    event.preventDefault();
    if (!workspace || !editor || lock.current) return;
    setValidationError(null);
    setError(null);
    const invalid = validateEditor(editor);
    if (invalid) { setValidationError(invalid); document.getElementById(invalid.field)?.focus(); return; }
    await perform(async () => {
      const expected_generation = workspace.generation;
      let next: Workspace;
      if (editor.kind === "project") {
        next = editor.id ? await api.updateProject({ expected_generation, id: editor.id, title: editor.title }) : await api.createProject({ expected_generation, title: editor.title });
      } else if (editor.kind === "objective") {
        next = editor.id ? await api.updateObjective({ expected_generation, id: editor.id, title: editor.title, criterion: editor.criterion }) : await api.createObjective({ expected_generation, project_id: editor.project_id, title: editor.title, criterion: editor.criterion });
      } else {
        next = await api.saveWork({ expected_generation, objective_id: editor.objective_id, description: editor.description, stop_condition: editor.stop_condition, note: editor.note || null });
      }
      const newProject = editor.kind === "project" && !editor.id ? next.projects.find((item) => !workspace.projects.some((old) => old.id === item.id))?.id : undefined;
      adopt(next, newProject); setEditor(null); setNotice(editor.kind === "work" ? "Next action and note saved on this device." : editor.kind === "project" ? "Project saved on this device." : "Objective and criterion saved on this device.");
    });
  }

  function restore() {
    if (!preview) return;
    protectedAction("Replacing local records will discard this unsaved draft.", () => void perform(async () => {
      const result = await api.restoreImport({ token: preview.token, expected_generation: preview.generation });
      adopt(result.workspace); setEditor(null); setPreview(null);
      setNotice(result.unchanged ? "This export already matches your saved records. No changes were needed." : result.backup_created ? "Records restored. A backup of the previous local records was created on this device." : "Records restored on this device.");
    }), () => setPreview(null));
  }

  const errorNotice = error && (
    <div className="error-banner" ref={errorNoticeRef}>
      <p role="alert">{error.message}</p>
      {error.code === "conflict" && <p>Your saved records changed. Reload to review them before trying again.</p>}
      <button type="button" className="secondary" disabled={busy || !!guard} onClick={reload}>
        {workspace ? "Reload saved records" : "Retry connection"}
      </button>
    </div>
  );

  return (
    <main className="work-shell">
      <header className="app-header">
        <div><p className="eyebrow">Your local workspace</p><h1>Ascension</h1></div>
        <span className="local-label">Stored on this device</span>
      </header>
      {loading && <p role="status">Opening your saved workspace...</p>}
      {!editor && !preview && errorNotice}
      {notice && <p role="status" className="notice">{notice}</p>}
      {busy && <p role="status" className="muted">Waiting for the desktop app...</p>}
      {workspace && (
        <fieldset className="workspace-controls" disabled={busy || !!guard || !!preview}>
          <legend className="visually-hidden">Workspace</legend>
          <section className="chooser" aria-label="Choose your work">
            <div className="chooser-field">
              <label htmlFor="project-select">Project</label>
              <select id="project-select" value={projectId} disabled={!workspace.projects.length} onChange={(event) => {
                const id = event.target.value;
                changeSelection(workspace.objectives.find((item) => item.project_id === id)?.id ?? null, id);
              }}>
                <option value="" disabled>No project yet</option>
                {workspace.projects.map((item) => <option key={item.id} value={item.id}>{item.title}</option>)}
              </select>
              <div className="button-row">
                <button type="button" className="secondary small" onClick={() => editProject()}>New project</button>
                {project && <button type="button" className="secondary small" onClick={() => editProject(project)}>Edit project</button>}
              </div>
            </div>
            <div className="chooser-field">
              <label htmlFor="objective-select">Parent objective</label>
              <select id="objective-select" value={selected?.id ?? ""}
                disabled={!workspace.objectives.some((item) => item.project_id === projectId)}
                onChange={(event) => changeSelection(event.target.value || null)}>
                <option value="">Choose an objective</option>
                {workspace.objectives.filter((item) => item.project_id === projectId).map((item) => (
                  <option key={item.id} value={item.id}>{item.title}</option>
                ))}
              </select>
              {project && (
                <div className="button-row">
                  <button type="button" className="secondary small" onClick={() => editObjective()}>New objective</button>
                  {selected && <button type="button" className="secondary small" onClick={() => editObjective(selected)}>Edit objective</button>}
                </div>
              )}
            </div>
          </section>
          <section className="work-panel" aria-labelledby="work-heading">
            <div className="section-heading">
              <h2 id="work-heading" tabIndex={-1}>Work</h2>
              {selected && (
                <button type="button" className="secondary" onClick={editWork}>
                  {action ? "Edit next action and note" : "Add next action"}
                </button>
              )}
            </div>
            {!selected ? (
              <div className="empty-state">
                <h3>Start with a clear next step</h3>
                <p>{workspace.projects.length
                  ? "Choose or create a bounded objective, then save one concrete next action."
                  : "Create a project, then add a bounded objective and a concrete next action."}</p>
                {!workspace.projects.length && (
                  <button type="button" onClick={() => editProject()}>Create your first project</button>
                )}
              </div>
            ) : (
              <>
                {action ? (
                  <div className="saved-action">
                    <p className="eyebrow">Saved next action</p>
                    <h3 className="preserve-text">{action.description}</h3>
                    <p className="detail-label">Stop condition</p>
                    <p className="preserve-text">{action.stop_condition}</p>
                  </div>
                ) : (
                  <p className="empty-action">No next action saved yet. Describe one concrete action and where you will stop.</p>
                )}
                <div className="saved-note">
                  <h3>Resumption note</h3>
                  <p className={note ? "preserve-text" : "muted"}>{note?.text ?? "No resumption note saved."}</p>
                </div>
                <details className="objective-context">
                  <summary>Objective and criterion</summary>
                  <h3 className="preserve-text">{selected.title}</h3>
                  <p className="detail-label">Criterion</p>
                  <p className="preserve-text">{selected.criterion}</p>
                </details>
              </>
            )}
          </section>
          {editor && (
            <section className="editor-panel" aria-labelledby="editor-heading" ref={editorRef}>
              <div className="section-heading">
                <h2 id="editor-heading">{editor.kind === "work" ? "Edit next action and note" : `${editor.id ? "Edit" : "New"} ${editor.kind}`}</h2>
                <span className="draft-label">Unsaved draft</span>
              </div>
              {editor.kind === "objective" && <p className="muted">Project: {workspace.projects.find((item) => item.id === editor.project_id)?.title}</p>}
              {editor.kind === "work" && <p className="muted">Objective: {selected?.title}</p>}
              <form onSubmit={(event) => void submit(event)} noValidate>
                {editor.kind === "project" && (
                  <TextField id="project-title" label="Project name" value={editor.title} limit={120}
                    error={fieldError("project-title")} onChange={(title) => updateEditor({ ...editor, title })} />
                )}
                {editor.kind === "objective" && (
                  <>
                    <TextField id="objective-title" label="Bounded objective" value={editor.title} limit={500}
                      error={fieldError("objective-title")} onChange={(title) => updateEditor({ ...editor, title })} />
                    <TextField id="criterion" label="Criterion: how will you recognize the outcome?"
                      value={editor.criterion} limit={2000} multiline
                      error={fieldError("criterion")} onChange={(criterion) => updateEditor({ ...editor, criterion })} />
                  </>
                )}
                {editor.kind === "work" && (
                  <>
                    <TextField id="description" label="Next action: what will you do?"
                      value={editor.description} limit={2000} multiline
                      error={fieldError("description")} onChange={(description) => updateEditor({ ...editor, description })} />
                    <TextField id="stop-condition" label="Stop condition: where will you stop?"
                      value={editor.stop_condition} limit={2000} multiline
                      error={fieldError("stop-condition")} onChange={(stop_condition) => updateEditor({ ...editor, stop_condition })} />
                    <TextField id="note" label="Resumption note" value={editor.note} limit={8000} multiline optional
                      error={fieldError("note")} onChange={(value) => updateEditor({ ...editor, note: value })} />
                    <p className="muted">Leave the note blank to clear it when you save.</p>
                  </>
                )}
                {!preview && errorNotice}
                <div className="button-row">
                  <button type="submit">{busy ? "Saving..." : editor.kind === "work" ? "Save next action and note" : `Save ${editor.kind}`}</button>
                  <button type="button" className="secondary" onClick={() => protectedAction(
                    "Closing the editor will discard this unsaved draft.",
                    () => { setEditor(null); setError(null); },
                  )}>Cancel editing</button>
                </div>
              </form>
            </section>
          )}
          <footer className="records-panel">
            <div>
              <h2>Records</h2>
              <p className="muted">Export saved records or preview a file to restore. Unsaved drafts are not exported.</p>
            </div>
            <div className="button-row">
              <button type="button" className="secondary" onClick={() => void perform(async () => {
                const result = await api.exportWorkspace();
                if (result) setNotice(`Saved records exported to ${result.file_name}.`);
              })}>Export records</button>
              <button type="button" className="secondary" onClick={() => void perform(async () => {
                const result = await api.previewImport();
                if (result) setPreview(result);
              })}>Import records</button>
              <button type="button" className="text-button" onClick={reload}>Reload saved records</button>
            </div>
          </footer>
        </fieldset>
      )}
      {preview && !guard && (
        <Modal title="Review import" onCancel={() => { if (!busy) setPreview(null); }}>
          <p>File: <strong>{preview.file_name}</strong></p>
          <p>This replaces all local records with the records in this file. Records missing from the file will be removed. A backup of changed local records is created before replacement.</p>
          <table>
            <caption>Record counts</caption>
            <thead><tr><th scope="col">Records</th><th scope="col">Local</th><th scope="col">Incoming</th></tr></thead>
            <tbody>
              {(["projects", "objectives", "actions", "notes"] as const).map((key) => (
                <tr key={key}>
                  <th scope="row">{key[0].toUpperCase() + key.slice(1)}</th>
                  <td>{preview.current_counts[key]}</td><td>{preview.incoming_counts[key]}</td>
                </tr>
              ))}
            </tbody>
          </table>
          {error && (
            <p role="alert" className="modal-error">
              {error.message}{error.code === "conflict" && " Cancel and preview the file again to review the latest local records."}
            </p>
          )}
          <div className="button-row">
            <button type="button" className="secondary" disabled={busy} onClick={() => setPreview(null)}>Cancel import</button>
            <button type="button" disabled={busy} onClick={restore}>{busy ? "Restoring..." : "Replace local records"}</button>
          </div>
        </Modal>
      )}
      {guard && (
        <Modal title="Keep your unsaved draft?" onCancel={() => { guard.onKeep?.(); setGuard(null); }}>
          <p>{guard.description}</p>
          <div className="button-row">
            <button type="button" onClick={() => { guard.onKeep?.(); setGuard(null); }}>Keep editing</button>
            <button type="button" className="secondary" onClick={() => {
              const task = guard.action;
              setGuard(null);
              task();
            }}>Discard draft and continue</button>
          </div>
        </Modal>
      )}
    </main>
  );
}

export default App;
