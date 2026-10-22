//! Private, coherent daily recovery. Filesystem work never holds the app's service lock.
use crate::domain::{CommandError, RecordSet, encoded_records, new_id, now, valid_id};
use crate::storage::{DB_SCHEMA_VERSION, read_workspace};
use chrono::{Datelike, Local, NaiveDate};
use rusqlite::{
    Connection, OpenFlags,
    backup::{Backup, StepResult},
    params,
};
use serde::Serialize;
use std::collections::{BTreeSet, HashSet, VecDeque};
use std::fs::{self, OpenOptions};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

const OWNER: &str = "ascension-daily-recovery-v1";
const PREFIX: &str = "daily-v1-";
const SUFFIX: &str = ".sqlite3";
const FAILURE: &str = "Automatic recovery could not be verified. Your saved changes are still saved. Retry recovery protection or export a portable copy.";
const RETENTION_FAILURE: &str = "The latest recovery snapshot is verified, but older automatic snapshots could not all be pruned. Your recovery files have been retained.";

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum BackupState {
    Checking,
    Idle,
    Running,
    Ready,
    Warning,
}

#[derive(Clone, Debug, Serialize)]
pub struct BackupStatus {
    pub state: BackupState,
    pub latest_successful_day: Option<String>,
    pub snapshot_count: usize,
    pub warning: Option<String>,
}

/// Coarse, content-free state for an explicitly enabled restore diagnostic.
#[derive(Clone, Copy, Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub(crate) enum BackupPhase {
    Idle,
    Inventory,
    Snapshot,
    Retention,
    Unknown,
}

#[derive(Clone)]
pub(crate) struct BackupProbe(Arc<Mutex<Shared>>);

impl BackupProbe {
    pub(crate) fn phase(&self) -> BackupPhase {
        self.0
            .lock()
            .map_or(BackupPhase::Unknown, |state| state.phase)
    }
}

struct Shared {
    status: BackupStatus,
    queue: VecDeque<NaiveDate>,
    completed: HashSet<NaiveDate>,
    running: bool,
    phase: BackupPhase,
}

// The worker owns only Shared, so dropping the manager can join it without a
// reference cycle. This also keeps disposable profiles from outliving tests.
pub(crate) struct BackupManager {
    database: PathBuf,
    directory: PathBuf,
    shared: Arc<Mutex<Shared>>,
    worker: Mutex<Option<JoinHandle<()>>>,
}

#[derive(Clone)]
struct Snapshot {
    path: PathBuf,
    day: NaiveDate,
    id: String,
}

impl BackupManager {
    pub(crate) fn new(database: &Path, directory: &Path) -> Self {
        let manager = Self {
            database: database.into(),
            directory: directory.into(),
            shared: Arc::new(Mutex::new(Shared {
                status: BackupStatus {
                    state: BackupState::Checking,
                    latest_successful_day: None,
                    snapshot_count: 0,
                    warning: None,
                },
                queue: VecDeque::new(),
                completed: HashSet::new(),
                running: false,
                phase: BackupPhase::Idle,
            })),
            worker: Mutex::new(None),
        };
        manager.start_worker(true);
        manager
    }

    pub(crate) fn data_changed(&self) {
        self.request(Local::now().date_naive());
    }

    pub(crate) fn diagnostic_probe(&self) -> BackupProbe {
        BackupProbe(Arc::clone(&self.shared))
    }

    pub(crate) fn status(&self) -> BackupStatus {
        self.shared
            .lock()
            .map(|s| {
                let mut status = s.status.clone();
                // Do not expose a transient idle/ready result between inventory
                // and queued changed-data work; the UI must keep polling.
                if s.running && status.state != BackupState::Checking {
                    status.state = BackupState::Running;
                }
                status
            })
            .unwrap_or_else(|_| BackupStatus {
                state: BackupState::Warning,
                latest_successful_day: None,
                snapshot_count: 0,
                warning: Some(FAILURE.into()),
            })
    }

    pub(crate) fn retry(&self) -> BackupStatus {
        // An explicit retry is allowed without changing application records.
        let day = Local::now().date_naive();
        if let Ok(mut shared) = self.shared.lock() {
            shared.completed.remove(&day);
        }
        self.request(day);
        self.status()
    }

    pub(crate) fn latest_export(&self) -> Result<(Vec<u8>, String), CommandError> {
        latest_export(&self.directory)
    }

    fn request(&self, day: NaiveDate) {
        if let Ok(mut shared) = self.shared.lock() {
            if shared.completed.contains(&day) {
                return;
            }
            if !shared.queue.contains(&day) {
                shared.queue.push_back(day);
            }
            shared.status.state = BackupState::Running;
        }
        self.start_worker(false);
    }

    fn start_worker(&self, inventory: bool) {
        let Ok(mut handle) = self.worker.lock() else {
            return;
        };
        let Ok(mut shared) = self.shared.lock() else {
            return;
        };
        if shared.running || (!inventory && shared.queue.is_empty()) {
            return;
        }
        shared.running = true;
        shared.phase = if inventory {
            BackupPhase::Inventory
        } else {
            BackupPhase::Snapshot
        };
        drop(shared);
        if let Some(previous) = handle.take() {
            let _ = previous.join();
        }
        let shared = Arc::clone(&self.shared);
        let database = self.database.clone();
        let directory = self.directory.clone();
        match thread::Builder::new()
            .name("ascension-recovery".into())
            .spawn(move || {
                run_worker(&database, &directory, &shared, inventory);
            }) {
            Ok(worker) => *handle = Some(worker),
            Err(_) => {
                if let Ok(mut shared) = self.shared.lock() {
                    shared.running = false;
                    shared.phase = BackupPhase::Idle;
                    shared.status.state = BackupState::Warning;
                    shared.status.warning = Some(FAILURE.into());
                }
            }
        }
    }
}

impl Drop for BackupManager {
    fn drop(&mut self) {
        if let Ok(handle) = self.worker.get_mut()
            && let Some(worker) = handle.take()
        {
            let _ = worker.join();
        }
    }
}

fn run_worker(database: &Path, directory: &Path, shared: &Mutex<Shared>, inventory: bool) {
    if inventory {
        set_phase(shared, BackupPhase::Inventory);
        #[cfg(test)]
        let result = test_gate::during_inventory(database, || inventory_snapshots(directory));
        #[cfg(not(test))]
        let result = inventory_snapshots(directory);
        if let Ok(mut state) = shared.lock() {
            match result {
                Ok((snapshots, unverified)) => {
                    update_status(&mut state, &snapshots, unverified.then_some(FAILURE));
                }
                Err(_) => update_status(&mut state, &[], Some(FAILURE)),
            }
        }
    }
    loop {
        let day = {
            let Ok(mut state) = shared.lock() else {
                return;
            };
            match state.queue.pop_front() {
                Some(day) => {
                    if state.completed.contains(&day) {
                        continue;
                    }
                    state.status.state = BackupState::Running;
                    day
                }
                None => {
                    state.running = false;
                    state.phase = BackupPhase::Idle;
                    return;
                }
            }
        };
        set_phase(shared, BackupPhase::Snapshot);
        let result = create_snapshot(database, directory, day).and_then(|_| {
            set_phase(shared, BackupPhase::Retention);
            let (snapshots, unverified) = inventory_snapshots(directory)?;
            let pruning_failed = prune(&snapshots).is_err();
            let (remaining, still_unverified) = inventory_snapshots(directory)?;
            let warning = if pruning_failed {
                Some(RETENTION_FAILURE)
            } else if unverified || still_unverified {
                Some(FAILURE)
            } else {
                None
            };
            Ok((remaining, warning))
        });
        let (snapshots, warning) = result.unwrap_or_else(|_| {
            // A failed retry may reveal that a formerly verified file is now
            // corrupt or unavailable. Refresh the claim without any pruning.
            set_phase(shared, BackupPhase::Inventory);
            let verified = inventory_snapshots(directory)
                .map(|(snapshots, _)| snapshots)
                .unwrap_or_default();
            (verified, Some(FAILURE))
        });
        if let Ok(mut state) = shared.lock() {
            update_status(&mut state, &snapshots, warning);
        }
    }
}

fn set_phase(shared: &Mutex<Shared>, phase: BackupPhase) {
    if let Ok(mut state) = shared.lock() {
        state.phase = phase;
    }
}

fn update_status(state: &mut Shared, snapshots: &[Snapshot], warning: Option<&str>) {
    state.completed = snapshots.iter().map(|snapshot| snapshot.day).collect();
    state.status.latest_successful_day = snapshots
        .iter()
        .map(|snapshot| snapshot.day)
        .max()
        .map(|day| day.to_string());
    state.status.snapshot_count = snapshots.len();
    state.status.warning = warning.map(str::to_owned);
    state.status.state = if warning.is_some() {
        BackupState::Warning
    } else if snapshots.is_empty() {
        BackupState::Idle
    } else {
        BackupState::Ready
    };
}

fn backup_error() -> CommandError {
    CommandError::new("backup_failed", FAILURE)
}

fn filename(day: NaiveDate) -> String {
    format!("{PREFIX}{day}{SUFFIX}")
}

fn parse_filename(path: &Path) -> Option<NaiveDate> {
    let name = path.file_name()?.to_str()?;
    let day = name.strip_prefix(PREFIX)?.strip_suffix(SUFFIX)?;
    let date = NaiveDate::parse_from_str(day, "%Y-%m-%d").ok()?;
    (filename(date) == name).then_some(date)
}

fn create_snapshot(database: &Path, directory: &Path, day: NaiveDate) -> Result<(), CommandError> {
    fs::create_dir_all(directory).map_err(|_| backup_error())?;
    verify_directory(directory)?;
    let final_path = directory.join(filename(day));
    if final_path.try_exists().map_err(|_| backup_error())? {
        inspect_snapshot(&final_path, day)?;
        return Ok(());
    }
    let pending = tempfile::Builder::new()
        .prefix(".daily-recovery-pending-")
        .suffix(".sqlite3.tmp")
        .tempfile_in(directory)
        .map_err(|_| backup_error())?
        .into_temp_path();
    let source = Connection::open_with_flags(database, OpenFlags::SQLITE_OPEN_READ_ONLY)
        .map_err(|_| backup_error())?;
    source
        .busy_timeout(Duration::from_secs(2))
        .map_err(|_| backup_error())?;
    let transaction = source.unchecked_transaction().map_err(|_| backup_error())?;
    // Pin a read snapshot before backup; concurrent WAL writers can continue.
    let _: String = transaction
        .query_row(
            "SELECT generation FROM workspace_state WHERE singleton=1",
            [],
            |row| row.get(0),
        )
        .map_err(|_| backup_error())?;
    #[cfg(test)]
    test_gate::pause(database, test_gate::Point::SnapshotPinned);
    let mut destination = Connection::open(&pending).map_err(|_| backup_error())?;
    {
        let backup = Backup::new(&transaction, &mut destination).map_err(|_| backup_error())?;
        let deadline = Instant::now() + Duration::from_secs(15);
        loop {
            if Instant::now() >= deadline {
                return Err(backup_error());
            }
            match backup.step(128).map_err(|_| backup_error())? {
                StepResult::Done => break,
                StepResult::More => {}
                StepResult::Busy | StepResult::Locked => thread::sleep(Duration::from_millis(5)),
                _ => return Err(backup_error()),
            }
        }
    }
    transaction.commit().map_err(|_| backup_error())?;
    destination
        .pragma_update(None, "journal_mode", "DELETE")
        .map_err(|_| backup_error())?;
    destination
        .pragma_update(None, "synchronous", "FULL")
        .map_err(|_| backup_error())?;
    let has_marker: bool = destination
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE name='ascension_daily_recovery')",
            [],
            |row| row.get(0),
        )
        .map_err(|_| backup_error())?;
    if has_marker {
        // A user may have recovered by opening a standalone snapshot. Replace
        // only its verified subsystem marker, and only inside this new clone.
        snapshot_marker(&destination)?;
        destination
            .execute_batch("DROP TABLE ascension_daily_recovery;")
            .map_err(|_| backup_error())?;
    }
    destination.execute_batch("CREATE TABLE ascension_daily_recovery(singleton INTEGER PRIMARY KEY CHECK(singleton=1),owner TEXT NOT NULL,snapshot_id TEXT NOT NULL,local_day TEXT NOT NULL,created_at TEXT NOT NULL);")
        .map_err(|_| backup_error())?;
    destination
        .execute(
            "INSERT INTO ascension_daily_recovery VALUES(1,?1,?2,?3,?4)",
            params![OWNER, new_id(), day.to_string(), now()],
        )
        .map_err(|_| backup_error())?;
    drop(destination);
    // Validation includes both SQLite and the complete application authority graph.
    inspect_snapshot(&pending, day)?;
    OpenOptions::new()
        .write(true)
        .open(&pending)
        .and_then(|file| file.sync_all())
        .map_err(|_| backup_error())?;
    if pending.persist_noclobber(&final_path).is_err() {
        // Another process may have won the same-day race. Never overwrite it.
        inspect_snapshot(&final_path, day)?;
    }
    inspect_snapshot(&final_path, day)?;
    Ok(())
}

fn inspect_snapshot(path: &Path, day: NaiveDate) -> Result<(Snapshot, RecordSet), CommandError> {
    let metadata = fs::symlink_metadata(path).map_err(|_| backup_error())?;
    if !metadata.file_type().is_file() || is_link(&metadata) {
        return Err(backup_error());
    }
    let connection = Connection::open_with_flags(path, OpenFlags::SQLITE_OPEN_READ_ONLY)
        .map_err(|_| backup_error())?;
    let transaction = connection
        .unchecked_transaction()
        .map_err(|_| backup_error())?;
    let version: u32 = transaction
        .pragma_query_value(None, "user_version", |row| row.get(0))
        .map_err(|_| backup_error())?;
    #[cfg(test)]
    test_gate::inventory_reader_pinned();
    if version != DB_SCHEMA_VERSION {
        return Err(backup_error());
    }
    let check: String = transaction
        .query_row("PRAGMA quick_check", [], |row| row.get(0))
        .map_err(|_| backup_error())?;
    let foreign_keys: i64 = transaction
        .query_row("SELECT COUNT(*) FROM pragma_foreign_key_check", [], |row| {
            row.get(0)
        })
        .map_err(|_| backup_error())?;
    if check != "ok" || foreign_keys != 0 {
        return Err(backup_error());
    }
    let (id, recorded_day) = snapshot_marker(&transaction)?;
    if recorded_day != day {
        return Err(backup_error());
    }
    let workspace = read_workspace(&transaction).map_err(|_| backup_error())?;
    let records = RecordSet::from(&workspace);
    encoded_records(&records).map_err(|_| backup_error())?;
    transaction.commit().map_err(|_| backup_error())?;
    Ok((
        Snapshot {
            path: path.into(),
            day,
            id,
        },
        records,
    ))
}

fn snapshot_marker(connection: &Connection) -> Result<(String, NaiveDate), CommandError> {
    let count: u32 = connection
        .query_row("SELECT COUNT(*) FROM ascension_daily_recovery", [], |row| {
            row.get(0)
        })
        .map_err(|_| backup_error())?;
    let (owner, id, local_day): (String, String, String) = connection
        .query_row(
            "SELECT owner,snapshot_id,local_day FROM ascension_daily_recovery WHERE singleton=1",
            [],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .map_err(|_| backup_error())?;
    let date = NaiveDate::parse_from_str(&local_day, "%Y-%m-%d").map_err(|_| backup_error())?;
    if count != 1 || owner != OWNER || local_day != date.to_string() || valid_id(&id).is_err() {
        return Err(backup_error());
    }
    Ok((id, date))
}

fn inventory_snapshots(directory: &Path) -> Result<(Vec<Snapshot>, bool), CommandError> {
    if !directory.try_exists().map_err(|_| backup_error())? {
        return Ok((Vec::new(), false));
    }
    verify_directory(directory)?;
    let mut snapshots = Vec::new();
    let mut unverified = false;
    for entry in fs::read_dir(directory).map_err(|_| backup_error())? {
        let path = entry.map_err(|_| backup_error())?.path();
        let Some(day) = parse_filename(&path) else {
            continue;
        };
        match inspect_snapshot(&path, day) {
            Ok((snapshot, _)) => snapshots.push(snapshot),
            Err(_) => unverified = true,
        }
    }
    snapshots.sort_by_key(|snapshot| snapshot.day);
    Ok((snapshots, unverified))
}

fn is_link(metadata: &fs::Metadata) -> bool {
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        // Reject junctions and other reparse points as well as symlinks.
        metadata.file_attributes() & 0x400 != 0
    }
    #[cfg(not(windows))]
    {
        metadata.file_type().is_symlink()
    }
}

fn verify_directory(directory: &Path) -> Result<(), CommandError> {
    let metadata = fs::symlink_metadata(directory).map_err(|_| backup_error())?;
    if !metadata.is_dir() || is_link(&metadata) {
        return Err(backup_error());
    }
    Ok(())
}

fn retention_days(snapshots: &[Snapshot]) -> BTreeSet<NaiveDate> {
    let mut descending: Vec<_> = snapshots.iter().map(|snapshot| snapshot.day).collect();
    descending.sort_unstable_by(|a, b| b.cmp(a));
    descending.dedup();
    let mut keep: BTreeSet<_> = descending.iter().take(7).copied().collect();
    let mut months = HashSet::new();
    for day in descending {
        if months.insert((day.year(), day.month())) {
            if months.len() > 3 {
                break;
            }
            keep.insert(day);
        }
    }
    keep
}

fn prune(snapshots: &[Snapshot]) -> Result<(), CommandError> {
    let keep = retention_days(snapshots);
    for snapshot in snapshots {
        if keep.contains(&snapshot.day) {
            continue;
        }
        // Verify ownership and identity again immediately before deleting.
        let (current, _) = inspect_snapshot(&snapshot.path, snapshot.day)?;
        if current.id != snapshot.id {
            return Err(backup_error());
        }
        fs::remove_file(&snapshot.path).map_err(|_| backup_error())?;
    }
    Ok(())
}

/// Produces a portable record set without changing or opening the live database.
/// The command layer supplies only the private application backup directory.
pub(crate) fn latest_export(directory: &Path) -> Result<(Vec<u8>, String), CommandError> {
    let (snapshots, _) = inventory_snapshots(directory)?;
    let latest = snapshots.last().ok_or_else(|| CommandError::new("backup_failed", "No verified automatic recovery snapshot is available. Choose a previously exported portable copy instead."))?;
    let (_, records) = inspect_snapshot(&latest.path, latest.day)?;
    Ok((
        encoded_records(&records)?,
        format!("Automatic recovery {}", latest.day),
    ))
}

#[cfg(test)]
#[path = "backup_tests.rs"]
mod tests;

// Per-database barriers exist only in Rust tests. No command, environment option,
// production delay, or globally paused worker is exposed to the application.
#[cfg(test)]
pub(crate) mod test_gate {
    use super::*;
    use std::sync::{Condvar, OnceLock, Weak};

    thread_local! {
        static INVENTORY_DATABASE: std::cell::RefCell<Option<PathBuf>> = const { std::cell::RefCell::new(None) };
    }

    pub(super) fn during_inventory<T>(database: &Path, operation: impl FnOnce() -> T) -> T {
        struct Reset(Option<PathBuf>);
        impl Drop for Reset {
            fn drop(&mut self) {
                INVENTORY_DATABASE.with(|context| *context.borrow_mut() = self.0.take());
            }
        }
        let previous = INVENTORY_DATABASE.with(|context| context.replace(Some(database.into())));
        let _reset = Reset(previous);
        operation()
    }

    pub(super) fn inventory_reader_pinned() {
        let database = INVENTORY_DATABASE.with(|context| context.borrow().clone());
        if let Some(database) = database {
            pause(&database, Point::Inventory);
        }
    }

    #[derive(Clone, Copy, PartialEq, Eq)]
    pub(crate) enum Point {
        Inventory,
        SnapshotPinned,
    }

    #[derive(Default)]
    struct State {
        reached: bool,
        released: bool,
    }

    #[derive(Default)]
    struct Barrier {
        state: Mutex<State>,
        condition: Condvar,
    }

    struct Registration {
        database: PathBuf,
        point: Point,
        barrier: Weak<Barrier>,
    }

    fn registry() -> &'static Mutex<Vec<Registration>> {
        static REGISTRY: OnceLock<Mutex<Vec<Registration>>> = OnceLock::new();
        REGISTRY.get_or_init(|| Mutex::new(Vec::new()))
    }

    pub(crate) struct Gate(Arc<Barrier>);

    pub(crate) fn install(database: &Path, point: Point) -> Gate {
        let barrier = Arc::new(Barrier::default());
        let mut entries = registry().lock().unwrap();
        entries.retain(|entry| entry.barrier.strong_count() > 0);
        entries.push(Registration {
            database: database.into(),
            point,
            barrier: Arc::downgrade(&barrier),
        });
        Gate(barrier)
    }

    impl Gate {
        pub(crate) fn wait_until_reached(&self, timeout: Duration) -> bool {
            self.0
                .condition
                .wait_timeout_while(self.0.state.lock().unwrap(), timeout, |state| {
                    !state.reached
                })
                .unwrap()
                .0
                .reached
        }

        pub(crate) fn release(&self) {
            self.0.state.lock().unwrap().released = true;
            self.0.condition.notify_all();
        }
    }

    impl Drop for Gate {
        fn drop(&mut self) {
            self.release();
        }
    }

    pub(super) fn pause(database: &Path, point: Point) {
        let barrier = registry()
            .lock()
            .unwrap()
            .iter()
            .find(|entry| entry.database == database && entry.point == point)
            .and_then(|entry| entry.barrier.upgrade());
        let Some(barrier) = barrier else { return };
        let mut state = barrier.state.lock().unwrap();
        if state.reached || state.released {
            return;
        }
        state.reached = true;
        barrier.condition.notify_all();
        while !state.released {
            state = barrier.condition.wait(state).unwrap();
        }
    }
}
