use crate::domain::*;
use crate::storage::Repository;
use std::path::{Path, PathBuf};

struct PendingImport {
    token: String,
    generation: String,
    records: RecordSet,
}

pub struct WorkspaceService {
    repository: Repository,
    backup_dir: PathBuf,
    pending_import: Option<PendingImport>,
}

impl WorkspaceService {
    pub fn open(db_path: &Path, backup_dir: &Path) -> Result<Self, CommandError> {
        Ok(Self {
            repository: Repository::open(db_path, backup_dir)?,
            backup_dir: backup_dir.into(),
            pending_import: None,
        })
    }

    pub fn load_workspace(&self) -> Result<Workspace, CommandError> {
        self.repository.load()
    }

    pub fn create_project(&mut self, input: CreateProjectInput) -> Result<Workspace, CommandError> {
        text(&input.title, 120, "Project title")?;
        self.repository
            .change(&input.expected_generation, |workspace| {
                let timestamp = now();
                workspace.projects.push(Project {
                    id: new_id(),
                    revision: 1,
                    created_at: timestamp.clone(),
                    updated_at: timestamp,
                    title: input.title,
                });
                Ok(true)
            })
    }

    pub fn update_project(&mut self, input: UpdateProjectInput) -> Result<Workspace, CommandError> {
        valid_id(&input.id)?;
        text(&input.title, 120, "Project title")?;
        self.repository
            .change(&input.expected_generation, |workspace| {
                let project = workspace
                    .projects
                    .iter_mut()
                    .find(|project| project.id == input.id)
                    .ok_or_else(|| CommandError::invalid("The project does not exist."))?;
                if project.title == input.title {
                    return Ok(false);
                }
                project.revision = next_revision(project.revision)?;
                project.title = input.title;
                project.updated_at = now();
                Ok(true)
            })
    }

    pub fn create_objective(
        &mut self,
        input: CreateObjectiveInput,
    ) -> Result<Workspace, CommandError> {
        valid_id(&input.project_id)?;
        text(&input.title, 500, "Objective title")?;
        text(&input.criterion, 2_000, "Criterion")?;
        self.repository
            .change(&input.expected_generation, |workspace| {
                if !workspace
                    .projects
                    .iter()
                    .any(|project| project.id == input.project_id)
                {
                    return Err(CommandError::invalid("The project does not exist."));
                }
                let timestamp = now();
                let id = new_id();
                workspace.selected_objective_id = Some(id.clone());
                workspace.objectives.push(Objective {
                    id,
                    revision: 1,
                    created_at: timestamp.clone(),
                    updated_at: timestamp,
                    project_id: input.project_id,
                    title: input.title,
                    criterion: input.criterion,
                });
                Ok(true)
            })
    }

    pub fn update_objective(
        &mut self,
        input: UpdateObjectiveInput,
    ) -> Result<Workspace, CommandError> {
        valid_id(&input.id)?;
        text(&input.title, 500, "Objective title")?;
        text(&input.criterion, 2_000, "Criterion")?;
        self.repository
            .change(&input.expected_generation, |workspace| {
                let objective = workspace
                    .objectives
                    .iter_mut()
                    .find(|objective| objective.id == input.id)
                    .ok_or_else(|| CommandError::invalid("The objective does not exist."))?;
                if objective.title == input.title && objective.criterion == input.criterion {
                    return Ok(false);
                }
                objective.revision = next_revision(objective.revision)?;
                objective.title = input.title;
                objective.criterion = input.criterion;
                objective.updated_at = now();
                Ok(true)
            })
    }

    pub fn select_objective(
        &mut self,
        input: SelectObjectiveInput,
    ) -> Result<Workspace, CommandError> {
        if let Some(id) = &input.id {
            valid_id(id)?;
        }
        self.repository
            .change(&input.expected_generation, |workspace| {
                if input.id.as_ref().is_some_and(|id| {
                    !workspace
                        .objectives
                        .iter()
                        .any(|objective| &objective.id == id)
                }) {
                    return Err(CommandError::invalid(
                        "The selected objective does not exist.",
                    ));
                }
                if workspace.selected_objective_id == input.id {
                    return Ok(false);
                }
                workspace.selected_objective_id = input.id;
                Ok(true)
            })
    }

    pub fn save_work(&mut self, input: SaveWorkInput) -> Result<Workspace, CommandError> {
        valid_id(&input.objective_id)?;
        text(&input.description, 2_000, "Next action")?;
        text(&input.stop_condition, 2_000, "Stop condition")?;
        let note_text = input.note.filter(|note| !note.trim().is_empty());
        if let Some(note) = &note_text {
            text(note, 8_000, "Resumption note")?;
        }
        self.repository
            .change(&input.expected_generation, |workspace| {
                if !workspace
                    .objectives
                    .iter()
                    .any(|objective| objective.id == input.objective_id)
                {
                    return Err(CommandError::invalid("The objective does not exist."));
                }
                let mut changed = false;
                let timestamp = now();
                if let Some(action) = workspace
                    .actions
                    .iter_mut()
                    .find(|action| action.objective_id == input.objective_id)
                {
                    if action.description != input.description
                        || action.stop_condition != input.stop_condition
                    {
                        action.revision = next_revision(action.revision)?;
                        action.description = input.description;
                        action.stop_condition = input.stop_condition;
                        action.updated_at = timestamp.clone();
                        changed = true;
                    }
                } else {
                    workspace.actions.push(NextAction {
                        id: new_id(),
                        revision: 1,
                        created_at: timestamp.clone(),
                        updated_at: timestamp.clone(),
                        objective_id: input.objective_id.clone(),
                        description: input.description,
                        stop_condition: input.stop_condition,
                    });
                    changed = true;
                }
                if let Some(note_text) = note_text {
                    if let Some(note) = workspace
                        .notes
                        .iter_mut()
                        .find(|note| note.objective_id == input.objective_id)
                    {
                        if note.text != note_text {
                            note.revision = next_revision(note.revision)?;
                            note.text = note_text;
                            note.updated_at = timestamp;
                            changed = true;
                        }
                    } else {
                        workspace.notes.push(ResumptionNote {
                            id: new_id(),
                            revision: 1,
                            created_at: timestamp.clone(),
                            updated_at: timestamp,
                            objective_id: input.objective_id,
                            text: note_text,
                        });
                        changed = true;
                    }
                } else {
                    let before = workspace.notes.len();
                    workspace
                        .notes
                        .retain(|note| note.objective_id != input.objective_id);
                    changed |= before != workspace.notes.len();
                }
                Ok(changed)
            })
    }

    pub fn export_bytes(&self) -> Result<Vec<u8>, CommandError> {
        encoded_records(&RecordSet::from(&self.repository.load()?))
    }

    pub fn discard_preview(&mut self) {
        self.pending_import = None;
    }

    pub fn preview_import(
        &mut self,
        bytes: &[u8],
        file_name: String,
    ) -> Result<ImportPreview, CommandError> {
        // Failed replacement previews must never leave an earlier file confirmable.
        self.discard_preview();
        let records = parse_import(bytes)?;
        let workspace = self.repository.load()?;
        let token = new_id();
        let incoming_counts = RecordCounts {
            projects: records.projects.len(),
            objectives: records.objectives.len(),
            actions: records.actions.len(),
            notes: records.notes.len(),
        };
        self.pending_import = Some(PendingImport {
            token: token.clone(),
            generation: workspace.generation.clone(),
            records,
        });
        Ok(ImportPreview {
            token,
            generation: workspace.generation.clone(),
            file_name,
            current_counts: RecordCounts::from(&workspace),
            incoming_counts,
        })
    }

    pub fn restore_import(&mut self, input: RestoreInput) -> Result<RestoreResult, CommandError> {
        valid_id(&input.token)?;
        valid_id(&input.expected_generation)?;
        let pending = self.pending_import.as_ref().ok_or_else(|| {
            CommandError::invalid("Choose and preview an import file before restoring it.")
        })?;
        if pending.token != input.token || pending.generation != input.expected_generation {
            return Err(CommandError::conflict());
        }
        let result = self.repository.restore(
            &input.expected_generation,
            pending.records.clone(),
            &self.backup_dir,
        )?;
        self.pending_import = None;
        Ok(result)
    }
}
