//! User-owned organization and factual stopping notes. No evidence or reward authority.
use crate::domain::*;
use rusqlite::{Connection, params};
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use sha2::{Digest, Sha256};
use std::collections::{HashMap, HashSet};

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ObjectiveStatus {
    Active,
    Paused,
    Archived,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ObjectivePlanning {
    pub objective_id: String,
    pub status: ObjectiveStatus,
    pub priority: u32,
    pub revision: u32,
    pub updated_at: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Checkpoint {
    pub id: String,
    pub command_id: String,
    pub sequence: u32,
    pub objective_id: String,
    pub note: String,
    #[serde(deserialize_with = "crate::domain::required_nullable")]
    pub reference: Option<String>,
    pub provenance: String,
    pub recorded_at: String,
    pub action_id: String,
    pub action_revision: u32,
    pub description: String,
    pub stop_condition: String,
    #[serde(deserialize_with = "crate::domain::required_nullable")]
    pub resumption_note: Option<String>,
    #[serde(deserialize_with = "crate::domain::required_nullable")]
    pub supersedes: Option<String>,
    #[serde(deserialize_with = "crate::domain::required_nullable")]
    pub correction_reason: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct RecordCheckpointInput {
    pub expected_generation: String,
    pub command_id: String,
    pub objective_id: String,
    pub note: String,
    #[serde(deserialize_with = "crate::domain::required_nullable")]
    pub reference: Option<String>,
    pub description: String,
    pub stop_condition: String,
    #[serde(deserialize_with = "crate::domain::required_nullable")]
    pub resumption_note: Option<String>,
    #[serde(deserialize_with = "crate::domain::required_nullable")]
    pub supersedes: Option<String>,
    #[serde(deserialize_with = "crate::domain::required_nullable")]
    pub correction_reason: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct CheckpointCommand {
    pub id: String,
    pub digest: String,
    pub input: RecordCheckpointInput,
    pub checkpoint_id: String,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct DailyState {
    pub objectives: Vec<ObjectivePlanning>,
    pub checkpoints: Vec<Checkpoint>,
    pub commands: Vec<CheckpointCommand>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SetObjectiveStatusInput {
    pub expected_generation: String,
    pub objective_id: String,
    pub status: ObjectiveStatus,
}
#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReorderObjectivesInput {
    pub expected_generation: String,
    pub objective_ids: Vec<String>,
}
#[derive(Clone, Debug, Serialize)]
pub struct CheckpointResult {
    pub workspace: Workspace,
    pub checkpoint: Checkpoint,
}

impl DailyState {
    pub fn record_count(&self) -> usize {
        self.objectives.len() + self.checkpoints.len() + self.commands.len()
    }
    pub fn normalize(&mut self) {
        self.objectives
            .sort_by(|a, b| a.objective_id.cmp(&b.objective_id));
        self.checkpoints.sort_by_key(|c| c.sequence);
        self.commands.sort_by(|a, b| a.id.cmp(&b.id));
    }
    pub fn has_history(&self, objectives: &[Objective]) -> bool {
        let legacy = from_legacy(objectives);
        let priorities: HashMap<_, _> = legacy
            .objectives
            .iter()
            .map(|o| (&o.objective_id, o.priority))
            .collect();
        !self.checkpoints.is_empty()
            || !self.commands.is_empty()
            || self
                .objectives
                .iter()
                .any(|o| o.revision != 1 || o.status != ObjectiveStatus::Active)
            || self.objectives.iter().any(|o| {
                priorities
                    .get(&o.objective_id)
                    .is_none_or(|priority| *priority != o.priority)
            })
    }
}
pub(crate) fn from_legacy(objectives: &[Objective]) -> DailyState {
    let mut ordered: Vec<_> = objectives.iter().collect();
    ordered.sort_by(|a, b| a.id.cmp(&b.id));
    DailyState {
        objectives: ordered
            .into_iter()
            .enumerate()
            .map(|(index, o)| ObjectivePlanning {
                objective_id: o.id.clone(),
                status: ObjectiveStatus::Active,
                priority: index as u32,
                revision: 1,
                updated_at: o.updated_at.clone(),
            })
            .collect(),
        ..DailyState::default()
    }
}
fn db<T>(value: rusqlite::Result<T>) -> Result<T, CommandError> {
    value.map_err(|_| CommandError::storage())
}
fn encode<T: Serialize>(value: &T) -> Result<String, CommandError> {
    serde_json::to_string(value).map_err(|_| CommandError::storage())
}
fn digest(input: &RecordCheckpointInput) -> Result<String, CommandError> {
    Ok(format!("{:x}", Sha256::digest(encode(input)?.as_bytes())))
}
fn rows<T: DeserializeOwned>(c: &Connection, table: &str) -> Result<Vec<T>, CommandError> {
    let mut statement = db(c.prepare(&format!("SELECT payload FROM {table} ORDER BY id")))?;
    db(statement.query_map([], |r| r.get::<_, String>(0)))?
        .map(|v| serde_json::from_str(&db(v)?).map_err(|_| CommandError::storage()))
        .collect()
}
fn insert<T: Serialize>(
    c: &Connection,
    table: &str,
    id: &str,
    value: &T,
) -> Result<(), CommandError> {
    db(c.execute(
        &format!("INSERT INTO {table}(id,payload) VALUES(?1,?2)"),
        params![id, encode(value)?],
    ))?;
    Ok(())
}
pub(crate) fn read(c: &Connection) -> Result<DailyState, CommandError> {
    let mut state = DailyState {
        objectives: rows(c, "objective_planning")?,
        checkpoints: rows(c, "work_checkpoints")?,
        commands: rows(c, "checkpoint_commands")?,
    };
    state.normalize();
    Ok(state)
}
pub(crate) fn sync(
    c: &Connection,
    before: &DailyState,
    after: &DailyState,
) -> Result<(), CommandError> {
    for row in &after.objectives {
        match before
            .objectives
            .iter()
            .find(|v| v.objective_id == row.objective_id)
        {
            Some(old) if old == row => {}
            Some(_) => {
                db(c.execute(
                    "UPDATE objective_planning SET payload=?2 WHERE id=?1",
                    params![row.objective_id, encode(row)?],
                ))?;
            }
            None => insert(c, "objective_planning", &row.objective_id, row)?,
        }
    }
    Ok(())
}
pub(crate) fn clear(c: &Connection) -> Result<(), CommandError> {
    db(c.execute_batch("DELETE FROM checkpoint_commands; DELETE FROM work_checkpoints; DELETE FROM objective_planning;"))?;
    Ok(())
}
pub(crate) fn restore(c: &Connection, state: &DailyState) -> Result<(), CommandError> {
    sync(c, &DailyState::default(), state)?;
    for checkpoint in &state.checkpoints {
        insert(c, "work_checkpoints", &checkpoint.id, checkpoint)?;
    }
    for command in &state.commands {
        insert(c, "checkpoint_commands", &command.id, command)?;
    }
    Ok(())
}
pub(crate) fn set_status(
    workspace: &mut Workspace,
    input: SetObjectiveStatusInput,
) -> Result<bool, CommandError> {
    let row = workspace
        .daily
        .objectives
        .iter_mut()
        .find(|v| v.objective_id == input.objective_id)
        .ok_or_else(|| CommandError::invalid("The objective does not exist."))?;
    if row.status == input.status {
        return Ok(false);
    }
    row.status = input.status;
    row.revision = next_revision(row.revision)?;
    row.updated_at = now();
    if workspace.selected_objective_id.as_deref() == Some(&input.objective_id)
        && input.status != ObjectiveStatus::Active
    {
        workspace.selected_objective_id = workspace
            .daily
            .objectives
            .iter()
            .filter(|v| v.status == ObjectiveStatus::Active)
            .min_by_key(|v| v.priority)
            .map(|v| v.objective_id.clone());
    }
    Ok(true)
}
pub(crate) fn reorder(
    workspace: &mut Workspace,
    input: ReorderObjectivesInput,
) -> Result<bool, CommandError> {
    let mut positions: Vec<_> = workspace
        .daily
        .objectives
        .iter()
        .filter(|o| o.status == ObjectiveStatus::Active)
        .map(|o| o.priority)
        .collect();
    positions.sort();
    let wanted: HashSet<_> = input.objective_ids.iter().collect();
    if input.objective_ids.len() != positions.len()
        || wanted.len() != positions.len()
        || workspace
            .daily
            .objectives
            .iter()
            .filter(|o| o.status == ObjectiveStatus::Active)
            .any(|o| !wanted.contains(&o.objective_id))
    {
        return Err(CommandError::invalid(
            "Provide each active objective exactly once in priority order.",
        ));
    }
    let mut changed = false;
    let timestamp = now();
    for (id, priority) in input.objective_ids.iter().zip(positions) {
        let row = workspace
            .daily
            .objectives
            .iter_mut()
            .find(|o| &o.objective_id == id)
            .ok_or_else(|| CommandError::invalid("The objective does not exist."))?;
        if row.priority != priority {
            row.priority = priority;
            row.revision = next_revision(row.revision)?;
            row.updated_at = timestamp.clone();
            changed = true;
        }
    }
    Ok(changed)
}
fn validate_input(i: &RecordCheckpointInput) -> Result<(), CommandError> {
    valid_id(&i.expected_generation)?;
    valid_id(&i.command_id)?;
    valid_id(&i.objective_id)?;
    text(&i.note, 8000, "Factual checkpoint note")?;
    text(&i.description, 2000, "Next action")?;
    text(&i.stop_condition, 2000, "Stop condition")?;
    if let Some(v) = &i.reference {
        text(v, 2000, "Checkpoint reference")?;
    }
    if let Some(v) = &i.resumption_note {
        text(v, 8000, "Resumption note")?;
    }
    if i.supersedes.is_some() != i.correction_reason.is_some() {
        return Err(CommandError::invalid(
            "A checkpoint correction requires the previous checkpoint and a reason.",
        ));
    }
    if let Some(v) = &i.supersedes {
        valid_id(v)?;
    }
    if let Some(v) = &i.correction_reason {
        text(v, 2000, "Correction reason")?;
    }
    Ok(())
}
pub(crate) fn record(
    c: &Connection,
    input: RecordCheckpointInput,
) -> Result<Checkpoint, CommandError> {
    validate_input(&input)?;
    let mut workspace = crate::storage::read_workspace(c)?;
    let command_digest = digest(&input)?;
    // A lost response must remain recoverable after unrelated writes or restore.
    if let Some(command) = workspace
        .daily
        .commands
        .iter()
        .find(|v| v.id == input.command_id)
    {
        if command.digest != command_digest || command.input != input {
            return Err(CommandError::new(
                "idempotency_conflict",
                "This checkpoint command identity was already used with different input.",
            ));
        }
        return workspace
            .daily
            .checkpoints
            .iter()
            .find(|v| v.id == command.checkpoint_id)
            .cloned()
            .ok_or_else(CommandError::storage);
    }
    if workspace.generation != input.expected_generation {
        return Err(CommandError::conflict());
    }
    if !workspace
        .objectives
        .iter()
        .any(|o| o.id == input.objective_id)
    {
        return Err(CommandError::invalid("The objective does not exist."));
    }
    if let Some(previous) = &input.supersedes
        && (!workspace
            .daily
            .checkpoints
            .iter()
            .any(|p| &p.id == previous && p.objective_id == input.objective_id)
            || workspace
                .daily
                .checkpoints
                .iter()
                .any(|p| p.supersedes.as_ref() == Some(previous)))
    {
        return Err(CommandError::invalid(
            "Correct the latest account in this checkpoint's correction chain.",
        ));
    }
    let before = workspace.clone();
    let timestamp = now();
    crate::service::apply_saved_work(
        &mut workspace,
        &input.objective_id,
        input.description.clone(),
        input.stop_condition.clone(),
        input.resumption_note.clone(),
        &timestamp,
    )?;
    workspace.generation = new_id();
    crate::storage::apply_workspace_changes(c, &before, &workspace)?;
    let action = workspace
        .actions
        .iter()
        .find(|a| a.objective_id == input.objective_id)
        .ok_or_else(CommandError::storage)?;
    let checkpoint = Checkpoint {
        id: new_id(),
        command_id: input.command_id.clone(),
        sequence: next_revision(workspace.daily.checkpoints.len() as u32)?,
        objective_id: input.objective_id.clone(),
        note: input.note.clone(),
        reference: input.reference.clone(),
        provenance: "self_report".into(),
        recorded_at: timestamp,
        action_id: action.id.clone(),
        action_revision: action.revision,
        description: input.description.clone(),
        stop_condition: input.stop_condition.clone(),
        resumption_note: input.resumption_note.clone(),
        supersedes: input.supersedes.clone(),
        correction_reason: input.correction_reason.clone(),
    };
    insert(c, "work_checkpoints", &checkpoint.id, &checkpoint)?;
    let command = CheckpointCommand {
        id: input.command_id.clone(),
        digest: command_digest,
        input,
        checkpoint_id: checkpoint.id.clone(),
    };
    insert(c, "checkpoint_commands", &command.id, &command)?;
    Ok(checkpoint)
}

pub(crate) fn validate(state: &DailyState, records: &RecordSet) -> Result<(), CommandError> {
    let invalid =
        || CommandError::invalid("Daily organization or checkpoint history is inconsistent.");
    let mut objective_ids = HashSet::new();
    let mut priorities = HashSet::new();
    if state.objectives.len() != records.objectives.len() {
        return Err(invalid());
    }
    for row in &state.objectives {
        valid_id(&row.objective_id)?;
        crate::domain::validate_timestamp(&row.updated_at)?;
        if row.revision == 0
            || !records.objectives.iter().any(|o| o.id == row.objective_id)
            || !objective_ids.insert(&row.objective_id)
            || !priorities.insert(row.priority)
            || row.priority as usize >= state.objectives.len()
        {
            return Err(invalid());
        }
    }
    if records.selected_objective_id.as_ref().is_some_and(|id| {
        state
            .objectives
            .iter()
            .any(|o| &o.objective_id == id && o.status != ObjectiveStatus::Active)
    }) {
        return Err(CommandError::invalid(
            "The focused objective must be active.",
        ));
    }
    if state.checkpoints.is_empty() && state.commands.is_empty() {
        return Ok(());
    }
    let mut ids = HashSet::new();
    // Every new identity must also be distinct from all prior-stage identities.
    let value = serde_json::to_value(records).map_err(|_| CommandError::storage())?;
    fn collect(value: &serde_json::Value, ids: &mut HashSet<String>) {
        match value {
            serde_json::Value::Object(map) => {
                for (key, value) in map {
                    if key == "daily" {
                        continue;
                    }
                    if key == "id" {
                        if let Some(id) = value.as_str() {
                            ids.insert(id.into());
                        }
                    } else {
                        collect(value, ids);
                    }
                }
            }
            serde_json::Value::Array(values) => {
                for v in values {
                    collect(v, ids);
                }
            }
            _ => {}
        }
    }
    collect(&value, &mut ids);
    for c in &state.checkpoints {
        valid_id(&c.id)?;
        if !ids.insert(c.id.clone()) {
            return Err(invalid());
        }
    }
    for c in &state.commands {
        valid_id(&c.id)?;
        if !ids.insert(c.id.clone()) {
            return Err(invalid());
        }
    }
    let checkpoints: HashMap<_, _> = state.checkpoints.iter().map(|c| (&c.id, c)).collect();
    let commands: HashMap<_, _> = state.commands.iter().map(|c| (&c.id, c)).collect();
    if checkpoints.len() != commands.len() {
        return Err(invalid());
    }
    let mut sequences = HashSet::new();
    let mut superseded = HashSet::new();
    for checkpoint in &state.checkpoints {
        crate::domain::validate_timestamp(&checkpoint.recorded_at)?;
        let command = commands.get(&checkpoint.command_id).ok_or_else(invalid)?;
        validate_input(&command.input)?;
        let i = &command.input;
        if command.id != i.command_id
            || command.checkpoint_id != checkpoint.id
            || command.digest != digest(i)?
            || checkpoint.provenance != "self_report"
            || checkpoint.sequence == 0
            || checkpoint.sequence as usize > checkpoints.len()
            || !sequences.insert(checkpoint.sequence)
            || checkpoint.objective_id != i.objective_id
            || checkpoint.note != i.note
            || checkpoint.reference != i.reference
            || checkpoint.description != i.description
            || checkpoint.stop_condition != i.stop_condition
            || checkpoint.resumption_note != i.resumption_note
            || checkpoint.supersedes != i.supersedes
            || checkpoint.correction_reason != i.correction_reason
        {
            return Err(invalid());
        }
        let action = records
            .actions
            .iter()
            .find(|a| a.id == checkpoint.action_id && a.objective_id == checkpoint.objective_id)
            .ok_or_else(invalid)?;
        if checkpoint.action_revision == 0
            || checkpoint.action_revision > action.revision
            || (checkpoint.action_revision == action.revision
                && (checkpoint.description != action.description
                    || checkpoint.stop_condition != action.stop_condition))
        {
            return Err(invalid());
        }
        if let Some(id) = &checkpoint.supersedes {
            let previous = checkpoints.get(id).ok_or_else(invalid)?;
            if previous.sequence >= checkpoint.sequence
                || previous.objective_id != checkpoint.objective_id
                || !superseded.insert(id)
            {
                return Err(invalid());
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests;
