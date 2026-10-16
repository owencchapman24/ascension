import { invoke, isTauri } from "@tauri-apps/api/core";

export interface AppStatus {
  app_name: string;
  schema_version: number;
}

export async function getAppStatus(): Promise<AppStatus> {
  if (!isTauri()) {
    throw new Error("Open the Ascension desktop app to use your local workspace. A native desktop connection is required.");
  }
  return invoke<AppStatus>("get_app_status");
}
