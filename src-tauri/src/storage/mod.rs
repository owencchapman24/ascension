use crate::domain::*;
use rusqlite::{Connection, MAIN_DB, OpenFlags, TransactionBehavior, params};
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::Path;
use std::time::Duration;

const RECORDS_MIGRATION: &str = include_str!("../../migrations/001_records.sql");
const STATE_MIGRATION: &str = include_str!("../../migrations/002_workspace_state.sql");

pub(crate) struct Repository {
    connection: Connection,
}

fn storage<T>(result: rusqlite::Result<T>) -> Result<T, CommandError> {
    result.map_err(|_| CommandError::storage())
}

fn ensure_empty(connection: &Connection) -> Result<(), CommandError> {
    let objects: i64 = storage(connection.query_row(
        "SELECT COUNT(*) FROM sqlite_master WHERE name NOT GLOB 'sqlite_*'",
        [],
        |row| row.get(0),
    ))?;
    if objects != 0 {
        return Err(CommandError::new(
            "storage_error",
            "The existing database has an unrecognized schema. It has not been replaced.",
        ));
    }
    Ok(())
}

impl Repository {
    pub(crate) fn open(path: &Path, backup_dir: &Path) -> Result<Self, CommandError> {
        if let Some(parent) = path
            .parent()
            .filter(|parent| !parent.as_os_str().is_empty())
        {
            fs::create_dir_all(parent).map_err(|_| CommandError::storage())?;
        }
        let mut connection = storage(Connection::open(path))?;
        storage(connection.busy_timeout(Duration::from_secs(5)))?;
        // Inspect compatibility before any pragma that changes the existing database.
        let version: u32 =
            storage(connection.pragma_query_value(None, "user_version", |row| row.get(0)))?;
        if version > 2 {
            return Err(CommandError::new(
                "unsupported_schema",
                "This workspace was created by a newer app version. It has not been changed.",
            ));
        }
        if version == 0 {
            ensure_empty(&connection)?;
        }
        let check: String =
            storage(connection.query_row("PRAGMA quick_check", [], |row| row.get(0)))?;
        if check != "ok" {
            return Err(CommandError::storage());
        }
        storage(connection.pragma_update(None, "foreign_keys", "ON"))?;
        let broken: i64 = storage(connection.query_row(
            "SELECT COUNT(*) FROM pragma_foreign_key_check",
            [],
            |row| row.get(0),
        ))?;
        if broken != 0 {
            return Err(CommandError::storage());
        }
        if version < 2 {
            let transaction =
                storage(connection.transaction_with_behavior(TransactionBehavior::Immediate))?;
            let locked_version: u32 =
                storage(transaction.pragma_query_value(None, "user_version", |row| row.get(0)))?;
            if locked_version > 2 {
                return Err(CommandError::new(
                    "unsupported_schema",
                    "This workspace was created by a newer app version. It has not been changed.",
                ));
            }
            if locked_version == 1 {
                // A second read-only connection snapshots committed data while this
                // connection's IMMEDIATE lock excludes other database writers.
                let source = storage(Connection::open_with_flags(
                    path,
                    OpenFlags::SQLITE_OPEN_READ_ONLY,
                ))?;
                migration_backup(&source, backup_dir)?;
            }
            if locked_version < 2 {
                if locked_version == 0 {
                    ensure_empty(&transaction)?;
                    storage(transaction.execute_batch(RECORDS_MIGRATION))?;
                }
                storage(transaction.execute_batch(STATE_MIGRATION))?;
                storage(transaction.execute("INSERT INTO workspace_state(singleton,generation,selected_objective_id) VALUES (1,?1,NULL)", [new_id()]))?;
                storage(transaction.pragma_update(None, "user_version", 2))?;
                // An invalid old record must roll back the schema upgrade as well.
                read_workspace(&transaction)?;
            }
            storage(transaction.commit())?;
        }
        storage(connection.pragma_update(None, "journal_mode", "WAL"))?;
        storage(connection.pragma_update(None, "synchronous", "FULL"))?;
        let repository = Self { connection };
        repository.load()?;
        Ok(repository)
    }

    pub(crate) fn load(&self) -> Result<Workspace, CommandError> {
        // Keep generation, selection and all collections in one consistent read snapshot.
        let transaction = storage(self.connection.unchecked_transaction())?;
        let workspace = read_workspace(&transaction)?;
        storage(transaction.commit())?;
        Ok(workspace)
    }

    pub(crate) fn change<F>(
        &mut self,
        expected_generation: &str,
        change: F,
    ) -> Result<Workspace, CommandError>
    where
        F: FnOnce(&mut Workspace) -> Result<bool, CommandError>,
    {
        valid_id(expected_generation)?;
        let transaction = storage(
            self.connection
                .transaction_with_behavior(TransactionBehavior::Immediate),
        )?;
        let mut workspace = read_workspace(&transaction)?;
        if workspace.generation != expected_generation {
            return Err(CommandError::conflict());
        }
        if change(&mut workspace)? {
            workspace.generation = new_id();
            let mut records = RecordSet::from(&workspace);
            records.normalize();
            validate_records(&records)?;
            workspace = records.into_workspace(workspace.generation);
            write_workspace(&transaction, &workspace)?;
        }
        storage(transaction.commit())?;
        Ok(workspace)
    }

    pub(crate) fn restore(
        &mut self,
        expected_generation: &str,
        records: RecordSet,
        backup_dir: &Path,
    ) -> Result<RestoreResult, CommandError> {
        let transaction = storage(
            self.connection
                .transaction_with_behavior(TransactionBehavior::Immediate),
        )?;
        let current = read_workspace(&transaction)?;
        if current.generation != expected_generation {
            return Err(CommandError::conflict());
        }
        if RecordSet::from(&current) == records {
            storage(transaction.commit())?;
            return Ok(RestoreResult {
                workspace: current,
                backup_created: false,
                unchanged: true,
            });
        }
        // The write lock binds the backup to precisely the state this restore replaces.
        write_restore_backup(&current, backup_dir)?;
        let workspace = records.into_workspace(new_id());
        write_workspace(&transaction, &workspace)?;
        storage(transaction.commit())?;
        Ok(RestoreResult {
            workspace,
            backup_created: true,
            unchanged: false,
        })
    }
}

fn migration_backup(connection: &Connection, backup_dir: &Path) -> Result<(), CommandError> {
    fs::create_dir_all(backup_dir).map_err(|_| CommandError::backup())?;
    let path = backup_dir.join(format!("before-migration-{}.sqlite3", new_id()));
    let file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&path)
        .map_err(|_| CommandError::backup())?;
    drop(file);
    connection
        .backup(MAIN_DB, &path, None)
        .map_err(|_| CommandError::backup())?;
    let backup = Connection::open_with_flags(&path, OpenFlags::SQLITE_OPEN_READ_WRITE)
        .map_err(|_| CommandError::backup())?;
    // A backup is a standalone recovery file, even when the source uses WAL.
    backup
        .pragma_update(None, "journal_mode", "DELETE")
        .map_err(|_| CommandError::backup())?;
    let check: String = backup
        .query_row("PRAGMA quick_check", [], |row| row.get(0))
        .map_err(|_| CommandError::backup())?;
    let version: u32 = backup
        .pragma_query_value(None, "user_version", |row| row.get(0))
        .map_err(|_| CommandError::backup())?;
    if check != "ok" || version != 1 {
        return Err(CommandError::backup());
    }
    drop(backup);
    OpenOptions::new()
        .write(true)
        .open(&path)
        .and_then(|file| file.sync_all())
        .map_err(|_| CommandError::backup())
}

fn write_restore_backup(workspace: &Workspace, backup_dir: &Path) -> Result<(), CommandError> {
    let bytes = encoded_records(&RecordSet::from(workspace)).map_err(|_| CommandError::backup())?;
    fs::create_dir_all(backup_dir).map_err(|_| CommandError::backup())?;
    let path = backup_dir.join(format!("before-restore-{}.json", new_id()));
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .map_err(|_| CommandError::backup())?;
    file.write_all(&bytes)
        .and_then(|_| file.flush())
        .and_then(|_| file.sync_all())
        .map_err(|_| CommandError::backup())
}

fn read_workspace(connection: &Connection) -> Result<Workspace, CommandError> {
    let (generation, selected_objective_id): (String, Option<String>) =
        storage(connection.query_row(
            "SELECT generation,selected_objective_id FROM workspace_state WHERE singleton=1",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        ))?;
    let projects = {
        let mut statement =
            storage(connection.prepare(
                "SELECT id,title,revision,created_at,updated_at FROM projects ORDER BY id",
            ))?;
        let rows = storage(statement.query_map([], |row| {
            Ok(Project {
                id: row.get(0)?,
                title: row.get(1)?,
                revision: row.get(2)?,
                created_at: row.get(3)?,
                updated_at: row.get(4)?,
            })
        }))?;
        storage(rows.collect::<rusqlite::Result<Vec<_>>>())?
    };
    let objectives = {
        let mut statement = storage(connection.prepare("SELECT id,project_id,title,criterion,revision,created_at,updated_at FROM objectives ORDER BY id"))?;
        let rows = storage(statement.query_map([], |row| {
            Ok(Objective {
                id: row.get(0)?,
                project_id: row.get(1)?,
                title: row.get(2)?,
                criterion: row.get(3)?,
                revision: row.get(4)?,
                created_at: row.get(5)?,
                updated_at: row.get(6)?,
            })
        }))?;
        storage(rows.collect::<rusqlite::Result<Vec<_>>>())?
    };
    let actions = {
        let mut statement = storage(connection.prepare("SELECT id,objective_id,description,stop_condition,revision,created_at,updated_at FROM actions ORDER BY id"))?;
        let rows = storage(statement.query_map([], |row| {
            Ok(NextAction {
                id: row.get(0)?,
                objective_id: row.get(1)?,
                description: row.get(2)?,
                stop_condition: row.get(3)?,
                revision: row.get(4)?,
                created_at: row.get(5)?,
                updated_at: row.get(6)?,
            })
        }))?;
        storage(rows.collect::<rusqlite::Result<Vec<_>>>())?
    };
    let notes = {
        let mut statement = storage(connection.prepare(
            "SELECT id,objective_id,text,revision,created_at,updated_at FROM notes ORDER BY id",
        ))?;
        let rows = storage(statement.query_map([], |row| {
            Ok(ResumptionNote {
                id: row.get(0)?,
                objective_id: row.get(1)?,
                text: row.get(2)?,
                revision: row.get(3)?,
                created_at: row.get(4)?,
                updated_at: row.get(5)?,
            })
        }))?;
        storage(rows.collect::<rusqlite::Result<Vec<_>>>())?
    };
    let workspace = Workspace {
        generation,
        selected_objective_id,
        projects,
        objectives,
        actions,
        notes,
    };
    valid_id(&workspace.generation).map_err(|_| CommandError::storage())?;
    validate_records(&RecordSet::from(&workspace)).map_err(|_| CommandError::storage())?;
    Ok(workspace)
}

fn write_workspace(connection: &Connection, workspace: &Workspace) -> Result<(), CommandError> {
    storage(connection.execute_batch("UPDATE workspace_state SET selected_objective_id=NULL; DELETE FROM notes; DELETE FROM actions; DELETE FROM objectives; DELETE FROM projects;"))?;
    {
        let mut statement = storage(connection.prepare(
            "INSERT INTO projects(id,title,revision,created_at,updated_at) VALUES (?1,?2,?3,?4,?5)",
        ))?;
        for project in &workspace.projects {
            storage(statement.execute(params![
                project.id,
                project.title,
                project.revision,
                project.created_at,
                project.updated_at
            ]))?;
        }
    }
    {
        let mut statement = storage(connection.prepare("INSERT INTO objectives(id,project_id,title,criterion,revision,created_at,updated_at) VALUES (?1,?2,?3,?4,?5,?6,?7)"))?;
        for objective in &workspace.objectives {
            storage(statement.execute(params![
                objective.id,
                objective.project_id,
                objective.title,
                objective.criterion,
                objective.revision,
                objective.created_at,
                objective.updated_at
            ]))?;
        }
    }
    {
        let mut statement = storage(connection.prepare("INSERT INTO actions(id,objective_id,description,stop_condition,revision,created_at,updated_at) VALUES (?1,?2,?3,?4,?5,?6,?7)"))?;
        for action in &workspace.actions {
            storage(statement.execute(params![
                action.id,
                action.objective_id,
                action.description,
                action.stop_condition,
                action.revision,
                action.created_at,
                action.updated_at
            ]))?;
        }
    }
    {
        let mut statement = storage(connection.prepare("INSERT INTO notes(id,objective_id,text,revision,created_at,updated_at) VALUES (?1,?2,?3,?4,?5,?6)"))?;
        for note in &workspace.notes {
            storage(statement.execute(params![
                note.id,
                note.objective_id,
                note.text,
                note.revision,
                note.created_at,
                note.updated_at
            ]))?;
        }
    }
    storage(connection.execute(
        "UPDATE workspace_state SET generation=?1,selected_objective_id=?2 WHERE singleton=1",
        params![workspace.generation, workspace.selected_objective_id],
    ))?;
    Ok(())
}
