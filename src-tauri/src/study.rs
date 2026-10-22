//! Prospective work observations. This module has no reward mutation capability.
use crate::domain::{CommandError, RecordSet, new_id, now, text, valid_id};
use chrono::{DateTime, SecondsFormat, Utc};
use rusqlite::{Connection, params};
use serde::{Deserialize, Serialize, Serializer, de::DeserializeOwned};
use sha2::{Digest, Sha256};
use std::collections::{HashMap, HashSet};

#[cfg(test)]
thread_local! { static TEST_CLOCK: std::cell::RefCell<Option<String>> = const { std::cell::RefCell::new(None) }; }
fn study_now() -> String {
    #[cfg(test)]
    if let Some(value) = TEST_CLOCK.with(|clock| clock.borrow().clone()) {
        return value;
    }
    now()
}

macro_rules! model {
    ($name:ident {$($field:ident:$ty:ty),* $(,)?}) => {
        #[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
        #[serde(deny_unknown_fields)]
        pub struct $name {$(#[serde(deserialize_with="required")] pub $field:$ty,)*}
    };
}
fn required<'de, D: serde::Deserializer<'de>, T: Deserialize<'de>>(d: D) -> Result<T, D::Error> {
    T::deserialize(d)
}
macro_rules! enumeration {
    ($name:ident {$($variant:ident),*}) => {
        #[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
        #[serde(rename_all="snake_case")]
        pub enum $name {$($variant),*}
    };
}
enumeration!(Phase { E00, E01 });
enumeration!(StudyStatus {
    Configured,
    Ready,
    Active,
    Paused,
    Completed,
    Abandoned
});
enumeration!(Condition { D, DPlusB });
enumeration!(Priority { Ordinary, High });
enumeration!(Report {
    Started,
    Nonstart,
    Unknown
});
enumeration!(Precision {
    Exact,
    Approximate,
    Unknown
});
enumeration!(OutcomeState {
    Pending,
    ObservedStart,
    ObservedNonstart,
    Missing
});
enumeration!(ExposureKind {
    CueDelivered,
    PreviewDelivered,
    CheckpointDelivered,
    Seen,
    VoluntaryWorld,
    Deviation
});
enumeration!(BurdenCategory {
    Planning,
    Logging,
    Review,
    Game,
    Correction,
    Other,
    UsefulWork
});

model!(ScheduleEpoch {
    id: String,
    label: String,
    effective_from: String,
    timezone: String,
    created_at: String
});
model!(StudyManifest {
    id:String,phase:Phase,protocol_version:String,created_at:String,epoch_id:String,
    algorithm:String,block_size:u32,seed:String,allocation:Vec<Condition>,allocation_hash:String,
    window_seconds:u32,preview_seconds:u32,reveal_rule:String,presentation_policy:String,
    analysis_definition:String,review_after:Vec<u32>
});
model!(ProtocolEvent {id:String,manifest_id:String,status:StudyStatus,reason:String,acknowledgements:Vec<String>,created_at:String,sequence:u32});
model!(WorkOpportunity {
    id:String,project_id:String,objective_id:String,criterion_id:String,criterion_version:u32,criterion_snapshot:String,criterion_registry_revision:u64,
    action_id:String,action_revision:u32,action_snapshot:String,stop_snapshot:String,
    domain:String,meaningful_start:String,cue_at:String,window_seconds:u32,timezone:String,
    epoch_id:String,eligible:bool,eligibility_decided_at:String,unavailability:Option<String>,
    priority:Priority,available_minutes:u32,deadline_at:Option<String>,context:Option<String>,
    manifest_id:Option<String>,protocol_event_id:Option<String>,created_at:String
});
model!(Assignment {
    id: String,
    opportunity_id: String,
    manifest_id: String,
    position: u32,
    condition: Condition,
    assigned_at: String,
    issued_bc_at_assignment: u32,
    ledger_sequence: u32,
    h1_saturated: bool
});
model!(OutcomeEvent {
    id:String,opportunity_id:String,sequence:u32,report:Report,started_at:Option<String>,
    precision:Precision,uncertainty_seconds:Option<u32>,observed_at:String,source:String,
    note:Option<String>,technical_failure:bool,uncertainty_reason:Option<String>,
    correction_reason:Option<String>,supersedes:Option<String>,state:OutcomeState,
    latency_seconds:Option<u32>,censored_at_seconds:Option<u32>,late_start:bool,
    priority_advanced:Option<bool>,quality_note:Option<String>,acceptance_id:Option<String>
});
model!(ExposureEvent {id:String,opportunity_id:Option<String>,kind:ExposureKind,note:Option<String>,created_at:String,issued_bc:u32,ledger_sequence:u32,preference_sequence:u32,protocol_event_id:Option<String>,h1_saturated:bool});
model!(BurdenEvent {id:String,opportunity_id:Option<String>,category:BurdenCategory,minutes:u32,note:Option<String>,created_at:String,method:String});
model!(Reflection {id:String,manifest_id:Option<String>,note:String,created_at:String});
model!(PresentationPreference {
    id: String,
    sequence: u32,
    enabled: bool,
    created_at: String
});
model!(SetConstructionEnabledInput {
    expected_generation: String,
    enabled: bool
});

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct StudyState {
    pub epochs: Vec<ScheduleEpoch>,
    pub manifests: Vec<StudyManifest>,
    pub protocol_events: Vec<ProtocolEvent>,
    pub opportunities: Vec<WorkOpportunity>,
    pub assignments: Vec<Assignment>,
    pub outcomes: Vec<OutcomeEvent>,
    pub exposures: Vec<ExposureEvent>,
    pub burdens: Vec<BurdenEvent>,
    pub reflections: Vec<Reflection>,
    pub preferences: Vec<PresentationPreference>,
}
model!(CreateScheduleEpochInput {
    expected_generation: String,
    label: String,
    effective_from: String,
    timezone: String
});
model!(ConfigureStudyInput {
    expected_generation: String,
    phase: Phase,
    epoch_id: String
});
model!(TransitionStudyInput {expected_generation:String,manifest_id:String,status:StudyStatus,reason:String,acknowledgements:Vec<String>});
model!(PlanOpportunityInput {
    expected_generation:String,objective_id:String,epoch_id:String,manifest_id:Option<String>,domain:String,
    meaningful_start:String,cue_at:String,timezone:String,window_seconds:u32,eligible:bool,
    unavailability:Option<String>,priority:Priority,available_minutes:u32,deadline_at:Option<String>,context:Option<String>
});
model!(PlanOrdinaryOpportunityInput {
    expected_generation: String,
    objective_id: String,
    meaningful_start: String,
    cue_at: String,
    timezone: String,
    available_minutes: u32
});
model!(RecordOpportunityOutcomeInput {
    expected_generation:String,opportunity_id:String,report:Report,started_at:Option<String>,precision:Precision,
    uncertainty_seconds:Option<u32>,note:Option<String>,technical_failure:bool,
    uncertainty_reason:Option<String>,correction_reason:Option<String>,priority_advanced:Option<bool>,quality_note:Option<String>,acceptance_id:Option<String>
});
model!(RecordStudyExposureInput {expected_generation:String,opportunity_id:Option<String>,kind:ExposureKind,note:Option<String>});
model!(RecordStudyBurdenInput {expected_generation:String,opportunity_id:Option<String>,category:BurdenCategory,minutes:u32,note:Option<String>});
model!(RecordStudyReflectionInput {expected_generation:String,manifest_id:Option<String>,note:String});

const ANALYSIS: &str = "as-assigned-v1: all originally eligible assigned opportunities; starts/N to (starts+missing)/N; pending separate; epoch and deadline strata; no significance claim";
const ACKS: [&str; 7] = [
    "e00_assessed",
    "frozen_protocol",
    "schedule_epoch",
    "assignment_and_window",
    "matched_support",
    "as_assigned_analysis",
    "voluntary_stop",
];
fn invalid(message: &str) -> CommandError {
    CommandError::invalid(message)
}
fn db<T>(result: rusqlite::Result<T>) -> Result<T, CommandError> {
    result.map_err(|error| {
        crate::restore_diagnostics::sqlite(&error);
        CommandError::storage()
    })
}
fn encode<T: Serialize>(value: &T) -> Result<String, CommandError> {
    serde_json::to_string(value).map_err(|_| CommandError::storage())
}
fn stamp(value: &str) -> Result<DateTime<Utc>, CommandError> {
    let t = DateTime::parse_from_rfc3339(value)
        .map_err(|_| invalid("Study timestamps require UTC millisecond precision."))?
        .with_timezone(&Utc);
    if value.len() != 24 || !("1900"..="2200").contains(&&value[..4]) {
        return Err(invalid(
            "Study timestamps must be between years 1900 and 2200.",
        ));
    }
    if t.to_rfc3339_opts(SecondsFormat::Millis, true) != value {
        return Err(invalid(
            "Study timestamps require UTC millisecond precision.",
        ));
    }
    Ok(t)
}
fn optional(value: &Option<String>, field: &str) -> Result<(), CommandError> {
    if let Some(v) = value {
        text(v, 2000, field)?;
    }
    Ok(())
}
fn insert<T: Serialize>(c: &Connection, table: &str, id: &str, v: &T) -> Result<(), CommandError> {
    crate::restore_diagnostics::table_operation("insert", table);
    db(c.execute(
        &format!("INSERT INTO {table}(id,payload) VALUES(?1,?2)"),
        params![id, encode(v)?],
    ))?;
    Ok(())
}
fn rows<T: DeserializeOwned>(c: &Connection, table: &str) -> Result<Vec<T>, CommandError> {
    let mut q = db(c.prepare(&format!("SELECT payload FROM {table} ORDER BY id")))?;
    db(q.query_map([], |r| r.get::<_, String>(0)))?
        .map(|v| serde_json::from_str(&db(v)?).map_err(|_| CommandError::storage()))
        .collect()
}
fn generation(c: &Connection, expected: &str) -> Result<(), CommandError> {
    valid_id(expected)?;
    let g: String = db(c.query_row(
        "SELECT generation FROM workspace_state WHERE singleton=1",
        [],
        |r| r.get(0),
    ))?;
    if g != expected {
        return Err(CommandError::conflict());
    }
    Ok(())
}
fn changed(c: &Connection) -> Result<(), CommandError> {
    db(c.execute(
        "UPDATE workspace_state SET generation=?1 WHERE singleton=1",
        [new_id()],
    ))?;
    Ok(())
}
pub(crate) fn read(c: &Connection) -> Result<StudyState, CommandError> {
    Ok(StudyState {
        epochs: rows(c, "schedule_epochs")?,
        manifests: rows(c, "study_manifests")?,
        protocol_events: rows(c, "study_protocol_events")?,
        opportunities: rows(c, "work_opportunities")?,
        assignments: rows(c, "study_assignments")?,
        outcomes: rows(c, "opportunity_outcomes")?,
        exposures: rows(c, "study_exposures")?,
        burdens: rows(c, "study_burdens")?,
        reflections: rows(c, "study_reflections")?,
        preferences: rows(c, "presentation_preferences")?,
    })
}
impl StudyState {
    pub fn record_count(&self) -> usize {
        self.epochs.len()
            + self.manifests.len()
            + self.protocol_events.len()
            + self.opportunities.len()
            + self.assignments.len()
            + self.outcomes.len()
            + self.exposures.len()
            + self.burdens.len()
            + self.reflections.len()
            + self.preferences.len()
    }
    pub fn normalize(&mut self) {
        macro_rules! sort {($($f:ident),*)=>{$(self.$f.sort_by(|a,b|a.id.cmp(&b.id));)*};}
        sort!(
            epochs,
            manifests,
            protocol_events,
            opportunities,
            assignments,
            outcomes,
            exposures,
            burdens,
            reflections,
            preferences
        );
    }
    pub fn status(&self, id: &str) -> StudyStatus {
        self.protocol_events
            .iter()
            .filter(|e| e.manifest_id == id)
            .max_by_key(|e| e.sequence)
            .map_or(StudyStatus::Configured, |e| e.status)
    }
    pub fn latest_outcome(&self, id: &str) -> Option<&OutcomeEvent> {
        self.outcomes
            .iter()
            .filter(|e| e.opportunity_id == id)
            .max_by_key(|e| e.sequence)
    }
}

// SHA-256 counter stream + rejection sampling + Fisher-Yates: stable across
// platforms, balanced blocks, independent of observations and app-open events.
fn allocation(seed: &str, blocks: u32) -> Vec<Condition> {
    let mut result = Vec::new();
    let mut counter = 0u64;
    for _ in 0..blocks {
        let mut block = [
            Condition::D,
            Condition::D,
            Condition::DPlusB,
            Condition::DPlusB,
        ];
        for i in (1..4).rev() {
            let n = (i + 1) as u32;
            let limit = u32::MAX - u32::MAX % n;
            let sample = loop {
                let digest = Sha256::digest(
                    format!("ascension-e01-sha256-fy-v1:{seed}:{counter}").as_bytes(),
                );
                counter += 1;
                let value = u32::from_be_bytes(digest[..4].try_into().expect("four digest bytes"));
                if value < limit {
                    break value % n;
                }
            };
            block.swap(i, sample as usize);
        }
        result.extend(block);
    }
    result
}
fn allocation_hash(sequence: &[Condition]) -> Result<String, CommandError> {
    Ok(format!(
        "{:x}",
        Sha256::digest(encode(&sequence)?.as_bytes())
    ))
}
pub(crate) fn create_epoch(
    c: &Connection,
    i: CreateScheduleEpochInput,
) -> Result<(), CommandError> {
    generation(c, &i.expected_generation)?;
    text(&i.label, 120, "Schedule label")?;
    text(&i.timezone, 100, "Timezone")?;
    stamp(&i.effective_from)?;
    let s = read(c)?;
    let timestamp = study_now();
    for m in &s.manifests {
        if m.phase == Phase::E01 && s.status(&m.id) == StudyStatus::Active {
            let e=ProtocolEvent{id:new_id(),manifest_id:m.id.clone(),status:StudyStatus::Paused,reason:"A new schedule epoch was declared; existing assignments are preserved. A new versioned protocol is required for another epoch.".into(),acknowledgements:vec![],created_at:timestamp.clone(),sequence:next_protocol_sequence(&s,&m.id)};
            insert(c, "study_protocol_events", &e.id, &e)?;
        }
    }
    let row = ScheduleEpoch {
        id: new_id(),
        label: i.label,
        effective_from: i.effective_from,
        timezone: i.timezone,
        created_at: timestamp,
    };
    insert(c, "schedule_epochs", &row.id, &row)?;
    changed(c)
}
fn next_protocol_sequence(s: &StudyState, id: &str) -> u32 {
    s.protocol_events
        .iter()
        .filter(|e| e.manifest_id == id)
        .map(|e| e.sequence)
        .max()
        .unwrap_or(0)
        + 1
}
pub(crate) fn configure(c: &Connection, i: ConfigureStudyInput) -> Result<(), CommandError> {
    generation(c, &i.expected_generation)?;
    let s = read(c)?;
    if !s.epochs.iter().any(|e| e.id == i.epoch_id) {
        return Err(invalid("Choose an existing schedule epoch."));
    }
    let seed = if i.phase == Phase::E01 {
        new_id()
    } else {
        String::new()
    };
    let sequence = if i.phase == Phase::E01 {
        allocation(&seed, 15)
    } else {
        vec![]
    };
    let row=StudyManifest{id:new_id(),phase:i.phase,protocol_version:"stage3c-v1".into(),created_at:study_now(),epoch_id:i.epoch_id,algorithm:if i.phase==Phase::E01{"sha256-fy-v1"}else{"none"}.into(),block_size:if i.phase==Phase::E01{4}else{0},seed,allocation_hash:allocation_hash(&sequence)?,allocation:sequence,window_seconds:900,preview_seconds:60,reveal_rule:"preview: cue minus 60 seconds through cue; checkpoint: after observation window; no animation during work".into(),presentation_policy:"h1-ledger-64-v1".into(),analysis_definition:ANALYSIS.into(),review_after:vec![30,60]};
    insert(c, "study_manifests", &row.id, &row)?;
    changed(c)
}
fn validate_acknowledgements(values: &[String]) -> Result<(), CommandError> {
    let unique: HashSet<_> = values.iter().collect();
    if unique.len() != values.len() || values.iter().any(|v| !ACKS.contains(&v.as_str())) {
        return Err(invalid(
            "Protocol acknowledgements must be recognized and unique.",
        ));
    }
    Ok(())
}
fn status_at(s: &StudyState, id: &str, at: &str) -> StudyStatus {
    s.protocol_events
        .iter()
        .filter(|e| e.manifest_id == id && e.created_at.as_str() <= at)
        .max_by_key(|e| e.sequence)
        .map_or(StudyStatus::Configured, |e| e.status)
}
fn valid_transition(from: StudyStatus, to: StudyStatus) -> bool {
    matches!(
        (from, to),
        (
            StudyStatus::Configured,
            StudyStatus::Ready | StudyStatus::Active | StudyStatus::Abandoned
        ) | (
            StudyStatus::Ready,
            StudyStatus::Active | StudyStatus::Abandoned
        ) | (
            StudyStatus::Active,
            StudyStatus::Paused | StudyStatus::Completed | StudyStatus::Abandoned
        ) | (
            StudyStatus::Paused,
            StudyStatus::Active | StudyStatus::Completed | StudyStatus::Abandoned
        )
    )
}
pub(crate) fn transition(c: &Connection, i: TransitionStudyInput) -> Result<(), CommandError> {
    generation(c, &i.expected_generation)?;
    validate_acknowledgements(&i.acknowledgements)?;
    text(&i.reason, 2000, "Protocol decision reason")?;
    let s = read(c)?;
    let m = s
        .manifests
        .iter()
        .find(|m| m.id == i.manifest_id)
        .ok_or_else(|| invalid("The study does not exist."))?;
    let from = s.status(&m.id);
    if !valid_transition(from, i.status) {
        return Err(invalid("This study lifecycle transition is not permitted."));
    }
    if i.status == StudyStatus::Active {
        if s.manifests
            .iter()
            .any(|other| other.id != m.id && s.status(&other.id) == StudyStatus::Active)
        {
            return Err(invalid(
                "Pause the other study before activating this protocol.",
            ));
        }
        if m.phase == Phase::E01
            && (from == StudyStatus::Configured
                || i.acknowledgements.len() != ACKS.len()
                || ACKS
                    .iter()
                    .any(|a| !i.acknowledgements.iter().any(|v| v == a)))
        {
            return Err(invalid(
                "E01 requires readiness review, an explicit E00 feasibility assessment, and every protocol acknowledgement.",
            ));
        }
    }
    let row = ProtocolEvent {
        id: new_id(),
        manifest_id: i.manifest_id,
        status: i.status,
        reason: i.reason,
        acknowledgements: i.acknowledgements,
        created_at: study_now(),
        sequence: next_protocol_sequence(&s, &m.id),
    };
    insert(c, "study_protocol_events", &row.id, &row)?;
    changed(c)
}
/// Ordinary planning is deliberately independent of protocol enrollment and epoch
/// transitions. Creating this contextual epoch must not pause an active E01.
/// The repository runs epoch creation and the declaration in one transaction.
pub(crate) fn plan_ordinary(
    c: &Connection,
    i: PlanOrdinaryOpportunityInput,
) -> Result<(), CommandError> {
    generation(c, &i.expected_generation)?;
    valid_id(&i.objective_id)?;
    text(&i.timezone, 100, "Timezone")?;
    let at = study_now();
    let cue = stamp(&i.cue_at)?;
    if cue <= stamp(&at)? {
        return Err(invalid("Declare the opportunity before its cue."));
    }
    let state = read(c)?;
    let epoch = state
        .epochs
        .iter()
        .filter(|e| {
            e.label == "Ordinary work (automatic context)"
                && e.timezone == i.timezone
                && e.effective_from <= at
                && !state.manifests.iter().any(|m| m.epoch_id == e.id)
        })
        .min_by(|a, b| a.created_at.cmp(&b.created_at).then(a.id.cmp(&b.id)));
    let epoch_id = if let Some(epoch) = epoch {
        epoch.id.clone()
    } else {
        let row = ScheduleEpoch {
            id: new_id(),
            label: "Ordinary work (automatic context)".into(),
            effective_from: at.clone(),
            timezone: i.timezone.clone(),
            created_at: at,
        };
        insert(c, "schedule_epochs", &row.id, &row)?;
        row.id
    };
    plan(
        c,
        PlanOpportunityInput {
            expected_generation: i.expected_generation,
            objective_id: i.objective_id,
            epoch_id,
            manifest_id: None,
            domain: "Ordinary work".into(),
            meaningful_start: i.meaningful_start,
            cue_at: i.cue_at,
            timezone: i.timezone,
            window_seconds: 900,
            eligible: true,
            unavailability: None,
            priority: Priority::Ordinary,
            available_minutes: i.available_minutes,
            deadline_at: None,
            context: None,
        },
    )
}

pub(crate) fn plan(c: &Connection, i: PlanOpportunityInput) -> Result<(), CommandError> {
    generation(c, &i.expected_generation)?;
    let s = read(c)?;
    let at = study_now();
    let cue = stamp(&i.cue_at)?;
    if cue <= stamp(&at)? {
        return Err(invalid("Declare the opportunity before its cue."));
    }
    text(&i.domain, 120, "Work domain")?;
    text(&i.meaningful_start, 2000, "Meaningful start definition")?;
    text(&i.timezone, 100, "Timezone")?;
    if !(60..=86400).contains(&i.window_seconds) || !(1..=1440).contains(&i.available_minutes) {
        return Err(invalid(
            "Choose a window of 60–86400 seconds and available time of 1–1440 minutes.",
        ));
    }
    optional(&i.unavailability, "Known unavailability")?;
    optional(&i.context, "Optional context")?;
    if i.eligible && i.unavailability.is_some() || !i.eligible && i.unavailability.is_none() {
        return Err(invalid(
            "Record known unavailability for ineligible opportunities only.",
        ));
    }
    let epoch = s
        .epochs
        .iter()
        .find(|e| e.id == i.epoch_id)
        .ok_or_else(|| invalid("Choose a schedule epoch."))?;
    if cue < stamp(&epoch.effective_from)? {
        return Err(invalid("The cue precedes the schedule epoch."));
    }
    if let Some(deadline) = &i.deadline_at {
        stamp(deadline)?;
    }
    let objective: (String,) = db(c.query_row(
        "SELECT project_id FROM objectives WHERE id=?1",
        [&i.objective_id],
        |r| Ok((r.get(0)?,)),
    ))?;
    let action: (String, u32, String, String) = db(c.query_row(
        "SELECT id,revision,description,stop_condition FROM actions WHERE objective_id=?1",
        [&i.objective_id],
        |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
    ))?;
    let progress = crate::progress::read(c)?;
    let parent = progress
        .parents
        .iter()
        .find(|p| p.parent_id == i.objective_id)
        .ok_or_else(|| invalid("The objective does not exist."))?;
    let criterion = progress
        .criteria
        .iter()
        .filter(|r| r.criterion_lineage_id == parent.primary_lineage_id)
        .max_by_key(|r| r.criterion_version)
        .ok_or_else(|| invalid("The objective criterion does not exist."))?;
    let row = WorkOpportunity {
        id: new_id(),
        project_id: objective.0,
        objective_id: i.objective_id,
        criterion_id: criterion.id.clone(),
        criterion_version: criterion.criterion_version,
        criterion_snapshot: criterion.description.clone(),
        criterion_registry_revision: criterion.registry_revision,
        action_id: action.0,
        action_revision: action.1,
        action_snapshot: action.2,
        stop_snapshot: action.3,
        domain: i.domain,
        meaningful_start: i.meaningful_start,
        cue_at: i.cue_at,
        window_seconds: i.window_seconds,
        timezone: i.timezone,
        epoch_id: i.epoch_id,
        eligible: i.eligible,
        eligibility_decided_at: at.clone(),
        unavailability: i.unavailability,
        priority: i.priority,
        available_minutes: i.available_minutes,
        deadline_at: i.deadline_at,
        context: i.context,
        protocol_event_id: i.manifest_id.as_ref().and_then(|id| {
            s.protocol_events
                .iter()
                .filter(|e| &e.manifest_id == id)
                .max_by_key(|e| e.sequence)
                .map(|e| e.id.clone())
        }),
        manifest_id: i.manifest_id,
        created_at: at.clone(),
    };
    let assignment = if let Some(id) = &row.manifest_id {
        let m = s
            .manifests
            .iter()
            .find(|m| &m.id == id)
            .ok_or_else(|| invalid("The study does not exist."))?;
        if s.status(id) != StudyStatus::Active
            || m.epoch_id != row.epoch_id
            || m.window_seconds != row.window_seconds
        {
            return Err(invalid(
                "Use the active protocol's epoch and frozen observation window.",
            ));
        }
        if m.phase == Phase::E01 && row.eligible {
            if cue <= stamp(&at)? + chrono::Duration::seconds(i64::from(m.preview_seconds)) {
                return Err(invalid("Assign E01 before its 60-second preview boundary."));
            }
            if s.opportunities
                .iter()
                .filter(|o| o.manifest_id.as_ref() == Some(id) && o.eligible)
                .any(|o| {
                    let a = stamp(&o.cue_at).expect("validated cue");
                    let end = a + chrono::Duration::seconds(i64::from(o.window_seconds));
                    cue - chrono::Duration::seconds(60) < end
                        && cue + chrono::Duration::seconds(i64::from(row.window_seconds))
                            > a - chrono::Duration::seconds(60)
                })
            {
                return Err(invalid("E01 presentation windows may not overlap."));
            }
            let position = s
                .assignments
                .iter()
                .filter(|a| &a.manifest_id == id)
                .count() as u32;
            let condition = *m.allocation.get(position as usize).ok_or_else(|| {
                invalid("The frozen allocation is exhausted; a new protocol decision is required.")
            })?;
            Some(Assignment {
                id: new_id(),
                opportunity_id: row.id.clone(),
                manifest_id: id.clone(),
                position,
                condition,
                assigned_at: at,
                issued_bc_at_assignment: progress.wallet_bc,
                ledger_sequence: progress.ledger.len() as u32,
                h1_saturated: progress.wallet_bc >= 64,
            })
        } else {
            None
        }
    } else {
        None
    };
    insert(c, "work_opportunities", &row.id, &row)?;
    if let Some(a) = assignment {
        insert(c, "study_assignments", &a.id, &a)?;
    }
    changed(c)
}

fn classify(o: &WorkOpportunity, e: &mut OutcomeEvent) -> Result<(), CommandError> {
    let cue = stamp(&o.cue_at)?;
    let end = cue + chrono::Duration::seconds(i64::from(o.window_seconds));
    let observed = stamp(&e.observed_at)?;
    if observed < cue {
        return Err(invalid("An outcome cannot be observed before the cue."));
    }
    e.state = OutcomeState::Missing;
    e.latency_seconds = None;
    e.censored_at_seconds = None;
    e.late_start = false;
    match e.report {
        Report::Nonstart => {
            if observed < end {
                return Err(invalid(
                    "Record a nonstart only after the observation window.",
                ));
            }
            if e.started_at.is_some()
                || e.precision != Precision::Unknown
                || e.uncertainty_seconds.is_some()
            {
                return Err(invalid("A nonstart cannot supply a start timestamp."));
            }
            e.state = OutcomeState::ObservedNonstart;
            e.censored_at_seconds = Some(o.window_seconds);
        }
        Report::Unknown => {
            if observed < end {
                return Err(invalid(
                    "Leave the opportunity pending until its window ends.",
                ));
            }
            if e.started_at.is_some()
                || e.precision != Precision::Unknown
                || e.uncertainty_seconds.is_some()
            {
                return Err(invalid(
                    "Unknown timing must not contain a fabricated start timestamp.",
                ));
            }
        }
        Report::Started => {
            if e.precision == Precision::Unknown {
                if e.started_at.is_some() || e.uncertainty_seconds.is_some() {
                    return Err(invalid("Unknown timing cannot supply an exact timestamp."));
                }
                if e.uncertainty_reason.is_none() {
                    return Err(invalid(
                        "Explain the uncertainty without inventing a start time.",
                    ));
                }
            } else {
                let start = stamp(
                    e.started_at
                        .as_ref()
                        .ok_or_else(|| invalid("Provide the reported start time."))?,
                )?;
                let tolerance = match e.precision {
                    Precision::Exact => {
                        if e.uncertainty_seconds.is_some() {
                            return Err(invalid(
                                "Exact reports cannot carry approximate uncertainty.",
                            ));
                        }
                        0
                    }
                    Precision::Approximate => e
                        .uncertainty_seconds
                        .filter(|n| *n > 0 && *n <= 86400)
                        .ok_or_else(|| {
                            invalid("Approximate timing requires a positive uncertainty interval.")
                        })?,
                    Precision::Unknown => unreachable!(),
                };
                let low = start - chrono::Duration::seconds(i64::from(tolerance));
                let high = start + chrono::Duration::seconds(i64::from(tolerance));
                if start > observed || high < cue {
                    return Err(invalid(
                        "The reported start cannot precede its cue or lie after the report.",
                    ));
                }
                if low >= cue && high <= end {
                    e.state = OutcomeState::ObservedStart;
                    if e.precision == Precision::Exact {
                        e.latency_seconds = Some((start - cue).num_seconds() as u32);
                    }
                } else if low > end {
                    if observed < end {
                        return Err(invalid("A late start follows the primary window."));
                    }
                    e.state = OutcomeState::ObservedNonstart;
                    e.censored_at_seconds = Some(o.window_seconds);
                    e.late_start = true;
                } else if e.uncertainty_reason.is_none() {
                    return Err(invalid(
                        "Timing crosses the observation boundary; preserve an uncertainty reason.",
                    ));
                }
            }
        }
    }
    if e.state == OutcomeState::Missing && observed < end {
        return Err(invalid(
            "The window is still open. Leave this uncertain outcome pending and review it after the window.",
        ));
    }
    Ok(())
}
pub(crate) fn outcome(
    c: &Connection,
    i: RecordOpportunityOutcomeInput,
) -> Result<(), CommandError> {
    generation(c, &i.expected_generation)?;
    let s = read(c)?;
    let o = s
        .opportunities
        .iter()
        .find(|o| o.id == i.opportunity_id)
        .ok_or_else(|| invalid("The opportunity does not exist."))?;
    optional(&i.note, "Factual note")?;
    optional(&i.quality_note, "Domain quality note")?;
    optional(&i.uncertainty_reason, "Uncertainty reason")?;
    optional(&i.correction_reason, "Correction reason")?;
    let previous = s.latest_outcome(&o.id);
    if previous.is_some() && i.correction_reason.is_none() {
        return Err(invalid(
            "An outcome correction requires a reason; the original remains preserved.",
        ));
    }
    if previous.is_none() && i.correction_reason.is_some() {
        return Err(invalid("There is no earlier observation to correct."));
    }
    let mut e = OutcomeEvent {
        id: new_id(),
        opportunity_id: i.opportunity_id,
        sequence: previous.map_or(1, |p| p.sequence + 1),
        report: i.report,
        started_at: i.started_at,
        precision: i.precision,
        uncertainty_seconds: i.uncertainty_seconds,
        observed_at: study_now(),
        source: "local_user_self_report".into(),
        note: i.note,
        technical_failure: i.technical_failure,
        uncertainty_reason: i.uncertainty_reason,
        correction_reason: i.correction_reason,
        supersedes: previous.map(|p| p.id.clone()),
        state: OutcomeState::Missing,
        latency_seconds: None,
        censored_at_seconds: None,
        late_start: false,
        priority_advanced: i.priority_advanced,
        quality_note: i.quality_note,
        acceptance_id: i.acceptance_id,
    };
    classify(o, &mut e)?;
    insert(c, "opportunity_outcomes", &e.id, &e)?;
    changed(c)
}

#[derive(Clone, Debug, Serialize)]
pub struct Presentation {
    pub opportunity_id: Option<String>,
    pub condition: Option<Condition>,
    pub boundary: String,
    pub world_allowed: bool,
    pub enabled: bool,
}
fn presentation(s: &StudyState, at: &str) -> Presentation {
    let enabled = s
        .preferences
        .iter()
        .max_by_key(|p| p.sequence)
        .is_none_or(|p| p.enabled);
    let mut result = presentation_boundary(s, at);
    result.enabled = enabled;
    result.world_allowed &= enabled;
    result
}
fn presentation_boundary(s: &StudyState, at: &str) -> Presentation {
    let active = s
        .manifests
        .iter()
        .find(|m| m.phase == Phase::E01 && status_at(s, &m.id, at) == StudyStatus::Active);
    let Some(m) = active else {
        return Presentation {
            opportunity_id: None,
            condition: None,
            boundary: "none".into(),
            world_allowed: true,
            enabled: true,
        };
    };
    let candidate = s
        .opportunities
        .iter()
        .filter(|o| o.manifest_id.as_ref() == Some(&m.id) && o.eligible && o.cue_at.as_str() <= at)
        .max_by(|a, b| a.cue_at.cmp(&b.cue_at));
    let time = stamp(at).expect("current timestamp");
    let preview = s
        .opportunities
        .iter()
        .filter(|o| o.manifest_id.as_ref() == Some(&m.id) && o.eligible)
        .find(|o| {
            stamp(&o.cue_at)
                .is_ok_and(|cue| time >= cue - chrono::Duration::seconds(60) && time < cue)
        });
    let o = preview.or(candidate);
    if let Some(o) = o
        && let Some(a) = s.assignments.iter().find(|a| a.opportunity_id == o.id)
    {
        let cue = stamp(&o.cue_at).expect("validated cue");
        let boundary = if preview.is_some() {
            "preview"
        } else if time >= cue + chrono::Duration::seconds(i64::from(o.window_seconds)) {
            "checkpoint"
        } else {
            "none"
        };
        return Presentation {
            opportunity_id: Some(o.id.clone()),
            condition: Some(a.condition),
            boundary: boundary.into(),
            world_allowed: a.condition == Condition::DPlusB && boundary != "none",
            enabled: true,
        };
    }
    Presentation {
        opportunity_id: None,
        condition: None,
        boundary: "none".into(),
        world_allowed: false,
        enabled: true,
    }
}
pub(crate) fn exposure(c: &Connection, i: RecordStudyExposureInput) -> Result<(), CommandError> {
    generation(c, &i.expected_generation)?;
    let s = read(c)?;
    check_opportunity(&s, &i.opportunity_id)?;
    optional(&i.note, "Exposure note")?;
    // A cue/preview/checkpoint boundary has one durable delivery identity.
    // Renderer retry after a lost response must not duplicate that observation.
    if matches!(
        i.kind,
        ExposureKind::CueDelivered
            | ExposureKind::PreviewDelivered
            | ExposureKind::CheckpointDelivered
    ) && s
        .exposures
        .iter()
        .any(|e| e.opportunity_id == i.opportunity_id && e.kind == i.kind)
    {
        return Ok(());
    }
    let at = study_now();
    let p = presentation(&s, &at);
    if matches!(
        i.kind,
        ExposureKind::PreviewDelivered | ExposureKind::CheckpointDelivered
    ) {
        let expected = if i.kind == ExposureKind::PreviewDelivered {
            "preview"
        } else {
            "checkpoint"
        };
        if !p.world_allowed || p.boundary != expected || p.opportunity_id != i.opportunity_id {
            return Err(invalid(
                "Construction delivery is outside its assigned presentation boundary.",
            ));
        }
    }
    if i.kind == ExposureKind::CueDelivered {
        let o = s
            .opportunities
            .iter()
            .find(|o| Some(&o.id) == i.opportunity_id.as_ref())
            .ok_or_else(|| invalid("Cue delivery requires an opportunity."))?;
        if at < o.cue_at {
            return Err(invalid("The cue has not occurred."));
        }
    }
    if matches!(
        i.kind,
        ExposureKind::VoluntaryWorld | ExposureKind::Deviation
    ) && i.note.is_none()
    {
        return Err(invalid("Record a short exposure or deviation reason."));
    }
    let progress = crate::progress::read(c)?;
    let issued = progress.wallet_bc;
    let e = ExposureEvent {
        id: new_id(),
        opportunity_id: i.opportunity_id,
        kind: i.kind,
        note: i.note,
        created_at: at,
        issued_bc: issued,
        ledger_sequence: progress.ledger.len() as u32,
        preference_sequence: s.preferences.len() as u32,
        protocol_event_id: s
            .manifests
            .iter()
            .find(|m| s.status(&m.id) == StudyStatus::Active)
            .and_then(|m| {
                s.protocol_events
                    .iter()
                    .filter(|e| e.manifest_id == m.id)
                    .max_by_key(|e| e.sequence)
                    .map(|e| e.id.clone())
            }),
        h1_saturated: issued >= 64,
    };
    insert(c, "study_exposures", &e.id, &e)?;
    changed(c)
}
fn check_opportunity(s: &StudyState, id: &Option<String>) -> Result<(), CommandError> {
    if id
        .as_ref()
        .is_some_and(|id| !s.opportunities.iter().any(|o| &o.id == id))
    {
        return Err(invalid("The opportunity does not exist."));
    }
    Ok(())
}
pub(crate) fn burden(c: &Connection, i: RecordStudyBurdenInput) -> Result<(), CommandError> {
    generation(c, &i.expected_generation)?;
    let s = read(c)?;
    check_opportunity(&s, &i.opportunity_id)?;
    optional(&i.note, "Burden note")?;
    if i.minutes > 1440 {
        return Err(invalid("A burden estimate cannot exceed 1440 minutes."));
    }
    let e = BurdenEvent {
        id: new_id(),
        opportunity_id: i.opportunity_id,
        category: i.category,
        minutes: i.minutes,
        note: i.note,
        created_at: study_now(),
        method: "user_estimate_exclusive_minutes".into(),
    };
    insert(c, "study_burdens", &e.id, &e)?;
    changed(c)
}
pub(crate) fn reflection(
    c: &Connection,
    i: RecordStudyReflectionInput,
) -> Result<(), CommandError> {
    generation(c, &i.expected_generation)?;
    let s = read(c)?;
    if i.manifest_id
        .as_ref()
        .is_some_and(|id| !s.manifests.iter().any(|m| &m.id == id))
    {
        return Err(invalid("The study does not exist."));
    }
    text(&i.note, 2000, "Optional weekly reflection")?;
    let e = Reflection {
        id: new_id(),
        manifest_id: i.manifest_id,
        note: i.note,
        created_at: study_now(),
    };
    insert(c, "study_reflections", &e.id, &e)?;
    changed(c)
}

pub(crate) fn serialize_view<S: Serializer>(
    s: &StudyState,
    serializer: S,
) -> Result<S::Ok, S::Error> {
    let at = study_now();
    // These indexes live only for this validated workspace projection. They do
    // not cache authority across commands or replace the original event history.
    let opportunities: HashMap<_, _> = s.opportunities.iter().map(|o| (o.id.as_str(), o)).collect();
    let mut latest_outcomes: HashMap<&str, &OutcomeEvent> =
        HashMap::with_capacity(s.opportunities.len());
    for outcome in &s.outcomes {
        let latest = latest_outcomes
            .entry(outcome.opportunity_id.as_str())
            .or_insert(outcome);
        // Match Iterator::max_by_key, including its last-on-equal behavior.
        if outcome.sequence >= latest.sequence {
            *latest = outcome;
        }
    }
    let mut value = serde_json::to_value(s).map_err(serde::ser::Error::custom)?;
    for m in value["manifests"].as_array_mut().expect("manifest array") {
        let id = m["id"].as_str().expect("manifest id").to_owned();
        m.as_object_mut().expect("manifest").remove("seed");
        m.as_object_mut().expect("manifest").remove("allocation");
        m["status"] = serde_json::to_value(s.status(&id)).map_err(serde::ser::Error::custom)?;
    }
    for a in value["assignments"].as_array_mut().expect("assignments") {
        let o = opportunities.get(a["opportunity_id"].as_str().unwrap_or_default());
        if o.is_some_and(|o| {
            stamp(&o.cue_at)
                .is_ok_and(|cue| stamp(&at).expect("now") < cue - chrono::Duration::seconds(60))
        }) {
            a["condition"] = serde_json::Value::Null;
        }
    }
    value["presentation"] =
        serde_json::to_value(presentation(s, &at)).map_err(serde::ser::Error::custom)?;
    for o in value["opportunities"]
        .as_array_mut()
        .expect("opportunities")
    {
        let id = o["id"].as_str().expect("opportunity id");
        let raw = opportunities.get(id).expect("opportunity");
        let state = latest_outcomes.get(id).map_or_else(
            || {
                if stamp(&at).expect("now")
                    >= stamp(&raw.cue_at).expect("cue")
                        + chrono::Duration::seconds(i64::from(raw.window_seconds))
                {
                    OutcomeState::Missing
                } else {
                    OutcomeState::Pending
                }
            },
            |e| e.state,
        );
        o["outcome_state"] = serde_json::to_value(state).map_err(serde::ser::Error::custom)?;
    }
    value.serialize(serializer)
}

pub(crate) fn restore(c: &Connection, s: &StudyState) -> Result<(), CommandError> {
    macro_rules! put {($($field:ident=>$table:literal),*)=>{$(for v in &s.$field{insert(c,$table,&v.id,v)?;})*};}
    put!(epochs=>"schedule_epochs",manifests=>"study_manifests",protocol_events=>"study_protocol_events",opportunities=>"work_opportunities",assignments=>"study_assignments",outcomes=>"opportunity_outcomes",exposures=>"study_exposures",burdens=>"study_burdens",reflections=>"study_reflections",preferences=>"presentation_preferences");
    Ok(())
}
pub(crate) fn clear(c: &Connection) -> Result<(), CommandError> {
    db(c.execute_batch("DELETE FROM presentation_preferences;DELETE FROM study_reflections;DELETE FROM study_burdens;DELETE FROM study_exposures;DELETE FROM opportunity_outcomes;DELETE FROM study_assignments;DELETE FROM work_opportunities;DELETE FROM study_protocol_events;DELETE FROM study_manifests;DELETE FROM schedule_epochs;"))?;
    Ok(())
}

fn ledger_prefix(r: &RecordSet, sequence: u32, amount: u32) -> bool {
    r.progress.as_ref().is_some_and(|p| {
        sequence as usize <= p.ledger.len()
            && p.ledger
                .iter()
                .filter(|e| e.sequence <= sequence)
                .map(|e| e.amount_bc)
                .sum::<u32>()
                == amount
    })
}
pub(crate) fn validate(s: &StudyState, r: &RecordSet) -> Result<(), CommandError> {
    let mut ids = HashSet::new();
    macro_rules! identities {($($f:ident),*)=>{$(for row in &s.$f{valid_id(&row.id)?;if !ids.insert(row.id.as_str()){return Err(invalid("Study identities must be globally unique."));}})*};}
    identities!(
        epochs,
        manifests,
        protocol_events,
        opportunities,
        assignments,
        outcomes,
        exposures,
        burdens,
        reflections,
        preferences
    );
    for id in &ids {
        if r.projects.iter().any(|v| v.id == *id)
            || r.objectives.iter().any(|v| v.id == *id)
            || r.actions.iter().any(|v| v.id == *id)
            || r.notes.iter().any(|v| v.id == *id)
        {
            return Err(invalid("Study IDs collide with Work records."));
        }
    }
    if let Some(p) = &r.progress {
        macro_rules! distinct {($($f:ident),*)=>{$(if p.$f.iter().any(|v|ids.contains(v.id.as_str())){return Err(invalid("Study IDs collide with progress records."));})*};}
        distinct!(
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
    }
    let epochs: HashMap<_, _> = s.epochs.iter().map(|e| (e.id.as_str(), e)).collect();
    let manifests: HashMap<_, _> = s.manifests.iter().map(|m| (m.id.as_str(), m)).collect();
    let opportunities: HashMap<_, _> = s.opportunities.iter().map(|o| (o.id.as_str(), o)).collect();
    let mut outcome_groups: HashMap<&str, Vec<&OutcomeEvent>> =
        HashMap::with_capacity(s.opportunities.len());
    for outcome in &s.outcomes {
        outcome_groups
            .entry(outcome.opportunity_id.as_str())
            .or_default()
            .push(outcome);
    }
    for e in &s.epochs {
        text(&e.label, 120, "Epoch label")?;
        text(&e.timezone, 100, "Timezone")?;
        stamp(&e.effective_from)?;
        stamp(&e.created_at)?;
    }
    for m in &s.manifests {
        stamp(&m.created_at)?;
        if !epochs.contains_key(m.epoch_id.as_str())
            || m.protocol_version != "stage3c-v1"
            || m.window_seconds != 900
            || m.preview_seconds != 60
            || m.presentation_policy != "h1-ledger-64-v1"
            || m.analysis_definition != ANALYSIS
            || m.review_after != vec![30, 60]
            || m.reveal_rule
                != "preview: cue minus 60 seconds through cue; checkpoint: after observation window; no animation during work"
        {
            return Err(invalid(
                "A frozen study manifest is inconsistent or unsupported.",
            ));
        }
        if m.phase == Phase::E01 {
            valid_id(&m.seed)?;
            if m.block_size != 4
                || m.algorithm != "sha256-fy-v1"
                || m.allocation != allocation(&m.seed, 15)
            {
                return Err(invalid(
                    "The frozen balanced allocation does not match its seed and algorithm.",
                ));
            }
        } else if m.block_size != 0
            || m.algorithm != "none"
            || !m.seed.is_empty()
            || !m.allocation.is_empty()
        {
            return Err(invalid("E00 cannot contain randomized allocation."));
        }
        if m.allocation_hash != allocation_hash(&m.allocation)? {
            return Err(invalid("The allocation hash is inconsistent."));
        }
        if epochs[m.epoch_id.as_str()].created_at > m.created_at {
            return Err(invalid("A manifest predates its schedule epoch."));
        }
        let mut events: Vec<_> = s
            .protocol_events
            .iter()
            .filter(|e| e.manifest_id == m.id)
            .collect();
        events.sort_by_key(|e| e.sequence);
        let mut status = StudyStatus::Configured;
        let mut previous_at = m.created_at.as_str();
        for (index, e) in events.iter().enumerate() {
            stamp(&e.created_at)?;
            text(&e.reason, 2000, "Protocol reason")?;
            validate_acknowledgements(&e.acknowledgements)?;
            if e.created_at.as_str() < previous_at
                || e.sequence != index as u32 + 1
                || !valid_transition(status, e.status)
            {
                return Err(invalid("The protocol lifecycle history is inconsistent."));
            }
            if m.phase == Phase::E01
                && e.status == StudyStatus::Active
                && (status == StudyStatus::Configured
                    || e.acknowledgements.len() != ACKS.len()
                    || ACKS
                        .iter()
                        .any(|a| !e.acknowledgements.iter().any(|v| v == a)))
            {
                return Err(invalid(
                    "E01 activation is missing explicit readiness acknowledgements.",
                ));
            }
            status = e.status;
            previous_at = &e.created_at;
        }
    }
    if s.manifests
        .iter()
        .filter(|m| s.status(&m.id) == StudyStatus::Active)
        .count()
        > 1
    {
        return Err(invalid("Only one study can be active."));
    }
    let mut active_intervals: Vec<(&str, &str, Option<&str>)> = vec![];
    for m in &s.manifests {
        let mut events: Vec<_> = s
            .protocol_events
            .iter()
            .filter(|e| e.manifest_id == m.id)
            .collect();
        events.sort_by_key(|e| e.sequence);
        let mut start = None;
        for e in events {
            if e.status == StudyStatus::Active {
                start = Some(e.created_at.as_str());
            } else if let Some(at) = start.take() {
                active_intervals.push((&m.id, at, Some(&e.created_at)));
            }
        }
        if let Some(at) = start {
            active_intervals.push((&m.id, at, None));
        }
    }
    for (i, a) in active_intervals.iter().enumerate() {
        for b in active_intervals.iter().skip(i + 1) {
            if a.0 != b.0 && a.2.is_none_or(|end| b.1 < end) && b.2.is_none_or(|end| a.1 < end) {
                return Err(invalid("Historical active studies overlap."));
            }
        }
    }
    let mut preferences: Vec<_> = s.preferences.iter().collect();
    preferences.sort_by_key(|p| p.sequence);
    let mut last = "";
    for (index, p) in preferences.iter().enumerate() {
        stamp(&p.created_at)?;
        if p.sequence != index as u32 + 1 || p.created_at.as_str() < last {
            return Err(invalid("Presentation preference history is inconsistent."));
        }
        last = &p.created_at;
    }
    for e in &s.protocol_events {
        if !manifests.contains_key(e.manifest_id.as_str()) {
            return Err(invalid("A protocol event references a missing manifest."));
        }
    }
    for o in &s.opportunities {
        let objective = r
            .objectives
            .iter()
            .find(|p| p.id == o.objective_id)
            .ok_or_else(|| invalid("An opportunity objective is missing."))?;
        let action = r
            .actions
            .iter()
            .find(|a| a.id == o.action_id && a.objective_id == o.objective_id)
            .ok_or_else(|| invalid("An opportunity action is missing."))?;
        if action.revision == o.action_revision
            && (action.description != o.action_snapshot || action.stop_condition != o.stop_snapshot)
        {
            return Err(invalid(
                "The unchanged action revision contradicts its snapshot.",
            ));
        }
        let criterion = r
            .progress
            .as_ref()
            .and_then(|p| {
                p.criteria.iter().find(|c| {
                    c.id == o.criterion_id
                        && c.parent_id == o.objective_id
                        && c.criterion_version == o.criterion_version
                })
            })
            .ok_or_else(|| invalid("An opportunity criterion identity is inconsistent."))?;
        text(&o.criterion_snapshot, 2000, "Criterion snapshot")?;
        if o.criterion_registry_revision > criterion.registry_revision
            || ((o.criterion_registry_revision == criterion.registry_revision
                || criterion
                    .lock_registry_revision
                    .is_some_and(|v| v <= o.criterion_registry_revision))
                && o.criterion_snapshot != criterion.description)
        {
            return Err(invalid(
                "An opportunity criterion snapshot contradicts its declared revision.",
            ));
        }
        if objective.project_id != o.project_id
            || o.action_revision == 0
            || o.action_revision > action.revision
        {
            return Err(invalid(
                "An opportunity's Work references are inconsistent.",
            ));
        }
        text(&o.action_snapshot, 2000, "Action snapshot")?;
        text(&o.stop_snapshot, 2000, "Stop snapshot")?;
        text(&o.meaningful_start, 2000, "Meaningful start")?;
        text(&o.domain, 120, "Domain")?;
        text(&o.timezone, 100, "Timezone")?;
        let cue = stamp(&o.cue_at)?;
        if stamp(&o.created_at)? != stamp(&o.eligibility_decided_at)?
            || stamp(&o.created_at)? >= cue
            || !(60..=86400).contains(&o.window_seconds)
            || !(1..=1440).contains(&o.available_minutes)
        {
            return Err(invalid(
                "An opportunity was not prospectively locked with valid timing.",
            ));
        }
        let epoch = epochs
            .get(o.epoch_id.as_str())
            .ok_or_else(|| invalid("An opportunity epoch is missing."))?;
        if epoch.created_at > o.created_at {
            return Err(invalid("An opportunity predates its schedule epoch."));
        }
        if cue < stamp(&epoch.effective_from)? {
            return Err(invalid("An opportunity precedes its epoch."));
        }
        optional(&o.context, "Context")?;
        optional(&o.unavailability, "Unavailability")?;
        if o.eligible == o.unavailability.is_some() {
            return Err(invalid("Opportunity eligibility is inconsistent."));
        }
        if let Some(t) = &o.deadline_at {
            stamp(t)?;
        }
        if let Some(id) = &o.manifest_id {
            let event = s
                .protocol_events
                .iter()
                .find(|e| Some(&e.id) == o.protocol_event_id.as_ref() && &e.manifest_id == id)
                .ok_or_else(|| {
                    invalid("An opportunity is missing its active protocol decision.")
                })?;
            if event.status != StudyStatus::Active
                || event.created_at > o.created_at
                || s.protocol_events.iter().any(|e| {
                    e.manifest_id == *id
                        && e.sequence > event.sequence
                        && e.created_at < o.created_at
                })
            {
                return Err(invalid(
                    "The opportunity was not declared under an active protocol.",
                ));
            }
            let m = manifests
                .get(id.as_str())
                .ok_or_else(|| invalid("An opportunity protocol is missing."))?;
            if o.epoch_id != m.epoch_id || o.window_seconds != m.window_seconds {
                return Err(invalid("An opportunity violates the frozen protocol."));
            }
        } else if o.protocol_event_id.is_some() {
            return Err(invalid(
                "An ordinary opportunity cannot reference a protocol decision.",
            ));
        }
    }
    let mut assigned = HashSet::new();
    for m in &s.manifests {
        let mut assignments: Vec<_> = s
            .assignments
            .iter()
            .filter(|a| a.manifest_id == m.id)
            .collect();
        assignments.sort_by_key(|a| a.position);
        for pair in assignments.windows(2) {
            if pair[0].assigned_at > pair[1].assigned_at
                || pair[0].ledger_sequence > pair[1].ledger_sequence
                || pair[0].issued_bc_at_assignment > pair[1].issued_bc_at_assignment
            {
                return Err(invalid(
                    "Assignment sequence or issuance history moves backwards.",
                ));
            }
        }
        for (index, a) in assignments.iter().enumerate() {
            let o = opportunities
                .get(a.opportunity_id.as_str())
                .ok_or_else(|| invalid("An assignment opportunity is missing."))?;
            if !assigned.insert(o.id.as_str())
                || m.phase != Phase::E01
                || !o.eligible
                || o.manifest_id.as_ref() != Some(&m.id)
                || a.position != index as u32
                || m.allocation.get(index) != Some(&a.condition)
                || stamp(&a.assigned_at)? != stamp(&o.created_at)?
                || stamp(&a.assigned_at)? + chrono::Duration::seconds(60) >= stamp(&o.cue_at)?
                || a.h1_saturated != (a.issued_bc_at_assignment >= 64)
                || !ledger_prefix(r, a.ledger_sequence, a.issued_bc_at_assignment)
            {
                return Err(invalid("The immutable assignment history is inconsistent."));
            }
        }
    }
    for m in &s.manifests {
        if m.phase == Phase::E01 {
            let mut planned: Vec<_> = s
                .opportunities
                .iter()
                .filter(|o| o.manifest_id.as_ref() == Some(&m.id) && o.eligible)
                .collect();
            planned.sort_by(|a, b| a.cue_at.cmp(&b.cue_at));
            for pair in planned.windows(2) {
                if stamp(&pair[0].cue_at)?
                    + chrono::Duration::seconds(i64::from(pair[0].window_seconds))
                    > stamp(&pair[1].cue_at)? - chrono::Duration::seconds(60)
                {
                    return Err(invalid("E01 presentation windows overlap."));
                }
            }
        }
    }
    for a in &s.assignments {
        if !manifests.contains_key(a.manifest_id.as_str()) {
            return Err(invalid("An assignment protocol is missing."));
        }
    }
    for o in &s.opportunities {
        if o.eligible
            && o.manifest_id
                .as_ref()
                .and_then(|id| manifests.get(id.as_str()))
                .is_some_and(|m| m.phase == Phase::E01)
            && !assigned.contains(o.id.as_str())
        {
            return Err(invalid(
                "An eligible E01 opportunity is missing its original assignment.",
            ));
        }
        let mut events = outcome_groups.remove(o.id.as_str()).unwrap_or_default();
        events.sort_by_key(|e| e.sequence);
        let mut previous: Option<&OutcomeEvent> = None;
        for (index, e) in events.iter().enumerate() {
            optional(&e.note, "Factual note")?;
            optional(&e.quality_note, "Domain quality note")?;
            if e.acceptance_id.as_ref().is_some_and(|id| {
                !r.progress.as_ref().is_some_and(|p| {
                    p.acceptances
                        .iter()
                        .any(|a| &a.id == id && a.parent_id == o.objective_id)
                })
            }) {
                return Err(invalid(
                    "Outcome acceptance links must reference the same objective.",
                ));
            }
            optional(&e.uncertainty_reason, "Uncertainty")?;
            optional(&e.correction_reason, "Correction reason")?;
            if e.source != "local_user_self_report"
                || e.sequence != index as u32 + 1
                || e.supersedes.as_ref() != previous.map(|p| &p.id)
                || e.correction_reason.is_some() != previous.is_some()
                || previous.is_some_and(|p| p.observed_at > e.observed_at)
            {
                return Err(invalid("Outcome correction history is inconsistent."));
            }
            let mut expected = (*e).clone();
            classify(o, &mut expected)?;
            if expected != **e {
                return Err(invalid(
                    "An outcome classification contradicts its recorded timing.",
                ));
            }
            previous = Some(e);
        }
    }
    for e in &s.outcomes {
        if !opportunities.contains_key(e.opportunity_id.as_str()) {
            return Err(invalid("An outcome references a missing opportunity."));
        }
    }
    let mut deliveries = HashSet::new();
    for e in &s.exposures {
        stamp(&e.created_at)?;
        if e.preference_sequence as usize > s.preferences.len()
            || s.preferences.iter().any(|p| {
                if p.sequence <= e.preference_sequence {
                    p.created_at > e.created_at
                } else {
                    p.created_at < e.created_at
                }
            })
        {
            return Err(invalid(
                "An exposure's presentation preference prefix is inconsistent.",
            ));
        }
        if matches!(
            e.kind,
            ExposureKind::CueDelivered
                | ExposureKind::PreviewDelivered
                | ExposureKind::CheckpointDelivered
        ) && !deliveries.insert((e.opportunity_id.as_deref(), encode(&e.kind)?))
        {
            return Err(invalid(
                "An opportunity has duplicate delivery identities for one presentation boundary.",
            ));
        }
        if e.protocol_event_id.as_ref().is_some_and(|id| {
            !s.protocol_events
                .iter()
                .any(|p| &p.id == id && p.created_at <= e.created_at)
        }) {
            return Err(invalid(
                "An exposure protocol decision is missing or later than the exposure.",
            ));
        }
        check_opportunity(s, &e.opportunity_id)?;
        if matches!(
            e.kind,
            ExposureKind::PreviewDelivered | ExposureKind::CheckpointDelivered
        ) {
            let event = s
                .protocol_events
                .iter()
                .find(|p| Some(&p.id) == e.protocol_event_id.as_ref())
                .ok_or_else(|| {
                    invalid("A delivered study presentation lacks its active protocol decision.")
                })?;
            let o = opportunities
                .get(e.opportunity_id.as_deref().unwrap_or(""))
                .ok_or_else(|| invalid("A delivered study presentation lacks its opportunity."))?;
            if event.status != StudyStatus::Active
                || o.manifest_id.as_ref() != Some(&event.manifest_id)
                || event.created_at > e.created_at
                || s.protocol_events.iter().any(|p| {
                    p.manifest_id == event.manifest_id
                        && p.sequence > event.sequence
                        && p.created_at < e.created_at
                })
            {
                return Err(invalid(
                    "A construction exposure was not delivered under its active protocol.",
                ));
            }
            let mut historical = s.clone();
            historical.protocol_events.retain(|p| {
                if p.manifest_id == event.manifest_id {
                    p.sequence <= event.sequence
                } else {
                    p.created_at < e.created_at
                }
            });
            historical
                .preferences
                .retain(|p| p.sequence <= e.preference_sequence);
            let p = presentation(&historical, &e.created_at);
            let expected = if e.kind == ExposureKind::PreviewDelivered {
                "preview"
            } else {
                "checkpoint"
            };
            if !p.world_allowed || p.boundary != expected || p.opportunity_id != e.opportunity_id {
                return Err(invalid(
                    "A delivered construction exposure violates its historical boundary.",
                ));
            }
        }
        if e.kind == ExposureKind::CueDelivered {
            let o = opportunities
                .get(e.opportunity_id.as_deref().unwrap_or(""))
                .ok_or_else(|| invalid("Cue delivery requires an opportunity."))?;
            if e.created_at < o.cue_at {
                return Err(invalid("A delivered cue precedes its schedule."));
            }
        }
        if matches!(
            e.kind,
            ExposureKind::VoluntaryWorld | ExposureKind::Deviation
        ) && e.note.is_none()
        {
            return Err(invalid("An exposure override requires a reason."));
        }
        stamp(&e.created_at)?;
        if e.opportunity_id
            .as_ref()
            .and_then(|id| opportunities.get(id.as_str()))
            .is_some_and(|o| o.created_at > e.created_at)
        {
            return Err(invalid("An exposure predates its opportunity."));
        }
        optional(&e.note, "Exposure note")?;
        if e.h1_saturated != (e.issued_bc >= 64)
            || !ledger_prefix(r, e.ledger_sequence, e.issued_bc)
        {
            return Err(invalid("Exposure saturation history is inconsistent."));
        }
    }
    for e in &s.burdens {
        check_opportunity(s, &e.opportunity_id)?;
        stamp(&e.created_at)?;
        if e.opportunity_id
            .as_ref()
            .and_then(|id| opportunities.get(id.as_str()))
            .is_some_and(|o| o.created_at > e.created_at)
        {
            return Err(invalid("A burden observation predates its opportunity."));
        }
        optional(&e.note, "Burden note")?;
        if e.minutes > 1440 || e.method != "user_estimate_exclusive_minutes" {
            return Err(invalid(
                "Burden estimates must be exclusive, bounded minutes.",
            ));
        }
    }
    for e in &s.reflections {
        stamp(&e.created_at)?;
        text(&e.note, 2000, "Reflection")?;
        if e.manifest_id
            .as_ref()
            .and_then(|id| manifests.get(id.as_str()))
            .is_some_and(|m| m.created_at > e.created_at)
        {
            return Err(invalid("A reflection predates its manifest."));
        }
        if e.manifest_id
            .as_ref()
            .is_some_and(|id| !manifests.contains_key(id.as_str()))
        {
            return Err(invalid("A reflection protocol is missing."));
        }
    }
    Ok(())
}

pub(crate) fn set_construction_enabled(
    c: &Connection,
    i: SetConstructionEnabledInput,
) -> Result<(), CommandError> {
    generation(c, &i.expected_generation)?;
    let s = read(c)?;
    let previous = s.preferences.iter().max_by_key(|p| p.sequence);
    if previous.is_none_or(|p| p.enabled) == i.enabled {
        return Ok(());
    }
    let p = PresentationPreference {
        id: new_id(),
        sequence: previous.map_or(1, |p| p.sequence + 1),
        enabled: i.enabled,
        created_at: study_now(),
    };
    insert(c, "presentation_preferences", &p.id, &p)?;
    changed(c)
}
#[cfg(test)]
mod tests;
