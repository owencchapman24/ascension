use crate::domain::*;
use crate::progress::{self, *};
use crate::storage::Repository;
use crate::study::{self, *};
use crate::world::{
    self, WorldCommandInput, WorldCommandResult, WorldPreview, WorldPreviewInput, WorldView,
};
use serde::Serialize;
use std::path::{Path, PathBuf};

struct PendingImport {
    token: String,
    generation: String,
    records: RecordSet,
    source_schema: u32,
}

#[derive(Clone, Debug, Serialize)]
pub struct ReviewResult {
    pub workspace: Workspace,
    pub snapshot: ReviewSnapshot,
}
#[derive(Clone, Debug, Serialize)]
pub struct AcceptanceResult {
    pub workspace: Workspace,
    pub result: SettlementResult,
}

#[derive(Clone, Debug, Serialize)]
pub struct WorldResult {
    pub workspace: Workspace,
    pub result: WorldCommandResult,
}

pub struct WorkspaceService {
    repository: Repository,
    backup_dir: PathBuf,
    pending_import: Option<PendingImport>,
}

impl WorkspaceService {
    pub fn load_world(&self) -> Result<WorldView, CommandError> {
        self.repository.world_view()
    }

    pub fn preview_world(&self, input: WorldPreviewInput) -> Result<WorldPreview, CommandError> {
        self.repository.world_preview(input)
    }

    pub fn world_command(&mut self, input: WorldCommandInput) -> Result<WorldResult, CommandError> {
        self.repository
            .progress_change(|connection| world::execute(connection, input))
            .map(|(workspace, result)| WorldResult { workspace, result })
    }

    pub fn set_construction_enabled(
        &mut self,
        input: SetConstructionEnabledInput,
    ) -> Result<Workspace, CommandError> {
        self.repository
            .progress_change(|c| study::set_construction_enabled(c, input))
            .map(|(w, ())| w)
    }
    pub fn create_schedule_epoch(
        &mut self,
        input: CreateScheduleEpochInput,
    ) -> Result<Workspace, CommandError> {
        self.repository
            .progress_change(|c| study::create_epoch(c, input))
            .map(|(w, ())| w)
    }
    pub fn configure_study(
        &mut self,
        input: ConfigureStudyInput,
    ) -> Result<Workspace, CommandError> {
        self.repository
            .progress_change(|c| study::configure(c, input))
            .map(|(w, ())| w)
    }
    pub fn transition_study(
        &mut self,
        input: TransitionStudyInput,
    ) -> Result<Workspace, CommandError> {
        self.repository
            .progress_change(|c| study::transition(c, input))
            .map(|(w, ())| w)
    }
    pub fn plan_opportunity(
        &mut self,
        input: PlanOpportunityInput,
    ) -> Result<Workspace, CommandError> {
        self.repository
            .progress_change(|c| study::plan(c, input))
            .map(|(w, ())| w)
    }
    pub fn record_opportunity_outcome(
        &mut self,
        input: RecordOpportunityOutcomeInput,
    ) -> Result<Workspace, CommandError> {
        self.repository
            .progress_change(|c| study::outcome(c, input))
            .map(|(w, ())| w)
    }
    pub fn record_study_exposure(
        &mut self,
        input: RecordStudyExposureInput,
    ) -> Result<Workspace, CommandError> {
        self.repository
            .progress_change(|c| study::exposure(c, input))
            .map(|(w, ())| w)
    }
    pub fn record_study_burden(
        &mut self,
        input: RecordStudyBurdenInput,
    ) -> Result<Workspace, CommandError> {
        self.repository
            .progress_change(|c| study::burden(c, input))
            .map(|(w, ())| w)
    }
    pub fn record_study_reflection(
        &mut self,
        input: RecordStudyReflectionInput,
    ) -> Result<Workspace, CommandError> {
        self.repository
            .progress_change(|c| study::reflection(c, input))
            .map(|(w, ())| w)
    }

    pub fn save_criterion(&mut self, input: SaveCriterionInput) -> Result<Workspace, CommandError> {
        self.repository
            .progress_change(|connection| progress::save_criterion(connection, input))
            .map(|(workspace, ())| workspace)
    }
    pub fn lock_criterion(&mut self, input: LockCriterionInput) -> Result<Workspace, CommandError> {
        self.repository
            .progress_change(|connection| progress::lock_criterion(connection, input))
            .map(|(workspace, ())| workspace)
    }
    pub fn record_evidence(
        &mut self,
        input: RecordEvidenceInput,
    ) -> Result<Workspace, CommandError> {
        self.repository
            .progress_change(|connection| progress::record_evidence(connection, input))
            .map(|(workspace, ())| workspace)
    }
    pub fn review_progress(
        &mut self,
        input: ReviewProgressInput,
    ) -> Result<ReviewResult, CommandError> {
        self.repository
            .progress_change(|connection| progress::review(connection, input))
            .map(|(workspace, snapshot)| ReviewResult {
                workspace,
                snapshot,
            })
    }
    pub fn accept_progress(
        &mut self,
        input: AcceptProgressInput,
    ) -> Result<AcceptanceResult, CommandError> {
        self.repository
            .progress_change(|connection| progress::accept(connection, input))
            .map(|(workspace, result)| AcceptanceResult { workspace, result })
    }
    pub fn correct_acceptance(
        &mut self,
        input: CorrectAcceptanceInput,
    ) -> Result<Workspace, CommandError> {
        self.repository
            .progress_change(|connection| progress::correct(connection, input))
            .map(|(workspace, ())| workspace)
    }
    pub fn reject_review(&mut self, input: RejectReviewInput) -> Result<Workspace, CommandError> {
        self.repository
            .progress_change(|connection| progress::reject(connection, input))
            .map(|(workspace, ())| workspace)
    }
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

    pub fn backup_status(&self) -> crate::backup::BackupStatus {
        self.repository.backup_manager().status()
    }
    pub fn retry_backup(&self) -> crate::backup::BackupStatus {
        self.repository.backup_manager().retry()
    }
    pub fn preview_recovery(&mut self) -> Result<ImportPreview, CommandError> {
        self.discard_preview();
        let (bytes, name) = self.repository.backup_manager().latest_export()?;
        self.preview_import(&bytes, name)
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
                workspace
                    .daily
                    .objectives
                    .push(crate::daily::ObjectivePlanning {
                        objective_id: id.clone(),
                        status: crate::daily::ObjectiveStatus::Active,
                        priority: workspace.daily.objectives.len() as u32,
                        revision: 1,
                        updated_at: timestamp.clone(),
                    });
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
                if input.id.as_ref().is_some_and(|id| {
                    workspace.daily.objectives.iter().any(|o| {
                        &o.objective_id == id && o.status != crate::daily::ObjectiveStatus::Active
                    })
                }) {
                    return Err(CommandError::invalid(
                        "Reactivate the objective before focusing it.",
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
                apply_saved_work(
                    workspace,
                    &input.objective_id,
                    input.description,
                    input.stop_condition,
                    note_text,
                    &now(),
                )
            })
    }

    pub fn set_objective_status(
        &mut self,
        input: crate::daily::SetObjectiveStatusInput,
    ) -> Result<Workspace, CommandError> {
        valid_id(&input.objective_id)?;
        self.repository
            .change(&input.expected_generation.clone(), |workspace| {
                crate::daily::set_status(workspace, input)
            })
    }
    pub fn reorder_objectives(
        &mut self,
        input: crate::daily::ReorderObjectivesInput,
    ) -> Result<Workspace, CommandError> {
        self.repository
            .change(&input.expected_generation.clone(), |workspace| {
                crate::daily::reorder(workspace, input)
            })
    }
    pub fn record_checkpoint(
        &mut self,
        input: crate::daily::RecordCheckpointInput,
    ) -> Result<crate::daily::CheckpointResult, CommandError> {
        self.repository
            .progress_change(|c| crate::daily::record(c, input))
            .map(|(workspace, checkpoint)| crate::daily::CheckpointResult {
                workspace,
                checkpoint,
            })
    }
    pub fn plan_ordinary_opportunity(
        &mut self,
        input: PlanOrdinaryOpportunityInput,
    ) -> Result<Workspace, CommandError> {
        self.repository
            .progress_change(|c| study::plan_ordinary(c, input))
            .map(|(workspace, ())| workspace)
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
        let (records, source_schema) = parse_import(bytes)?;
        let workspace = self.repository.load()?;
        if source_schema < 4 && workspace.world.has_history() {
            return Err(CommandError::new(
                "unsafe_legacy_import",
                "An older export cannot replace River Reach history or its accounting. Choose a complete Stage 4 export.",
            ));
        }
        if source_schema < 3 && workspace.study.record_count() > 0 {
            return Err(CommandError::new(
                "unsafe_legacy_import",
                "An older export cannot replace opportunity or study history. Choose a complete Stage 3c export.",
            ));
        }
        if source_schema == 1 && !workspace.progress.grants.is_empty() {
            return Err(CommandError::new(
                "unsafe_legacy_import",
                "A legacy v1 export cannot replace earned reward history. Choose a complete Stage 3b export.",
            ));
        }
        if source_schema < 5 && workspace.daily.has_history(&workspace.objectives) {
            return Err(CommandError::new(
                "unsafe_legacy_import",
                "An older export cannot replace objective organization or checkpoint history. Choose a complete Stage 4.5 export.",
            ));
        }
        let token = new_id();
        let incoming_counts =
            RecordCounts::from(&records.clone().into_workspace(workspace.generation.clone()));
        self.pending_import = Some(PendingImport {
            token: token.clone(),
            generation: workspace.generation.clone(),
            records,
            source_schema,
        });
        Ok(ImportPreview {
            token,
            generation: workspace.generation.clone(),
            file_name,
            current_counts: RecordCounts::from(&workspace),
            incoming_counts,
            schema_version: source_schema,
            warning: if source_schema == 1 {
                "Legacy v1: criteria become unaccepted drafts; no credits are created. Legacy files cannot replace earned history.".into()
            } else if source_schema == 2 {
                "Legacy v2: complete reward history is preserved; opportunity and study fields initialize empty. Older files cannot replace any existing study history.".into()
            } else if source_schema == 3 {
                "Legacy v3: complete reward and study history is preserved; River Reach initializes empty. Older files cannot replace any existing River Reach history or accounting.".into()
            } else if source_schema == 4 {
                "Legacy v4: complete reward, study and River Reach history is preserved; objectives initialize active in stable ID order and checkpoints initialize empty. Older files cannot replace changed objective organization or checkpoint history.".into()
            } else {
                "Complete replacement includes objective lifecycle and priority, factual checkpoints and replay identities, Work, reward history, schedule epochs, frozen study protocols, assignments, outcomes and corrections, exposures, burden, reflections, construction preferences, and all River Reach construction, reservations, spending, layouts, command history, and Chronicle. History absent from this file will be removed. A complete recovery backup is saved first.".into()
            },
        })
    }

    pub fn restore_import(&mut self, input: RestoreInput) -> Result<RestoreResult, CommandError> {
        crate::restore_diagnostics::attempt(|| {
            crate::restore_diagnostics::attach_backup(
                self.repository.backup_manager().diagnostic_probe(),
            );
            crate::restore_diagnostics::phase("validate_preview");
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
                pending.source_schema,
            )?;
            self.pending_import = None;
            Ok(result)
        })
    }
}

pub(crate) fn apply_saved_work(
    workspace: &mut Workspace,
    objective_id: &str,
    description: String,
    stop_condition: String,
    note_text: Option<String>,
    timestamp: &str,
) -> Result<bool, CommandError> {
    if !workspace
        .objectives
        .iter()
        .any(|objective| objective.id == objective_id)
    {
        return Err(CommandError::invalid("The objective does not exist."));
    }
    let mut changed = false;
    if let Some(action) = workspace
        .actions
        .iter_mut()
        .find(|action| action.objective_id == objective_id)
    {
        if action.description != description || action.stop_condition != stop_condition {
            action.revision = next_revision(action.revision)?;
            action.description = description;
            action.stop_condition = stop_condition;
            action.updated_at = timestamp.to_owned();
            changed = true;
        }
    } else {
        workspace.actions.push(NextAction {
            id: new_id(),
            revision: 1,
            created_at: timestamp.to_owned(),
            updated_at: timestamp.to_owned(),
            objective_id: objective_id.to_owned(),
            description,
            stop_condition,
        });
        changed = true;
    }
    if let Some(note_text) = note_text {
        if let Some(note) = workspace
            .notes
            .iter_mut()
            .find(|note| note.objective_id == objective_id)
        {
            if note.text != note_text {
                note.revision = next_revision(note.revision)?;
                note.text = note_text;
                note.updated_at = timestamp.to_owned();
                changed = true;
            }
        } else {
            workspace.notes.push(ResumptionNote {
                id: new_id(),
                revision: 1,
                created_at: timestamp.to_owned(),
                updated_at: timestamp.to_owned(),
                objective_id: objective_id.to_owned(),
                text: note_text,
            });
            changed = true;
        }
    } else {
        let before = workspace.notes.len();
        workspace
            .notes
            .retain(|note| note.objective_id != objective_id);
        changed |= before != workspace.notes.len();
    }
    Ok(changed)
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &[u8] = include_bytes!("../../contracts/fixtures/sample-v1.json");

    fn import(service: &mut WorkspaceService, records: &RecordSet) -> Workspace {
        let preview = service
            .preview_import(
                &serde_json::to_vec(records).unwrap(),
                "synthetic.json".into(),
            )
            .unwrap();
        service
            .restore_import(RestoreInput {
                token: preview.token,
                expected_generation: preview.generation,
            })
            .unwrap()
            .workspace
    }

    #[test]
    fn note_revision_failure_rolls_back_a_simultaneous_action_edit_and_survives_reopen() {
        let directory = tempfile::tempdir().unwrap();
        let database = directory.path().join("workspace.sqlite3");
        let backups = directory.path().join("backups");
        let mut service = WorkspaceService::open(&database, &backups).unwrap();
        let mut records: RecordSet = serde_json::from_slice(SAMPLE).unwrap();
        records.notes[0].revision = u32::MAX;
        let original = import(&mut service, &records);
        let original_export = service.export_bytes().unwrap();
        let note = &records.notes[0];
        let action = records
            .actions
            .iter()
            .find(|action| action.objective_id == note.objective_id)
            .unwrap();
        let failure = service
            .save_work(SaveWorkInput {
                expected_generation: original.generation.clone(),
                objective_id: note.objective_id.clone(),
                description: "This action must not be partially saved".into(),
                stop_condition: action.stop_condition.clone(),
                note: Some("This note cannot advance its revision".into()),
            })
            .unwrap_err();
        assert_eq!(failure.code, "invalid_input");
        assert_eq!(service.load_workspace().unwrap(), original);
        assert_eq!(service.export_bytes().unwrap(), original_export);
        drop(service);
        let reopened = WorkspaceService::open(&database, &backups).unwrap();
        assert_eq!(reopened.load_workspace().unwrap(), original);
        assert_eq!(reopened.export_bytes().unwrap(), original_export);
    }

    #[test]
    fn unchanged_mutations_keep_preview_valid_but_stale_noops_are_rejected() {
        let directory = tempfile::tempdir().unwrap();
        let database = directory.path().join("workspace.sqlite3");
        let backups = directory.path().join("backups");
        let mut service = WorkspaceService::open(&database, &backups).unwrap();
        let records: RecordSet = serde_json::from_slice(SAMPLE).unwrap();
        let original = import(&mut service, &records);
        let preview = service.preview_import(SAMPLE, "same.json".into()).unwrap();
        let input = UpdateProjectInput {
            expected_generation: original.generation.clone(),
            id: original.projects[0].id.clone(),
            title: original.projects[0].title.clone(),
        };
        assert_eq!(service.update_project(input.clone()).unwrap(), original);
        let objective = &original.objectives[0];
        assert_eq!(
            service
                .update_objective(UpdateObjectiveInput {
                    expected_generation: original.generation.clone(),
                    id: objective.id.clone(),
                    title: objective.title.clone(),
                    criterion: objective.criterion.clone(),
                })
                .unwrap(),
            original
        );
        assert_eq!(
            service
                .select_objective(SelectObjectiveInput {
                    expected_generation: original.generation.clone(),
                    id: original.selected_objective_id.clone(),
                })
                .unwrap(),
            original
        );
        assert!(
            service
                .restore_import(RestoreInput {
                    token: preview.token,
                    expected_generation: preview.generation,
                })
                .unwrap()
                .unchanged
        );
        let current = service
            .create_project(CreateProjectInput {
                expected_generation: original.generation,
                title: "A concurrent synthetic addition".into(),
            })
            .unwrap();
        assert_eq!(service.update_project(input).unwrap_err().code, "conflict");
        assert_eq!(service.load_workspace().unwrap(), current);
    }
}
