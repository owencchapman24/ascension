use std::{
    fs::{self, File},
    io::{Read, Write},
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
};

use serde::Serialize;
use tauri::State;

use crate::progress::*;
use crate::service::{AcceptanceResult, ReviewResult};
use crate::{domain::*, service::WorkspaceService};

#[derive(Clone)]
pub(crate) struct AppState {
    service: Arc<Mutex<Result<WorkspaceService, CommandError>>>,
    data_dir: Option<PathBuf>,
}

fn error(code: &str, message: &str) -> CommandError {
    CommandError {
        code: code.into(),
        message: message.into(),
    }
}

fn storage_error() -> CommandError {
    error(
        "storage_error",
        "Local storage could not be opened. Close other Ascension windows and try reopening the app. Your records have not been replaced.",
    )
}

fn file_error() -> CommandError {
    error(
        "file_error",
        "The selected file could not be read or written. Check the location and try again.",
    )
}

impl AppState {
    pub(crate) fn open(data_dir: Option<PathBuf>) -> Self {
        let service = Self::open_service(data_dir.as_deref());
        Self {
            service: Arc::new(Mutex::new(service)),
            data_dir,
        }
    }

    fn open_service(data_dir: Option<&Path>) -> Result<WorkspaceService, CommandError> {
        let root = data_dir.ok_or_else(storage_error)?;
        fs::create_dir_all(root).map_err(|_| storage_error())?;
        WorkspaceService::open(&root.join("ascension.sqlite3"), &root.join("backups"))
    }

    fn with_service<T>(
        &self,
        operation: impl FnOnce(&mut WorkspaceService) -> Result<T, CommandError>,
    ) -> Result<T, CommandError> {
        let mut guard = self.service.lock().map_err(|_| storage_error())?;
        // A transient startup failure must not make the UI's retry permanently fail.
        // Reuse the same guarded open path; never recreate or replace a failed database.
        if guard.is_err() {
            *guard = Self::open_service(self.data_dir.as_deref());
        }
        let service = guard.as_mut().map_err(|failure| failure.clone())?;
        operation(service)
    }
}

async fn service_task<T: Send + 'static>(
    state: AppState,
    operation: impl FnOnce(&mut WorkspaceService) -> Result<T, CommandError> + Send + 'static,
) -> Result<T, CommandError> {
    tauri::async_runtime::spawn_blocking(move || state.with_service(operation))
        .await
        .map_err(|_| storage_error())?
}

#[tauri::command]
pub(crate) async fn load_workspace(state: State<'_, AppState>) -> Result<Workspace, CommandError> {
    service_task(state.inner().clone(), |service| service.load_workspace()).await
}

#[tauri::command]
pub(crate) async fn save_criterion(
    state: State<'_, AppState>,
    input: SaveCriterionInput,
) -> Result<Workspace, CommandError> {
    service_task(state.inner().clone(), move |service| {
        service.save_criterion(input)
    })
    .await
}
#[tauri::command]
pub(crate) async fn lock_criterion(
    state: State<'_, AppState>,
    input: LockCriterionInput,
) -> Result<Workspace, CommandError> {
    service_task(state.inner().clone(), move |service| {
        service.lock_criterion(input)
    })
    .await
}
#[tauri::command]
pub(crate) async fn record_evidence(
    state: State<'_, AppState>,
    input: RecordEvidenceInput,
) -> Result<Workspace, CommandError> {
    service_task(state.inner().clone(), move |service| {
        service.record_evidence(input)
    })
    .await
}
#[tauri::command]
pub(crate) async fn review_progress(
    state: State<'_, AppState>,
    input: ReviewProgressInput,
) -> Result<ReviewResult, CommandError> {
    service_task(state.inner().clone(), move |service| {
        service.review_progress(input)
    })
    .await
}
#[tauri::command]
pub(crate) async fn accept_progress(
    state: State<'_, AppState>,
    input: AcceptProgressInput,
) -> Result<AcceptanceResult, CommandError> {
    service_task(state.inner().clone(), move |service| {
        service.accept_progress(input)
    })
    .await
}
#[tauri::command]
pub(crate) async fn correct_acceptance(
    state: State<'_, AppState>,
    input: CorrectAcceptanceInput,
) -> Result<Workspace, CommandError> {
    service_task(state.inner().clone(), move |service| {
        service.correct_acceptance(input)
    })
    .await
}
#[tauri::command]
pub(crate) async fn reject_review(
    state: State<'_, AppState>,
    input: RejectReviewInput,
) -> Result<Workspace, CommandError> {
    service_task(state.inner().clone(), move |service| {
        service.reject_review(input)
    })
    .await
}

#[tauri::command]
pub(crate) async fn create_project(
    state: State<'_, AppState>,
    input: CreateProjectInput,
) -> Result<Workspace, CommandError> {
    service_task(state.inner().clone(), move |service| {
        service.create_project(input)
    })
    .await
}

#[tauri::command]
pub(crate) async fn update_project(
    state: State<'_, AppState>,
    input: UpdateProjectInput,
) -> Result<Workspace, CommandError> {
    service_task(state.inner().clone(), move |service| {
        service.update_project(input)
    })
    .await
}

#[tauri::command]
pub(crate) async fn create_objective(
    state: State<'_, AppState>,
    input: CreateObjectiveInput,
) -> Result<Workspace, CommandError> {
    service_task(state.inner().clone(), move |service| {
        service.create_objective(input)
    })
    .await
}

#[tauri::command]
pub(crate) async fn update_objective(
    state: State<'_, AppState>,
    input: UpdateObjectiveInput,
) -> Result<Workspace, CommandError> {
    service_task(state.inner().clone(), move |service| {
        service.update_objective(input)
    })
    .await
}

#[tauri::command]
pub(crate) async fn select_objective(
    state: State<'_, AppState>,
    input: SelectObjectiveInput,
) -> Result<Workspace, CommandError> {
    service_task(state.inner().clone(), move |service| {
        service.select_objective(input)
    })
    .await
}

#[tauri::command]
pub(crate) async fn save_work(
    state: State<'_, AppState>,
    input: SaveWorkInput,
) -> Result<Workspace, CommandError> {
    service_task(state.inner().clone(), move |service| {
        service.save_work(input)
    })
    .await
}

#[tauri::command]
pub(crate) async fn restore_import(
    state: State<'_, AppState>,
    input: RestoreInput,
) -> Result<RestoreResult, CommandError> {
    service_task(state.inner().clone(), move |service| {
        service.restore_import(input)
    })
    .await
}

#[derive(Serialize)]
pub(crate) struct ExportResult {
    file_name: String,
}

fn selected_name(path: &Path) -> String {
    path.file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_else(|| "Ascension records".into())
}

// The renderer never supplies a path. Native dialogs are the only file-selection boundary.
#[tauri::command]
pub(crate) async fn export_workspace(
    state: State<'_, AppState>,
) -> Result<Option<ExportResult>, CommandError> {
    let state = state.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        state.with_service(|_| Ok(()))?;
        let Some(path) = rfd::FileDialog::new()
            .set_title("Export saved Ascension records")
            .add_filter("Ascension records", &["json"])
            .set_file_name("ascension-records.json")
            .save_file()
        else {
            return Ok(None);
        };
        let bytes = state.with_service(|service| service.export_bytes())?;
        let root = state.data_dir.as_deref().ok_or_else(storage_error)?;
        write_export(&path, root, &bytes)?;
        Ok(Some(ExportResult {
            file_name: selected_name(&path),
        }))
    })
    .await
    .map_err(|_| file_error())?
}

#[tauri::command]
pub(crate) async fn preview_import(
    state: State<'_, AppState>,
) -> Result<Option<ImportPreview>, CommandError> {
    let state = state.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        state.with_service(|service| {
            service.discard_preview();
            Ok(())
        })?;
        let Some(path) = rfd::FileDialog::new()
            .set_title("Preview an Ascension export")
            .add_filter("Ascension records", &["json"])
            .pick_file()
        else {
            return Ok(None);
        };
        let bytes = read_import(&path)?;
        state
            .with_service(|service| service.preview_import(&bytes, selected_name(&path)))
            .map(Some)
    })
    .await
    .map_err(|_| file_error())?
}

fn read_import(path: &Path) -> Result<Vec<u8>, CommandError> {
    let file = File::open(path).map_err(|_| file_error())?;
    let metadata = file.metadata().map_err(|_| file_error())?;
    if !metadata.is_file() || metadata.len() > MAX_IMPORT_BYTES as u64 {
        return Err(error(
            "invalid_import",
            "Choose an Ascension JSON export no larger than 10 MiB.",
        ));
    }
    let mut bytes = Vec::new();
    file.take(MAX_IMPORT_BYTES as u64 + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| file_error())?;
    if bytes.len() > MAX_IMPORT_BYTES {
        return Err(error(
            "invalid_import",
            "Choose an Ascension JSON export no larger than 10 MiB.",
        ));
    }
    Ok(bytes)
}

fn write_export(path: &Path, app_data: &Path, bytes: &[u8]) -> Result<(), CommandError> {
    if !path
        .extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| extension.eq_ignore_ascii_case("json"))
    {
        return Err(error(
            "file_error",
            "Choose a filename ending in .json for the export.",
        ));
    }
    let parent = path.parent().ok_or_else(file_error)?;
    let parent = parent.canonicalize().map_err(|_| file_error())?;
    let app_data = app_data.canonicalize().map_err(|_| file_error())?;
    if parent.starts_with(&app_data)
        || path
            .canonicalize()
            .is_ok_and(|existing| existing.starts_with(&app_data))
    {
        return Err(error(
            "file_error",
            "Choose an export location outside Ascension's application-data folder.",
        ));
    }
    // Writing a sibling temporary file keeps an existing export intact on write failure.
    let mut temporary = tempfile::NamedTempFile::new_in(parent).map_err(|_| file_error())?;
    temporary.write_all(bytes).map_err(|_| file_error())?;
    temporary.as_file().sync_all().map_err(|_| file_error())?;
    let file = temporary.persist(path).map_err(|_| file_error())?;
    file.sync_all().map_err(|_| error("file_error", "The export was written, but Windows could not confirm it was saved durably. Try exporting again."))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn export_replaces_a_selected_file_and_protects_app_storage() {
        let directory = tempfile::tempdir().unwrap();
        let app_data = directory.path().join("app-data");
        fs::create_dir(&app_data).unwrap();
        let selected = directory.path().join("export.json");
        fs::write(&selected, "old export").unwrap();
        write_export(&selected, &app_data, b"new export").unwrap();
        assert_eq!(fs::read(&selected).unwrap(), b"new export");
        let protected = app_data.join("backup.json");
        fs::write(&protected, "saved backup").unwrap();
        assert_eq!(
            write_export(&protected, &app_data, b"replacement")
                .unwrap_err()
                .code,
            "file_error"
        );
        assert_eq!(fs::read(protected).unwrap(), b"saved backup");
        assert_eq!(
            write_export(
                &directory.path().join("records.sqlite3"),
                &app_data,
                b"wrong extension"
            )
            .unwrap_err()
            .code,
            "file_error"
        );
    }

    #[test]
    fn export_failure_preserves_existing_file_and_import_read_is_bounded() {
        let directory = tempfile::tempdir().unwrap();
        let app_data = directory.path().join("app-data");
        fs::create_dir(&app_data).unwrap();
        let selected = directory.path().join("kept.txt");
        fs::write(&selected, "original").unwrap();
        assert!(write_export(&selected, &app_data, b"replacement").is_err());
        assert_eq!(fs::read(&selected).unwrap(), b"original");
        let large = directory.path().join("oversized.json");
        File::create(&large)
            .unwrap()
            .set_len(MAX_IMPORT_BYTES as u64 + 1)
            .unwrap();
        assert_eq!(read_import(&large).unwrap_err().code, "invalid_import");
        assert_eq!(read_import(&selected).unwrap(), b"original");
        assert_eq!(
            read_import(&directory.path().join("absent.json"))
                .unwrap_err()
                .code,
            "file_error"
        );
    }

    #[test]
    fn exports_reject_normalized_app_data_paths_but_allow_a_similar_sibling() {
        let directory = tempfile::tempdir().unwrap();
        let app_data = directory.path().join("app-data");
        let nested = app_data.join("nested");
        let sibling = directory.path().join("app-data-exports");
        fs::create_dir_all(&nested).unwrap();
        fs::create_dir(&sibling).unwrap();
        for selected in [
            nested.join("backup.json"),
            nested.join("..").join("backup.json"),
        ] {
            assert_eq!(
                write_export(&selected, &app_data, b"replacement")
                    .unwrap_err()
                    .code,
                "file_error"
            );
            assert!(!selected.exists());
        }
        let selected = sibling.join("records.JSON");
        write_export(&selected, &app_data, b"export").unwrap();
        assert_eq!(fs::read(selected).unwrap(), b"export");
    }

    #[cfg(windows)]
    #[test]
    fn exports_reject_windows_case_aliases_of_app_data() {
        let directory = tempfile::tempdir().unwrap();
        let app_data = directory.path().join("app-data");
        fs::create_dir(&app_data).unwrap();
        let selected = directory.path().join("APP-DATA").join("backup.json");
        fs::write(&selected, b"existing backup").unwrap();
        assert_eq!(
            write_export(&selected, &app_data, b"replacement")
                .unwrap_err()
                .code,
            "file_error"
        );
        assert_eq!(fs::read(selected).unwrap(), b"existing backup");
    }

    #[cfg(windows)]
    #[test]
    fn locked_export_target_is_preserved_and_temporary_file_is_removed() {
        use std::os::windows::fs::OpenOptionsExt;

        let directory = tempfile::tempdir().unwrap();
        let app_data = directory.path().join("app-data");
        fs::create_dir(&app_data).unwrap();
        let selected = directory.path().join("records.json");
        fs::write(&selected, b"existing export").unwrap();
        let locked = fs::OpenOptions::new()
            .read(true)
            .share_mode(1) // FILE_SHARE_READ, deliberately excluding delete/replace.
            .open(&selected)
            .unwrap();
        assert_eq!(
            write_export(&selected, &app_data, b"replacement")
                .unwrap_err()
                .code,
            "file_error"
        );
        assert_eq!(fs::read(&selected).unwrap(), b"existing export");
        assert_eq!(fs::read_dir(directory.path()).unwrap().count(), 2);
        drop(locked);
        write_export(&selected, &app_data, b"replacement").unwrap();
        assert_eq!(fs::read(selected).unwrap(), b"replacement");
    }

    #[test]
    fn import_read_accepts_the_exact_size_limit_and_rejects_a_directory() {
        let directory = tempfile::tempdir().unwrap();
        let selected = directory.path().join("boundary.json");
        File::create(&selected)
            .unwrap()
            .set_len(MAX_IMPORT_BYTES as u64)
            .unwrap();
        assert_eq!(read_import(&selected).unwrap().len(), MAX_IMPORT_BYTES);
        assert!(read_import(directory.path()).is_err());
    }

    #[test]
    fn unavailable_storage_returns_a_safe_error_without_replacing_existing_bytes() {
        let absent = AppState::open(None);
        assert_eq!(
            absent
                .with_service(|service| service.load_workspace())
                .unwrap_err()
                .code,
            "storage_error"
        );
        let directory = tempfile::tempdir().unwrap();
        let database = directory.path().join("ascension.sqlite3");
        let original = b"synthetic corrupt database";
        fs::write(&database, original).unwrap();
        let state = AppState::open(Some(directory.path().into()));
        let failure = state
            .with_service(|service| service.load_workspace())
            .unwrap_err();
        assert_eq!(failure.code, "storage_error");
        assert!(
            !failure
                .message
                .contains(&directory.path().to_string_lossy().to_string())
        );
        assert_eq!(fs::read(database).unwrap(), original);
    }

    #[cfg(windows)]
    #[test]
    fn retry_recovers_after_a_startup_file_lock_clears_without_replacing_records() {
        use std::os::windows::fs::OpenOptionsExt;

        let directory = tempfile::tempdir().unwrap();
        let original = {
            let state = AppState::open(Some(directory.path().into()));
            state
                .with_service(|service| {
                    let initial = service.load_workspace()?;
                    service.create_project(CreateProjectInput {
                        expected_generation: initial.generation,
                        title: "Synthetic records survive startup retry".into(),
                    })
                })
                .unwrap()
        };
        let locked = fs::OpenOptions::new()
            .read(true)
            .write(true)
            .share_mode(0)
            .open(directory.path().join("ascension.sqlite3"))
            .unwrap();
        let state = AppState::open(Some(directory.path().into()));
        assert_eq!(
            state
                .with_service(|service| service.load_workspace())
                .unwrap_err()
                .code,
            "storage_error"
        );
        drop(locked);
        assert_eq!(
            state
                .with_service(|service| service.load_workspace())
                .unwrap(),
            original
        );
    }
}
