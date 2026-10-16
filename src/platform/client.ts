import { invoke, isTauri } from "@tauri-apps/api/core";

export interface RecordMetadata { id: string; revision: number; created_at: string; updated_at: string }
export interface Project extends RecordMetadata { title: string }
export interface Objective extends RecordMetadata { project_id: string; title: string; criterion: string }
export interface NextAction extends RecordMetadata { objective_id: string; description: string; stop_condition: string }
export interface ResumptionNote extends RecordMetadata { objective_id: string; text: string }
export interface Workspace {
  generation: string;
  selected_objective_id: string | null;
  projects: Project[];
  objectives: Objective[];
  actions: NextAction[];
  notes: ResumptionNote[];
}
export interface RecordCounts { projects: number; objectives: number; actions: number; notes: number }
export interface ExportResult { file_name: string }
export interface ImportPreview {
  token: string;
  generation: string;
  file_name: string;
  current_counts: RecordCounts;
  incoming_counts: RecordCounts;
}
export interface RestoreResult { workspace: Workspace; backup_created: boolean; unchanged: boolean }
export interface CommandError { code: string; message: string }
interface GenerationInput { expected_generation: string }
export interface ProjectInput extends GenerationInput { title: string }
export interface ObjectiveInput extends GenerationInput { project_id: string; title: string; criterion: string }
export interface SaveWorkInput extends GenerationInput {
  objective_id: string;
  description: string;
  stop_condition: string;
  note: string | null;
}

async function command<T>(name: string, input?: object): Promise<T> {
  if (!isTauri()) {
    throw new Error("Open the Ascension desktop app to use your local workspace. A native desktop connection is required.");
  }
  return input === undefined ? invoke<T>(name) : invoke<T>(name, { input });
}

export const loadWorkspace = () => command<Workspace>("load_workspace");
export const createProject = (input: ProjectInput) => command<Workspace>("create_project", input);
export const updateProject = (input: ProjectInput & { id: string }) => command<Workspace>("update_project", input);
export const createObjective = (input: ObjectiveInput) => command<Workspace>("create_objective", input);
export const updateObjective = (input: GenerationInput & { id: string; title: string; criterion: string }) => command<Workspace>("update_objective", input);
export const selectObjective = (input: GenerationInput & { id: string | null }) => command<Workspace>("select_objective", input);
export const saveWork = (input: SaveWorkInput) => command<Workspace>("save_work", input);
export const exportWorkspace = () => command<ExportResult | null>("export_workspace");
export const previewImport = () => command<ImportPreview | null>("preview_import");
export const restoreImport = (input: { token: string; expected_generation: string }) => command<RestoreResult>("restore_import", input);

export function normalizeError(error: unknown): CommandError {
  if (typeof error === "object" && error !== null && "code" in error && "message" in error && typeof error.code === "string" && typeof error.message === "string") {
    return { code: error.code, message: error.message };
  }
  if (error instanceof Error) return { code: "connection_error", message: error.message };
  return { code: "connection_error", message: "The desktop app could not finish this request. Your draft has been kept. Please try again." };
}
