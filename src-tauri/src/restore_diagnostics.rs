//! Opt-in, bounded engineering diagnostics. Never serialize an input or raw error.
use crate::backup::{BackupPhase, BackupProbe};
use crate::domain::CommandError;
use serde::Serialize;
use std::cell::RefCell;
use std::fs::{self, OpenOptions};
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::Instant;

const FILE_NAME: &str = "restore-diagnostics-v1.jsonl";
const HEADER: &[u8] = b"{\"diagnostic_format\":\"ascension-restore-diagnostics-v1\"}\n";
const MAX_BYTES: u64 = 4 * 1024 * 1024;
static NATIVE: OnceLock<Option<Arc<Sink>>> = OnceLock::new();

struct Sink {
    directory: PathBuf,
    sequence: AtomicU64,
    writer: Mutex<()>,
}

fn regular(metadata: &fs::Metadata) -> bool {
    if metadata.file_type().is_symlink() {
        return false;
    }
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        if metadata.file_attributes() & 0x400 != 0 {
            return false;
        }
    }
    true
}

impl Sink {
    fn new(directory: &Path) -> Arc<Self> {
        Arc::new(Self {
            directory: directory.into(),
            sequence: AtomicU64::new(1),
            writer: Mutex::new(()),
        })
    }

    fn append(&self, entry: &Entry) -> std::io::Result<()> {
        let _lock = self.writer.lock().map_err(|_| std::io::ErrorKind::Other)?;
        let directory = fs::symlink_metadata(&self.directory)?;
        if !regular(&directory) || !directory.is_dir() {
            return Err(std::io::ErrorKind::InvalidInput.into());
        }
        let path = self.directory.join(FILE_NAME);
        let mut options = OpenOptions::new();
        options.read(true).append(true);
        #[cfg(windows)]
        {
            use std::os::windows::fs::OpenOptionsExt;
            options.custom_flags(0x0020_0000); // FILE_FLAG_OPEN_REPARSE_POINT
            options.share_mode(1); // Read sharing only; no concurrent writer or replacement.
        }
        let mut file = match options.create_new(true).open(&path) {
            Ok(mut file) => {
                file.write_all(HEADER)?;
                file
            }
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
                let metadata = fs::symlink_metadata(&path)?;
                if !regular(&metadata) || !metadata.is_file() || metadata.len() > MAX_BYTES {
                    return Err(std::io::ErrorKind::InvalidInput.into());
                }
                options.create_new(false);
                let mut file = options.open(&path)?;
                if !regular(&file.metadata()?) {
                    return Err(std::io::ErrorKind::InvalidInput.into());
                }
                let mut header = vec![0; HEADER.len()];
                file.read_exact(&mut header)?;
                if header != HEADER {
                    return Err(std::io::ErrorKind::InvalidData.into());
                }
                file
            }
            Err(error) => return Err(error),
        };
        let mut bytes = serde_json::to_vec(entry)?;
        bytes.push(b'\n');
        let length = file.seek(SeekFrom::End(0))?;
        if length.saturating_add(bytes.len() as u64) > MAX_BYTES {
            return Err(std::io::ErrorKind::FileTooLarge.into());
        }
        file.write_all(&bytes)?;
        file.flush()
    }
}

fn permitted(identifier: &str, enabled: bool) -> bool {
    enabled
        && identifier
            .strip_prefix("io.ascension.verification.r")
            .is_some_and(|suffix| {
                suffix.len() == 32
                    && suffix
                        .bytes()
                        .all(|v| v.is_ascii_digit() || (b'a'..=b'f').contains(&v))
            })
}

pub(crate) fn configure_native(identifier: &str, directory: Option<&Path>) {
    let enabled = permitted(
        identifier,
        std::env::var_os("ASCENSION_RESTORE_DIAGNOSTICS").is_some_and(|v| v == "1"),
    );
    let _ = NATIVE.set(if enabled {
        directory.map(Sink::new)
    } else {
        None
    });
}

#[derive(Serialize)]
struct Entry {
    diagnostic_version: u8,
    attempt_sequence: u64,
    phase: &'static str,
    operation: &'static str,
    sqlite_primary_code: Option<i32>,
    sqlite_extended_code: Option<i32>,
    filesystem_category: Option<&'static str>,
    os_error_code: Option<i32>,
    command_error_code: Option<&'static str>,
    outcome: &'static str,
    transaction_outcome: &'static str,
    rollback_sqlite_primary_code: Option<i32>,
    rollback_sqlite_extended_code: Option<i32>,
    displaced_backup_created: bool,
    backup_phase_start: Option<BackupPhase>,
    backup_phase_failure: Option<BackupPhase>,
    backup_phase_end: Option<BackupPhase>,
    elapsed_ms: u64,
}

struct Attempt {
    sink: Arc<Sink>,
    entry: Entry,
    started: Instant,
    failed: bool,
    probe: Option<BackupProbe>,
}

thread_local! {
    static ACTIVE: RefCell<Option<Attempt>> = const { RefCell::new(None) };
    #[cfg(test)]
    static TEST_SINK: RefCell<Option<Arc<Sink>>> = const { RefCell::new(None) };
}

fn change(f: impl FnOnce(&mut Attempt)) {
    ACTIVE.with(|active| {
        if let Some(attempt) = active.borrow_mut().as_mut() {
            f(attempt);
        }
    });
}

struct Guard(bool);
impl Drop for Guard {
    fn drop(&mut self) {
        if self.0 {
            ACTIVE.with(|active| {
                if let Some(mut attempt) = active.borrow_mut().take() {
                    attempt.entry.backup_phase_end = attempt.probe.as_ref().map(BackupProbe::phase);
                    attempt.entry.elapsed_ms =
                        attempt.started.elapsed().as_millis().min(u64::MAX as u128) as u64;
                    // Diagnostic failure never changes the result of saved work.
                    let _ = attempt.sink.append(&attempt.entry);
                }
            });
        }
    }
}

pub(crate) fn attempt<T>(f: impl FnOnce() -> Result<T, CommandError>) -> Result<T, CommandError> {
    let nested = ACTIVE.with(|active| active.borrow().is_some());
    if nested {
        return f();
    }
    #[cfg(test)]
    let sink = TEST_SINK
        .with(|value| value.borrow().clone())
        .or_else(|| NATIVE.get().cloned().flatten());
    #[cfg(not(test))]
    let sink = NATIVE.get().cloned().flatten();
    let Some(sink) = sink else {
        return f();
    };
    let sequence = sink.sequence.fetch_add(1, Ordering::Relaxed);
    ACTIVE.with(|active| {
        *active.borrow_mut() = Some(Attempt {
            entry: Entry {
                diagnostic_version: 1,
                attempt_sequence: sequence,
                phase: "service_access",
                operation: "service_access",
                sqlite_primary_code: None,
                sqlite_extended_code: None,
                filesystem_category: None,
                os_error_code: None,
                command_error_code: None,
                outcome: "interrupted",
                transaction_outcome: "not_started",
                rollback_sqlite_primary_code: None,
                rollback_sqlite_extended_code: None,
                displaced_backup_created: false,
                backup_phase_start: None,
                backup_phase_failure: None,
                backup_phase_end: None,
                elapsed_ms: 0,
            },
            sink,
            started: Instant::now(),
            failed: false,
            probe: None,
        });
    });
    let _guard = Guard(true);
    let result = f();
    change(|attempt| match &result {
        Ok(_) => {
            if attempt.entry.outcome != "unchanged" {
                attempt.entry.outcome = "success";
            }
        }
        Err(error) => {
            fail(attempt);
            attempt.entry.outcome = "error";
            attempt.entry.command_error_code = Some(match error.code.as_str() {
                "storage_error" => "storage_error",
                "backup_failed" => "backup_failed",
                "busy" => "busy",
                "conflict" => "conflict",
                "invalid_input" => "invalid_input",
                "invalid_import" => "invalid_import",
                "unsupported_schema" => "unsupported_schema",
                "unsafe_legacy_import" => "unsafe_legacy_import",
                _ => "other",
            });
        }
    });
    result
}

fn fail(attempt: &mut Attempt) {
    if !attempt.failed {
        attempt.entry.backup_phase_failure = attempt.probe.as_ref().map(BackupProbe::phase);
        attempt.failed = true;
    }
}

pub(crate) fn attach_backup(probe: BackupProbe) {
    change(|attempt| {
        attempt.entry.backup_phase_start = Some(probe.phase());
        attempt.probe = Some(probe);
    });
}
pub(crate) fn phase(value: &'static str) {
    change(|attempt| {
        if !attempt.failed {
            attempt.entry.phase = value;
            attempt.entry.operation = value;
        }
    });
}
pub(crate) fn table_operation(action: &'static str, table: &str) {
    // Table names are selected by private SQL helpers. Retain only an allowlist.
    let table = match table {
        "progress_parents" => "progress_parents",
        "reward_slots" => "reward_slots",
        "criterion_versions" => "criterion_versions",
        "validator_definitions" => "validator_definitions",
        "evidence_records" => "evidence_records",
        "review_snapshots" => "review_snapshots",
        "acceptances" => "acceptances",
        "reward_grants" => "reward_grants",
        "corrections" => "corrections",
        "ledger_entries" => "ledger_entries",
        "progress_audit" => "progress_audit",
        "committed_commands" => "committed_commands",
        "schedule_epochs" => "schedule_epochs",
        "study_manifests" => "study_manifests",
        "study_protocol_events" => "study_protocol_events",
        "work_opportunities" => "work_opportunities",
        "study_assignments" => "study_assignments",
        "opportunity_outcomes" => "opportunity_outcomes",
        "study_exposures" => "study_exposures",
        "study_burdens" => "study_burdens",
        "study_reflections" => "study_reflections",
        "presentation_preferences" => "presentation_preferences",
        "world_chapter" => "world_chapter",
        "world_spending" => "world_spending",
        "world_commands" => "world_commands",
        "objective_planning" => "objective_planning",
        "work_checkpoints" => "work_checkpoints",
        "checkpoint_commands" => "checkpoint_commands",
        _ => "other_table",
    };
    change(|attempt| {
        if !attempt.failed {
            attempt.entry.phase = action;
            attempt.entry.operation = table;
        }
    });
}
pub(crate) fn sqlite(error: &rusqlite::Error) {
    change(|attempt| {
        if !attempt.failed {
            if let Some(error) = error.sqlite_error() {
                attempt.entry.sqlite_primary_code = Some(error.extended_code & 0xff);
                attempt.entry.sqlite_extended_code = Some(error.extended_code);
            }
            fail(attempt);
        }
    });
}
pub(crate) fn filesystem(error: &std::io::Error) {
    change(|attempt| {
        if !attempt.failed {
            attempt.entry.filesystem_category = Some(match error.kind() {
                std::io::ErrorKind::NotFound => "not_found",
                std::io::ErrorKind::PermissionDenied => "permission_denied",
                std::io::ErrorKind::AlreadyExists => "already_exists",
                std::io::ErrorKind::NotADirectory => "not_a_directory",
                std::io::ErrorKind::IsADirectory => "is_a_directory",
                std::io::ErrorKind::StorageFull => "storage_full",
                std::io::ErrorKind::WriteZero => "write_zero",
                std::io::ErrorKind::Interrupted => "interrupted",
                std::io::ErrorKind::InvalidInput => "invalid_input",
                _ => "other",
            });
            attempt.entry.os_error_code = error.raw_os_error();
            fail(attempt);
        }
    });
}
pub(crate) fn transaction(value: &'static str) {
    change(|attempt| attempt.entry.transaction_outcome = value);
}
pub(crate) fn rollback_error(error: &rusqlite::Error) {
    change(|attempt| {
        attempt.entry.transaction_outcome = "rollback_failed";
        if let Some(error) = error.sqlite_error() {
            attempt.entry.rollback_sqlite_primary_code = Some(error.extended_code & 0xff);
            attempt.entry.rollback_sqlite_extended_code = Some(error.extended_code);
        }
    });
}
pub(crate) fn backup_created() {
    change(|attempt| attempt.entry.displaced_backup_created = true);
}
pub(crate) fn unchanged() {
    change(|attempt| attempt.entry.outcome = "unchanged");
}

#[cfg(test)]
pub(crate) fn with_test_sink<T>(directory: &Path, f: impl FnOnce() -> T) -> T {
    struct Reset(Option<Arc<Sink>>);
    impl Drop for Reset {
        fn drop(&mut self) {
            TEST_SINK.with(|sink| *sink.borrow_mut() = self.0.take());
        }
    }
    let prior = TEST_SINK.with(|sink| sink.replace(Some(Sink::new(directory))));
    let _reset = Reset(prior);
    f()
}

#[cfg(test)]
mod tests;
