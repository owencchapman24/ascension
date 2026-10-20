use crate::progress::ProgressState;
use chrono::{DateTime, SecondsFormat, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use uuid::Uuid;

pub const MAX_IMPORT_BYTES: usize = 10 * 1024 * 1024;
pub const MAX_RECORDS: usize = 10_000;

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct CommandError {
    pub code: String,
    pub message: String,
}

impl CommandError {
    pub fn new(code: &str, message: &str) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
        }
    }
    pub fn invalid(message: &str) -> Self {
        Self::new("invalid_input", message)
    }
    pub fn storage() -> Self {
        Self::new(
            "storage_error",
            "The local workspace could not be read or saved. Your existing records were not replaced.",
        )
    }
    pub fn conflict() -> Self {
        Self::new(
            "conflict",
            "The saved workspace changed. Reload it and review your input before trying again.",
        )
    }
    pub fn backup() -> Self {
        Self::new(
            "backup_failed",
            "A recovery backup could not be saved. Your existing records were not replaced.",
        )
    }
}

impl std::fmt::Display for CommandError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}
impl std::error::Error for CommandError {}

macro_rules! record {
    ($name:ident { $($field:ident : $ty:ty),* $(,)? }) => {
        #[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
        #[serde(deny_unknown_fields)]
        pub struct $name {
            pub id: String,
            pub revision: u32,
            pub created_at: String,
            pub updated_at: String,
            $(pub $field: $ty,)*
        }
    };
}
record!(Project { title: String });
record!(Objective {
    project_id: String,
    title: String,
    criterion: String
});
record!(NextAction {
    objective_id: String,
    description: String,
    stop_condition: String
});
record!(ResumptionNote {
    objective_id: String,
    text: String
});

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Workspace {
    pub generation: String,
    pub selected_objective_id: Option<String>,
    pub projects: Vec<Project>,
    pub objectives: Vec<Objective>,
    pub actions: Vec<NextAction>,
    pub notes: Vec<ResumptionNote>,
    pub progress: ProgressState,
    #[serde(serialize_with = "crate::study::serialize_view")]
    pub study: crate::study::StudyState,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct RecordSet {
    pub format: String,
    pub schema_version: u32,
    // A missing selection is malformed, even though an explicitly null one is valid.
    #[serde(deserialize_with = "required_nullable")]
    pub selected_objective_id: Option<String>,
    pub projects: Vec<Project>,
    pub objectives: Vec<Objective>,
    pub actions: Vec<NextAction>,
    pub notes: Vec<ResumptionNote>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub progress: Option<ProgressState>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub study: Option<crate::study::StudyState>,
}

fn required_nullable<'de, D: serde::Deserializer<'de>>(
    deserializer: D,
) -> Result<Option<String>, D::Error> {
    Option::<String>::deserialize(deserializer)
}

impl From<&Workspace> for RecordSet {
    fn from(workspace: &Workspace) -> Self {
        Self {
            format: "ascension-records".into(),
            schema_version: 3,
            selected_objective_id: workspace.selected_objective_id.clone(),
            projects: workspace.projects.clone(),
            objectives: workspace.objectives.clone(),
            actions: workspace.actions.clone(),
            notes: workspace.notes.clone(),
            progress: Some(workspace.progress.clone()),
            study: Some(workspace.study.clone()),
        }
    }
}

impl RecordSet {
    pub fn upgrade_legacy(&mut self) {
        if self.schema_version == 1 && self.progress.is_none() {
            self.progress = Some(crate::progress::from_legacy(&self.objectives));
            self.schema_version = 2;
        }
        if self.schema_version == 2 && self.study.is_none() {
            self.study = Some(crate::study::StudyState::default());
            self.schema_version = 3;
        }
    }
    pub fn normalize(&mut self) {
        self.projects.sort_by(|a, b| a.id.cmp(&b.id));
        self.objectives.sort_by(|a, b| a.id.cmp(&b.id));
        self.actions.sort_by(|a, b| a.id.cmp(&b.id));
        self.notes.sort_by(|a, b| a.id.cmp(&b.id));
        if let Some(study) = &mut self.study {
            study.normalize();
        }
        if let Some(progress) = &mut self.progress {
            progress.normalize();
        }
    }
    pub fn into_workspace(self, generation: String) -> Workspace {
        Workspace {
            generation,
            selected_objective_id: self.selected_objective_id,
            projects: self.projects,
            objectives: self.objectives,
            actions: self.actions,
            notes: self.notes,
            progress: self.progress.unwrap_or_default(),
            study: self.study.unwrap_or_default(),
        }
    }
}

macro_rules! input {
    ($name:ident { $($field:ident : $ty:ty),* $(,)? }) => {
        #[derive(Clone, Debug, Deserialize)]
        #[serde(deny_unknown_fields)]
        pub struct $name { pub expected_generation: String, $(pub $field: $ty,)* }
    };
}
input!(CreateProjectInput { title: String });
input!(UpdateProjectInput {
    id: String,
    title: String
});
input!(CreateObjectiveInput {
    project_id: String,
    title: String,
    criterion: String
});
input!(UpdateObjectiveInput {
    id: String,
    title: String,
    criterion: String
});

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SelectObjectiveInput {
    pub expected_generation: String,
    #[serde(deserialize_with = "required_nullable")]
    pub id: Option<String>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SaveWorkInput {
    pub expected_generation: String,
    pub objective_id: String,
    pub description: String,
    pub stop_condition: String,
    #[serde(deserialize_with = "required_nullable")]
    pub note: Option<String>,
}
input!(RestoreInput { token: String });

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct RecordCounts {
    pub projects: usize,
    pub objectives: usize,
    pub actions: usize,
    pub notes: usize,
    pub criteria: usize,
    pub evidence: usize,
    pub snapshots: usize,
    pub acceptances: usize,
    pub corrections: usize,
    pub grants: usize,
    pub ledger: usize,
    pub commands: usize,
    pub wallet_bc: u32,
    pub opportunities: usize,
    pub assignments: usize,
    pub outcome_events: usize,
    pub study_manifests: usize,
    pub schedule_epochs: usize,
    pub exposures: usize,
    pub burdens: usize,
    pub reflections: usize,
}
impl From<&Workspace> for RecordCounts {
    fn from(workspace: &Workspace) -> Self {
        Self {
            projects: workspace.projects.len(),
            objectives: workspace.objectives.len(),
            actions: workspace.actions.len(),
            notes: workspace.notes.len(),
            criteria: workspace.progress.criteria.len(),
            evidence: workspace.progress.evidence.len(),
            snapshots: workspace.progress.snapshots.len(),
            acceptances: workspace.progress.acceptances.len(),
            corrections: workspace.progress.corrections.len(),
            grants: workspace.progress.grants.len(),
            ledger: workspace.progress.ledger.len(),
            commands: workspace.progress.commands.len(),
            wallet_bc: workspace.progress.wallet_bc,
            opportunities: workspace.study.opportunities.len(),
            assignments: workspace.study.assignments.len(),
            outcome_events: workspace.study.outcomes.len(),
            study_manifests: workspace.study.manifests.len(),
            schedule_epochs: workspace.study.epochs.len(),
            exposures: workspace.study.exposures.len(),
            burdens: workspace.study.burdens.len(),
            reflections: workspace.study.reflections.len(),
        }
    }
}
#[derive(Clone, Debug, Serialize)]
pub struct ImportPreview {
    pub token: String,
    pub generation: String,
    pub file_name: String,
    pub current_counts: RecordCounts,
    pub incoming_counts: RecordCounts,
    pub schema_version: u32,
    pub warning: String,
}
#[derive(Clone, Debug, Serialize)]
pub struct RestoreResult {
    pub workspace: Workspace,
    pub backup_created: bool,
    pub unchanged: bool,
}

pub(crate) fn new_id() -> String {
    Uuid::new_v4().to_string()
}
pub(crate) fn now() -> String {
    Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true)
}
pub(crate) fn next_revision(revision: u32) -> Result<u32, CommandError> {
    revision
        .checked_add(1)
        .ok_or_else(|| CommandError::invalid("This record has reached its revision limit."))
}
pub(crate) fn valid_id(id: &str) -> Result<(), CommandError> {
    if Uuid::parse_str(id).is_ok_and(|value| value.to_string() == id) {
        Ok(())
    } else {
        Err(CommandError::invalid(
            "An identifier is not a canonical UUID.",
        ))
    }
}
pub(crate) fn text(value: &str, max: usize, field: &str) -> Result<(), CommandError> {
    if value.trim().is_empty() || value.contains('\0') || value.chars().count() > max {
        Err(CommandError::invalid(&format!(
            "{field} is required, must not contain a null character, and must be at most {max} characters."
        )))
    } else {
        Ok(())
    }
}
fn metadata(
    id: &str,
    revision: u32,
    created_at: &str,
    updated_at: &str,
    ids: &mut HashSet<String>,
) -> Result<(), CommandError> {
    valid_id(id)?;
    if !ids.insert(id.to_owned()) {
        return Err(CommandError::invalid("Record IDs must be unique."));
    }
    if revision == 0 {
        return Err(CommandError::invalid(
            "Record revisions must be positive integers.",
        ));
    }
    for timestamp in [created_at, updated_at] {
        let parsed = DateTime::parse_from_rfc3339(timestamp).map_err(|_| {
            CommandError::invalid(
                "Record timestamps must be UTC timestamps with millisecond precision.",
            )
        })?;
        if parsed
            .with_timezone(&Utc)
            .to_rfc3339_opts(SecondsFormat::Millis, true)
            != timestamp
        {
            return Err(CommandError::invalid(
                "Record timestamps must be UTC timestamps with millisecond precision.",
            ));
        }
    }
    Ok(())
}

pub(crate) fn validate_records(records: &RecordSet) -> Result<(), CommandError> {
    if ![1, 2, 3].contains(&records.schema_version) {
        return Err(CommandError::new(
            "unsupported_schema",
            "This record schema version is not supported. Your saved workspace is unchanged.",
        ));
    }
    if records.format != "ascension-records" {
        return Err(CommandError::invalid(
            "This is not an Ascension record set.",
        ));
    }
    if (records.schema_version == 1 && records.progress.is_some())
        || (records.schema_version >= 2 && records.progress.is_none())
        || (records.schema_version < 3 && records.study.is_some())
        || (records.schema_version == 3 && records.study.is_none())
    {
        return Err(CommandError::invalid(
            "The record schema and complete progress state do not match.",
        ));
    }
    if records.projects.len()
        + records.objectives.len()
        + records.actions.len()
        + records.notes.len()
        + records
            .progress
            .as_ref()
            .map_or(0, |state| state.record_count())
        + records
            .study
            .as_ref()
            .map_or(0, |state| state.record_count())
        > MAX_RECORDS
    {
        return Err(CommandError::invalid(
            "A workspace may contain at most 10,000 records.",
        ));
    }
    let mut ids = HashSet::new();
    let mut projects = HashSet::new();
    for project in &records.projects {
        metadata(
            &project.id,
            project.revision,
            &project.created_at,
            &project.updated_at,
            &mut ids,
        )?;
        text(&project.title, 120, "Project title")?;
        projects.insert(project.id.as_str());
    }
    let mut objectives = HashSet::new();
    for objective in &records.objectives {
        metadata(
            &objective.id,
            objective.revision,
            &objective.created_at,
            &objective.updated_at,
            &mut ids,
        )?;
        if !projects.contains(objective.project_id.as_str()) {
            return Err(CommandError::invalid(
                "An objective references a missing project.",
            ));
        }
        text(&objective.title, 500, "Objective title")?;
        text(&objective.criterion, 2_000, "Criterion")?;
        objectives.insert(objective.id.as_str());
    }
    let mut action_parents = HashSet::new();
    for action in &records.actions {
        metadata(
            &action.id,
            action.revision,
            &action.created_at,
            &action.updated_at,
            &mut ids,
        )?;
        if !objectives.contains(action.objective_id.as_str()) {
            return Err(CommandError::invalid(
                "An action references a missing objective.",
            ));
        }
        if !action_parents.insert(action.objective_id.as_str()) {
            return Err(CommandError::invalid(
                "An objective may have only one action.",
            ));
        }
        text(&action.description, 2_000, "Next action")?;
        text(&action.stop_condition, 2_000, "Stop condition")?;
    }
    let mut note_parents = HashSet::new();
    for note in &records.notes {
        metadata(
            &note.id,
            note.revision,
            &note.created_at,
            &note.updated_at,
            &mut ids,
        )?;
        if !objectives.contains(note.objective_id.as_str()) {
            return Err(CommandError::invalid(
                "A note references a missing objective.",
            ));
        }
        if !note_parents.insert(note.objective_id.as_str()) {
            return Err(CommandError::invalid(
                "An objective may have only one note.",
            ));
        }
        text(&note.text, 8_000, "Resumption note")?;
    }
    if records
        .selected_objective_id
        .as_ref()
        .is_some_and(|id| !objectives.contains(id.as_str()))
    {
        return Err(CommandError::invalid(
            "The selected objective does not exist.",
        ));
    }
    if let Some(progress) = &records.progress {
        macro_rules! distinct_progress_ids {
            ($($field:ident),*) => { $(for record in &progress.$field {
                if ids.contains(&record.id) { return Err(CommandError::invalid("Record IDs must be unique across work and progress records.")); }
            })* };
        }
        distinct_progress_ids!(
            slots,
            criteria,
            evidence,
            validators,
            snapshots,
            acceptances,
            corrections,
            grants,
            ledger,
            audit,
            commands
        );
        crate::progress::validate(progress, &records.objectives)?;
    }
    if let Some(study) = &records.study {
        crate::study::validate(study, records)?;
    }
    if encoded_records(records)?.len() > MAX_IMPORT_BYTES {
        return Err(CommandError::invalid(
            "The saved record set must fit within the 10 MiB export limit.",
        ));
    }
    Ok(())
}

pub(crate) fn encoded_records(records: &RecordSet) -> Result<Vec<u8>, CommandError> {
    serde_json::to_vec_pretty(records).map_err(|_| CommandError::storage())
}

pub(crate) fn parse_import(bytes: &[u8]) -> Result<(RecordSet, u32), CommandError> {
    if bytes.len() > MAX_IMPORT_BYTES {
        return Err(CommandError::new(
            "invalid_import",
            "The import file exceeds the 10 MiB limit.",
        ));
    }
    let mut records: RecordSet = serde_json::from_slice(bytes).map_err(|_| {
        CommandError::new(
            "invalid_import",
            "The file is not a complete, valid Ascension JSON record set.",
        )
    })?;
    validate_records(&records).map_err(|error| {
        if error.code == "unsupported_schema" {
            error
        } else {
            CommandError::new("invalid_import", &error.message)
        }
    })?;
    let source_schema = records.schema_version;
    records.upgrade_legacy();
    records.normalize();
    validate_records(&records)
        .map_err(|error| CommandError::new("invalid_import", &error.message))?;
    Ok((records, source_schema))
}
