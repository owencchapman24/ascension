//! Local, explicit progress acceptance. No client can register a trusted source,
//! choose an acceptance actor, or supply an issuance amount.
use crate::domain::{CommandError, MAX_RECORDS, Objective, new_id, now, text, valid_id};
use chrono::{DateTime, SecondsFormat, Utc};
use rusqlite::{Connection, OptionalExtension, params};
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, HashSet};
const MAX_REGISTRY_REVISION: u64 = 9_007_199_254_740_991;
fn supported_validator(validator: &ValidatorDefinition) -> bool {
    #[cfg(test)]
    {
        validator.source_provenance == "synthetic_registered_fixture"
    }
    #[cfg(not(test))]
    {
        let _ = validator;
        false
    }
}

macro_rules! model {
    ($name:ident { $($field:ident : $ty:ty),* $(,)? }) => {
        #[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
        #[serde(deny_unknown_fields)]
        pub struct $name { $(#[serde(deserialize_with="required_field")] pub $field: $ty,)* }
    };
}
fn required_field<'de, D: serde::Deserializer<'de>, T: Deserialize<'de>>(
    deserializer: D,
) -> Result<T, D::Error> {
    T::deserialize(deserializer)
}
macro_rules! enumeration {
    ($name:ident { $($variant:ident),* $(,)? }) => {
        #[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
        #[serde(rename_all = "snake_case")]
        pub enum $name { $($variant),* }
    };
}
enumeration!(CriterionState { Draft, Locked });
enumeration!(EvidencePolicy {
    UserAttestation,
    ReferencedEvidence,
    TrustedValidation
});
enumeration!(EvidenceKind {
    UserAttestation,
    SelfReport,
    ArtifactReference,
    ImportedMetadata,
    RegisteredValidator,
    ThirdPartyClaim
});
enumeration!(EvidenceStatus {
    Supports,
    Contradicts,
    Uncertain
});
enumeration!(Assistance {
    None,
    Assisted,
    Generated
});
enumeration!(ReviewDisposition {
    Pending,
    Accepted,
    Rejected
});
enumeration!(AcceptanceStatus {
    Accepted,
    Retracted,
    Historical
});
enumeration!(CorrectionKind {
    IncorrectAcceptance,
    HistoricalReopened
});

model!(RewardParent {
    parent_id: String,
    primary_lineage_id: String,
    reward_policy_version: String,
    lifetime_budget_bc: u32,
    provenance: String
});
model!(RewardSlot {
    id: String,
    parent_id: String,
    criterion_lineage_id: String,
    allocation_bc: u32,
    allocation_locked: bool
});
model!(Criterion {
    id:String, parent_id:String, criterion_lineage_id:String, criterion_version:u32,
    reward_slot_id:String, description:String, state:CriterionState, evidence_policy:EvidencePolicy,
    scope:String, artifact_id:Option<String>, artifact_version:Option<String>,
    requires_independent_work:bool, required_validators:Vec<String>, created_at:String,
    locked_at:Option<String>, lock_registry_revision:Option<u64>, registry_revision:u64
});
model!(ValidatorDefinition {
    id:String, parent_id:String, criterion_lineage_id:String, criterion_version:u32,
    artifact_id:Option<String>, artifact_version:Option<String>, scope:String,
    source_id:String, source_provenance:String, available:bool, registry_revision:u64
});
model!(Evidence {
    id:String, parent_id:String, criterion_lineage_id:String, criterion_version:u32,
    criterion_id:String, kind:EvidenceKind, source_id:String, source_provenance:String,
    artifact_id:Option<String>, artifact_version:Option<String>, scope:String,
    observed_at:String, recorded_at:String, registry_revision:u64, description:String,
    reference:Option<String>, status:EvidenceStatus, assistance:Assistance,
    uncertainty:Option<String>, supersedes:Option<String>, validator_id:Option<String>
});
model!(ReviewSnapshot {
    id:String, parent_id:String, criterion_lineage_id:String, criterion_version:u32,
    criterion_id:String, registry_revision:u64, criterion:Criterion,
    evidence_refs:Vec<String>, applicable_evidence:Vec<Evidence>, applicable_validators:Vec<ValidatorDefinition>,
    missing:Vec<String>, conflicts:Vec<String>, eligible:bool, factual_summary:String,
    assistance:Assistance, digest:String, reviewed_at:String, disposition:ReviewDisposition,
    rejection_reason:Option<String>
});
model!(Acceptance {
    id:String, parent_id:String, criterion_lineage_id:String, criterion_version:u32,
    criterion_id:String, reward_slot_id:String, snapshot_id:String, actor:String,
    method:String, assistance:Assistance, factual_summary:String, accepted_at:String,
    status:AcceptanceStatus, correction_id:Option<String>
});
model!(Correction {
    id: String,
    acceptance_id: String,
    registry_revision: u64,
    kind: CorrectionKind,
    reason: String,
    created_at: String
});
model!(RewardGrant {
    id: String,
    parent_id: String,
    reward_slot_id: String,
    criterion_lineage_id: String,
    acceptance_id: String,
    reward_policy_version: String,
    amount_bc: u32,
    created_at: String
});
model!(LedgerEntry {
    id: String,
    grant_id: String,
    sequence: u32,
    amount_bc: u32,
    created_at: String
});
model!(AuditEvent {
    id: String,
    kind: String,
    subject_id: String,
    actor: String,
    created_at: String
});
model!(SettlementResult { acceptance_id:String, grant_id:Option<String>, issued_bc:u32, balance_bc:u32, retained_reward:bool });
model!(CommittedCommand {
    id: String,
    ledger_sequence: u32,
    payload_digest: String,
    payload: AcceptProgressInput,
    result: SettlementResult
});

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ProgressState {
    pub registry_revision: u64,
    pub parents: Vec<RewardParent>,
    pub slots: Vec<RewardSlot>,
    pub criteria: Vec<Criterion>,
    pub evidence: Vec<Evidence>,
    pub validators: Vec<ValidatorDefinition>,
    pub snapshots: Vec<ReviewSnapshot>,
    pub acceptances: Vec<Acceptance>,
    pub corrections: Vec<Correction>,
    pub grants: Vec<RewardGrant>,
    pub ledger: Vec<LedgerEntry>,
    pub audit: Vec<AuditEvent>,
    pub commands: Vec<CommittedCommand>,
    pub wallet_bc: u32,
}

model!(SaveCriterionInput {
    expected_generation:String, parent_id:String, criterion_lineage_id:Option<String>, description:String,
    allocation_bc:u32, evidence_policy:EvidencePolicy, scope:String,
    artifact_id:Option<String>, artifact_version:Option<String>, requires_independent_work:bool,
    required_validators:Vec<String>
});
model!(LockCriterionInput {
    expected_generation: String,
    parent_id: String,
    criterion_lineage_id: String,
    criterion_version: u32
});
model!(RecordEvidenceInput {
    expected_generation:String, parent_id:String, criterion_lineage_id:String, criterion_version:u32,
    kind:EvidenceKind, description:String, reference:Option<String>, observed_at:Option<String>,
    status:EvidenceStatus, assistance:Assistance, uncertainty:Option<String>, supersedes:Option<String>
});
model!(ReviewProgressInput { expected_generation:String, parent_id:String, criterion_lineage_id:String, criterion_version:u32, evidence_refs:Vec<String>, factual_summary:String, assistance:Assistance });
model!(AcceptProgressInput {
    schema_version:u32, command_id:String, expected_generation:String, parent_id:String,
    criterion_lineage_id:String, criterion_version:u32, expected_parent_revision:u32,
    reviewed_snapshot_id:String, reviewed_snapshot_digest:String, evidence_refs:Vec<String>,
    factual_summary:String, assistance:Assistance
});
model!(CorrectAcceptanceInput {
    expected_generation: String,
    acceptance_id: String,
    kind: CorrectionKind,
    reason: String
});
model!(RejectReviewInput {
    expected_generation: String,
    snapshot_id: String,
    reason: String
});

fn db<T>(value: rusqlite::Result<T>) -> Result<T, CommandError> {
    value.map_err(|_| CommandError::storage())
}
fn invalid(message: &str) -> CommandError {
    CommandError::invalid(message)
}
fn encode<T: Serialize>(value: &T) -> Result<String, CommandError> {
    serde_json::to_string(value).map_err(|_| CommandError::storage())
}
fn hash<T: Serialize>(value: &T) -> Result<String, CommandError> {
    Ok(format!("{:x}", Sha256::digest(encode(value)?.as_bytes())))
}
fn timestamp(value: &str) -> Result<(), CommandError> {
    let parsed = DateTime::parse_from_rfc3339(value)
        .map_err(|_| invalid("A progress timestamp is invalid."))?;
    if parsed
        .with_timezone(&Utc)
        .to_rfc3339_opts(SecondsFormat::Millis, true)
        != value
    {
        return Err(invalid(
            "Progress timestamps require UTC millisecond precision.",
        ));
    }
    Ok(())
}
fn optional_text(value: &Option<String>, max: usize, field: &str) -> Result<(), CommandError> {
    if let Some(value) = value {
        text(value, max, field)?;
    }
    Ok(())
}
fn artifact(id: &Option<String>, version: &Option<String>) -> Result<(), CommandError> {
    optional_text(id, 500, "Artifact identity")?;
    optional_text(version, 200, "Artifact version")?;
    if id.is_some() != version.is_some() {
        return Err(invalid(
            "Artifact identity and version must be supplied together.",
        ));
    }
    Ok(())
}
fn refs(ids: &[String]) -> Result<(), CommandError> {
    if ids.len() > 100 {
        return Err(invalid(
            "A review may reference at most 100 evidence records.",
        ));
    }
    let mut unique = HashSet::new();
    for id in ids {
        valid_id(id)?;
        if !unique.insert(id) {
            return Err(invalid(
                "Duplicate evidence or validator references are not allowed.",
            ));
        }
    }
    Ok(())
}
fn generation(connection: &Connection, expected: &str) -> Result<(), CommandError> {
    valid_id(expected)?;
    let current: String = db(connection.query_row(
        "SELECT generation FROM workspace_state WHERE singleton=1",
        [],
        |row| row.get(0),
    ))?;
    if current != expected {
        return Err(CommandError::conflict());
    }
    Ok(())
}
fn changed(connection: &Connection) -> Result<(), CommandError> {
    db(connection.execute(
        "UPDATE workspace_state SET generation=?1 WHERE singleton=1",
        [new_id()],
    ))?;
    Ok(())
}
fn next_registry(connection: &Connection) -> Result<u64, CommandError> {
    let current: i64 = db(connection.query_row(
        "SELECT registry_revision FROM progress_state WHERE singleton=1",
        [],
        |row| row.get(0),
    ))?;
    let next = u64::try_from(current)
        .ok()
        .and_then(|n| n.checked_add(1))
        .filter(|n| *n <= MAX_REGISTRY_REVISION)
        .ok_or_else(|| invalid("The evidence registry reached its revision limit."))?;
    db(connection.execute(
        "UPDATE progress_state SET registry_revision=?1 WHERE singleton=1",
        [next as i64],
    ))?;
    Ok(next)
}
fn insert<T: Serialize>(
    connection: &Connection,
    table: &str,
    id: &str,
    value: &T,
) -> Result<(), CommandError> {
    db(connection.execute(
        &format!("INSERT INTO {table}(id,payload) VALUES(?1,?2)"),
        params![id, encode(value)?],
    ))?;
    Ok(())
}
fn update<T: Serialize>(
    connection: &Connection,
    table: &str,
    id: &str,
    value: &T,
) -> Result<(), CommandError> {
    if db(connection.execute(
        &format!("UPDATE {table} SET payload=?2 WHERE id=?1"),
        params![id, encode(value)?],
    ))? != 1
    {
        return Err(CommandError::storage());
    }
    Ok(())
}
fn rows<T: DeserializeOwned>(connection: &Connection, table: &str) -> Result<Vec<T>, CommandError> {
    let mut query = db(connection.prepare(&format!("SELECT payload FROM {table} ORDER BY id")))?;
    let values = db(query.query_map([], |row| row.get::<_, String>(0)))?;
    values
        .map(|value| serde_json::from_str(&db(value)?).map_err(|_| CommandError::storage()))
        .collect()
}
pub(crate) fn read(connection: &Connection) -> Result<ProgressState, CommandError> {
    Ok(ProgressState {
        registry_revision: db(connection.query_row(
            "SELECT registry_revision FROM progress_state WHERE singleton=1",
            [],
            |row| row.get::<_, i64>(0),
        ))?
        .try_into()
        .map_err(|_| CommandError::storage())?,
        parents: rows(connection, "progress_parents")?,
        slots: rows(connection, "reward_slots")?,
        criteria: rows(connection, "criterion_versions")?,
        evidence: rows(connection, "evidence_records")?,
        validators: rows(connection, "validator_definitions")?,
        snapshots: rows(connection, "review_snapshots")?,
        acceptances: rows(connection, "acceptances")?,
        corrections: rows(connection, "corrections")?,
        grants: rows(connection, "reward_grants")?,
        ledger: rows(connection, "ledger_entries")?,
        audit: rows(connection, "progress_audit")?,
        commands: rows(connection, "committed_commands")?,
        wallet_bc: db(connection.query_row(
            "SELECT COALESCE(SUM(amount_bc),0) FROM ledger_entries",
            [],
            |row| row.get(0),
        ))?,
    })
}
fn stable_id(parent: &str, kind: &str) -> String {
    let digest = Sha256::digest(format!("ascension-stage3b:{kind}:{parent}").as_bytes());
    let mut bytes = [0u8; 16];
    bytes.copy_from_slice(&digest[..16]);
    bytes[6] = (bytes[6] & 0x0f) | 0x50;
    bytes[8] = (bytes[8] & 0x3f) | 0x80;
    uuid::Uuid::from_bytes(bytes).to_string()
}
pub(crate) fn from_legacy(objectives: &[Objective]) -> ProgressState {
    let mut state = ProgressState::default();
    for objective in objectives {
        let lineage = stable_id(&objective.id, "lineage");
        let slot = stable_id(&objective.id, "slot");
        state.parents.push(RewardParent {
            parent_id: objective.id.clone(),
            primary_lineage_id: lineage.clone(),
            reward_policy_version: "bc-v1".into(),
            lifetime_budget_bc: 4,
            provenance: "legacy_stage3a".into(),
        });
        state.slots.push(RewardSlot {
            id: slot.clone(),
            parent_id: objective.id.clone(),
            criterion_lineage_id: lineage.clone(),
            allocation_bc: 4,
            allocation_locked: false,
        });
        state.criteria.push(Criterion {
            id: stable_id(&objective.id, "criterion-v1"),
            parent_id: objective.id.clone(),
            criterion_lineage_id: lineage,
            criterion_version: 1,
            reward_slot_id: slot,
            description: objective.criterion.clone(),
            state: CriterionState::Draft,
            evidence_policy: EvidencePolicy::UserAttestation,
            scope: "Bounded parent outcome".into(),
            artifact_id: None,
            artifact_version: None,
            requires_independent_work: false,
            required_validators: vec![],
            created_at: objective.created_at.clone(),
            locked_at: None,
            lock_registry_revision: None,
            registry_revision: 0,
        });
    }
    state.normalize();
    state
}
impl ProgressState {
    pub fn normalize(&mut self) {
        self.parents.sort_by(|a, b| a.parent_id.cmp(&b.parent_id));
        macro_rules! sort {($($field:ident),*)=>{$(self.$field.sort_by(|a,b|a.id.cmp(&b.id));)*};}
        sort!(
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
    pub fn record_count(&self) -> usize {
        self.parents.len()
            + self.slots.len()
            + self.criteria.len()
            + self.evidence.len()
            + self.validators.len()
            + self.snapshots.len()
            + self.acceptances.len()
            + self.corrections.len()
            + self.grants.len()
            + self.ledger.len()
            + self.audit.len()
            + self.commands.len()
    }
}
pub(crate) fn migrate_legacy(connection: &Connection) -> Result<(), CommandError> {
    let mut statement=db(connection.prepare("SELECT id,project_id,title,criterion,revision,created_at,updated_at FROM objectives ORDER BY id"))?;
    let objectives = db(statement.query_map([], |row| {
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
    let objectives = db(objectives.collect::<rusqlite::Result<Vec<_>>>())?;
    restore(connection, &from_legacy(&objectives))
}
pub(crate) fn sync_objectives(
    connection: &Connection,
    before: &[Objective],
    after: &[Objective],
) -> Result<(), CommandError> {
    for objective in after {
        if let Some(previous) = before.iter().find(|old| old.id == objective.id) {
            if previous.criterion != objective.criterion {
                let state = read(connection)?;
                let parent = state
                    .parents
                    .iter()
                    .find(|p| p.parent_id == objective.id)
                    .ok_or_else(CommandError::storage)?;
                let mut criterion =
                    current(&state, &objective.id, &parent.primary_lineage_id)?.clone();
                if criterion.state != CriterionState::Draft {
                    return Err(invalid(
                        "This criterion is locked. Create a prospective criterion version in Progress.",
                    ));
                }
                criterion.description = objective.criterion.clone();
                criterion.registry_revision = next_registry(connection)?;
                update(connection, "criterion_versions", &criterion.id, &criterion)?;
            }
        } else {
            let mut initial = from_legacy(std::slice::from_ref(objective));
            initial.parents[0].provenance = "local_created".into();
            initial.criteria[0].registry_revision = next_registry(connection)?;
            insert(
                connection,
                "progress_parents",
                &initial.parents[0].parent_id,
                &initial.parents[0],
            )?;
            insert(
                connection,
                "reward_slots",
                &initial.slots[0].id,
                &initial.slots[0],
            )?;
            insert(
                connection,
                "criterion_versions",
                &initial.criteria[0].id,
                &initial.criteria[0],
            )?;
        }
    }
    Ok(())
}
pub(crate) fn clear(connection: &Connection) -> Result<(), CommandError> {
    // The caller owns the complete restore transaction; no ordinary edit uses this.
    db(connection.execute_batch("PRAGMA defer_foreign_keys=ON; DELETE FROM committed_commands; DELETE FROM progress_audit; DELETE FROM ledger_entries; DELETE FROM reward_grants; DELETE FROM corrections; DELETE FROM acceptances; DELETE FROM review_snapshots; DELETE FROM evidence_records; DELETE FROM validator_definitions; DELETE FROM criterion_versions; DELETE FROM reward_slots; DELETE FROM progress_parents; UPDATE progress_state SET registry_revision=0 WHERE singleton=1;"))?;
    Ok(())
}
pub(crate) fn restore(connection: &Connection, state: &ProgressState) -> Result<(), CommandError> {
    db(connection.execute(
        "UPDATE progress_state SET registry_revision=?1 WHERE singleton=1",
        [state.registry_revision as i64],
    ))?;
    for value in &state.parents {
        insert(connection, "progress_parents", &value.parent_id, value)?;
    }
    macro_rules! put {
        ($field:ident,$table:literal) => {
            for value in &state.$field {
                insert(connection, $table, &value.id, value)?;
            }
        };
    }
    put!(slots, "reward_slots");
    put!(criteria, "criterion_versions");
    put!(validators, "validator_definitions");
    let mut evidence = state.evidence.iter().collect::<Vec<_>>();
    evidence.sort_by_key(|v| v.registry_revision);
    for value in evidence {
        insert(connection, "evidence_records", &value.id, value)?;
    }
    put!(snapshots, "review_snapshots");
    // Restored corrected grants retain the original eligible acceptance event.
    let mut acceptances = state.acceptances.iter().collect::<Vec<_>>();
    acceptances.sort_by_key(|a| a.status == AcceptanceStatus::Accepted);
    for value in acceptances {
        let mut original = value.clone();
        original.status = AcceptanceStatus::Accepted;
        original.correction_id = None;
        insert(connection, "acceptances", &value.id, &original)?;
        for grant in state.grants.iter().filter(|g| g.acceptance_id == value.id) {
            insert(connection, "reward_grants", &grant.id, grant)?;
        }
        if value.status != AcceptanceStatus::Accepted {
            update(connection, "acceptances", &value.id, value)?;
        }
    }
    put!(corrections, "corrections");
    put!(ledger, "ledger_entries");
    put!(audit, "progress_audit");
    put!(commands, "committed_commands");
    Ok(())
}
fn current<'a>(
    state: &'a ProgressState,
    parent: &str,
    lineage: &str,
) -> Result<&'a Criterion, CommandError> {
    valid_id(parent)?;
    valid_id(lineage)?;
    state
        .criteria
        .iter()
        .filter(|c| c.parent_id == parent && c.criterion_lineage_id == lineage)
        .max_by_key(|c| c.criterion_version)
        .ok_or_else(|| invalid("The criterion does not belong to this objective."))
}
fn exact<'a>(
    state: &'a ProgressState,
    parent: &str,
    lineage: &str,
    version: u32,
) -> Result<&'a Criterion, CommandError> {
    let criterion = current(state, parent, lineage)?;
    if criterion.criterion_version != version {
        return Err(CommandError::new(
            "stale_review",
            "The criterion changed. Review its current version.",
        ));
    }
    Ok(criterion)
}
fn audit(connection: &Connection, kind: &str, subject_id: &str) -> Result<(), CommandError> {
    let event = AuditEvent {
        id: new_id(),
        kind: kind.into(),
        subject_id: subject_id.into(),
        actor: "local_user".into(),
        created_at: now(),
    };
    insert(connection, "progress_audit", &event.id, &event)
}
pub(crate) fn save_criterion(
    connection: &Connection,
    input: SaveCriterionInput,
) -> Result<(), CommandError> {
    generation(connection, &input.expected_generation)?;
    valid_id(&input.parent_id)?;
    text(&input.description, 2000, "Criterion")?;
    text(&input.scope, 500, "Scope")?;
    artifact(&input.artifact_id, &input.artifact_version)?;
    refs(&input.required_validators)?;
    if !(1..=4).contains(&input.allocation_bc) {
        return Err(invalid(
            "Each allocation must be an integer from one through four.",
        ));
    }
    if input.evidence_policy != EvidencePolicy::TrustedValidation
        && !input.required_validators.is_empty()
    {
        return Err(invalid(
            "Required validators need the trusted-validation evidence policy.",
        ));
    }
    let state = read(connection)?;
    let parent = state
        .parents
        .iter()
        .find(|p| p.parent_id == input.parent_id)
        .ok_or_else(|| invalid("The objective does not exist."))?;
    let existing = input
        .criterion_lineage_id
        .as_ref()
        .map(|lineage| current(&state, &input.parent_id, lineage))
        .transpose()?;
    let old_slot = existing.and_then(|criterion| {
        state
            .slots
            .iter()
            .find(|slot| slot.id == criterion.reward_slot_id)
    });
    if old_slot
        .is_some_and(|slot| slot.allocation_locked && slot.allocation_bc != input.allocation_bc)
    {
        return Err(invalid(
            "A locked reward allocation cannot be renewed or changed.",
        ));
    }
    let allocated: u32 = state
        .slots
        .iter()
        .filter(|slot| {
            slot.parent_id == input.parent_id
                && Some(slot.id.as_str()) != old_slot.map(|s| s.id.as_str())
        })
        .map(|slot| slot.allocation_bc)
        .sum();
    if allocated + input.allocation_bc > 4 {
        return Err(invalid(
            "The parent's lifetime allocations may not exceed four Build Credits.",
        ));
    }
    let lineage = input.criterion_lineage_id.clone().unwrap_or_else(new_id);
    let slot_id = old_slot.map(|slot| slot.id.clone()).unwrap_or_else(new_id);
    let version = if let Some(old) = existing {
        if old.state == CriterionState::Draft {
            old.criterion_version
        } else {
            old.criterion_version
                .checked_add(1)
                .ok_or_else(|| invalid("Criterion version limit reached."))?
        }
    } else {
        1
    };
    let mut required = input.required_validators;
    required.sort();
    let criterion = Criterion {
        id: existing
            .filter(|old| old.state == CriterionState::Draft)
            .map(|old| old.id.clone())
            .unwrap_or_else(new_id),
        parent_id: input.parent_id.clone(),
        criterion_lineage_id: lineage.clone(),
        criterion_version: version,
        reward_slot_id: slot_id.clone(),
        description: input.description,
        state: CriterionState::Draft,
        evidence_policy: input.evidence_policy,
        scope: input.scope,
        artifact_id: input.artifact_id,
        artifact_version: input.artifact_version,
        requires_independent_work: input.requires_independent_work,
        required_validators: required,
        created_at: existing
            .filter(|old| old.state == CriterionState::Draft)
            .map(|old| old.created_at.clone())
            .unwrap_or_else(now),
        locked_at: None,
        lock_registry_revision: None,
        registry_revision: next_registry(connection)?,
    };
    let slot = RewardSlot {
        id: slot_id,
        parent_id: input.parent_id.clone(),
        criterion_lineage_id: lineage.clone(),
        allocation_bc: input.allocation_bc,
        allocation_locked: old_slot.is_some_and(|s| s.allocation_locked),
    };
    match old_slot {
        Some(old) if old != &slot => update(connection, "reward_slots", &slot.id, &slot)?,
        None => insert(connection, "reward_slots", &slot.id, &slot)?,
        _ => (),
    }
    if existing.is_some_and(|old| old.state == CriterionState::Draft) {
        update(connection, "criterion_versions", &criterion.id, &criterion)?;
    } else {
        insert(connection, "criterion_versions", &criterion.id, &criterion)?;
    }
    if parent.primary_lineage_id == lineage
        && existing.is_some_and(|old| old.description != criterion.description)
    {
        db(connection.execute(
            "UPDATE objectives SET revision=revision+1,updated_at=?2 WHERE id=?1",
            params![input.parent_id, now()],
        ))?;
    }
    audit(connection, "criterion_saved", &criterion.id)?;
    changed(connection)
}
pub(crate) fn lock_criterion(
    connection: &Connection,
    input: LockCriterionInput,
) -> Result<(), CommandError> {
    generation(connection, &input.expected_generation)?;
    let state = read(connection)?;
    let mut criterion = exact(
        &state,
        &input.parent_id,
        &input.criterion_lineage_id,
        input.criterion_version,
    )?
    .clone();
    if criterion.state == CriterionState::Locked {
        return Ok(());
    }
    let mut slot = state
        .slots
        .iter()
        .find(|slot| slot.id == criterion.reward_slot_id)
        .ok_or_else(CommandError::storage)?
        .clone();
    if !slot.allocation_locked {
        slot.allocation_locked = true;
        update(connection, "reward_slots", &slot.id, &slot)?;
    }
    let revision = next_registry(connection)?;
    criterion.state = CriterionState::Locked;
    criterion.locked_at = Some(now());
    criterion.lock_registry_revision = Some(revision);
    criterion.registry_revision = revision;
    update(connection, "criterion_versions", &criterion.id, &criterion)?;
    audit(connection, "criterion_locked", &criterion.id)?;
    changed(connection)
}
pub(crate) fn record_evidence(
    connection: &Connection,
    input: RecordEvidenceInput,
) -> Result<(), CommandError> {
    generation(connection, &input.expected_generation)?;
    if input.kind == EvidenceKind::RegisteredValidator {
        return Err(CommandError::new(
            "unavailable_authority",
            "No trusted validator registration route is available in this application.",
        ));
    }
    text(&input.description, 2000, "Factual progress")?;
    optional_text(&input.reference, 2000, "Evidence reference")?;
    optional_text(&input.uncertainty, 2000, "Uncertainty")?;
    if input.kind == EvidenceKind::ArtifactReference && input.reference.is_none() {
        return Err(invalid(
            "Referenced evidence needs a user-selected reference.",
        ));
    }
    let state = read(connection)?;
    let criterion = exact(
        &state,
        &input.parent_id,
        &input.criterion_lineage_id,
        input.criterion_version,
    )?;
    if let Some(id) = &input.supersedes {
        valid_id(id)?;
        let previous = state
            .evidence
            .iter()
            .find(|e| &e.id == id)
            .ok_or_else(|| invalid("The superseded evidence does not exist."))?;
        if previous.criterion_id != criterion.id
            || previous.source_id != "local_user"
            || previous.kind == EvidenceKind::RegisteredValidator
            || state
                .evidence
                .iter()
                .any(|e| e.supersedes.as_ref() == Some(id))
        {
            return Err(invalid(
                "Only current user-reported evidence for this exact criterion can be superseded.",
            ));
        }
    }
    let observed = input.observed_at.unwrap_or_else(now);
    timestamp(&observed)?;
    let evidence = Evidence {
        id: new_id(),
        parent_id: criterion.parent_id.clone(),
        criterion_lineage_id: criterion.criterion_lineage_id.clone(),
        criterion_version: criterion.criterion_version,
        criterion_id: criterion.id.clone(),
        kind: input.kind.clone(),
        source_id: "local_user".into(),
        source_provenance: if matches!(
            input.kind,
            EvidenceKind::ImportedMetadata | EvidenceKind::ThirdPartyClaim
        ) {
            "user_entered_unverified"
        } else {
            "local_user_report"
        }
        .into(),
        artifact_id: criterion.artifact_id.clone(),
        artifact_version: criterion.artifact_version.clone(),
        scope: criterion.scope.clone(),
        observed_at: observed,
        recorded_at: now(),
        registry_revision: next_registry(connection)?,
        description: input.description,
        reference: input.reference,
        status: input.status,
        assistance: input.assistance,
        uncertainty: input.uncertainty,
        supersedes: input.supersedes,
        validator_id: None,
    };
    insert(connection, "evidence_records", &evidence.id, &evidence)?;
    audit(connection, "evidence_recorded", &evidence.id)?;
    changed(connection)
}
fn snapshot_digest(snapshot: &ReviewSnapshot) -> Result<String, CommandError> {
    // Review identity/time/disposition do not alter its factual evidence basis.
    let mut canonical = snapshot.clone();
    canonical.id.clear();
    canonical.digest.clear();
    canonical.reviewed_at.clear();
    canonical.disposition = ReviewDisposition::Pending;
    canonical.rejection_reason = None;
    hash(&canonical)
}
fn applicable(evidence: &Evidence, criterion: &Criterion) -> bool {
    evidence.parent_id == criterion.parent_id
        && evidence.criterion_lineage_id == criterion.criterion_lineage_id
        && evidence.criterion_version == criterion.criterion_version
        && evidence.criterion_id == criterion.id
        && evidence.artifact_id == criterion.artifact_id
        && evidence.artifact_version == criterion.artifact_version
        && evidence.scope == criterion.scope
}
fn validator_applicable(validator: &ValidatorDefinition, criterion: &Criterion) -> bool {
    validator.parent_id == criterion.parent_id
        && validator.criterion_lineage_id == criterion.criterion_lineage_id
        && validator.criterion_version == criterion.criterion_version
        && validator.artifact_id == criterion.artifact_id
        && validator.artifact_version == criterion.artifact_version
        && validator.scope == criterion.scope
}
fn evaluate(
    state: &ProgressState,
    criterion: &Criterion,
    ids: &[String],
    summary: &str,
    assistance: &Assistance,
    revision: u64,
) -> Result<ReviewSnapshot, CommandError> {
    refs(ids)?;
    text(summary, 2000, "Factual summary")?;
    let superseded = state
        .evidence
        .iter()
        .filter(|e| e.registry_revision <= revision)
        .filter_map(|e| e.supersedes.as_ref())
        .collect::<HashSet<_>>();
    let mut evidence = state
        .evidence
        .iter()
        .filter(|e| {
            e.registry_revision <= revision
                && applicable(e, criterion)
                && !superseded.contains(&e.id)
        })
        .cloned()
        .collect::<Vec<_>>();
    evidence.sort_by(|a, b| a.id.cmp(&b.id));
    let mut validators = state
        .validators
        .iter()
        .filter(|v| v.registry_revision <= revision && validator_applicable(v, criterion))
        .cloned()
        .collect::<Vec<_>>();
    validators.sort_by(|a, b| a.id.cmp(&b.id));
    let mut selected = Vec::new();
    for id in ids {
        let record = state
            .evidence
            .iter()
            .find(|e| &e.id == id)
            .ok_or_else(|| invalid("An evidence reference does not exist."))?;
        if !applicable(record, criterion)
            || record.registry_revision > revision
            || superseded.contains(&record.id)
        {
            return Err(invalid(
                "An evidence reference is superseded or belongs to a different criterion, artifact, or scope.",
            ));
        }
        selected.push(record);
    }
    let mut missing = Vec::new();
    let mut conflicts = Vec::new();
    if criterion.state != CriterionState::Locked {
        missing.push("The criterion must be locked prospectively.".into());
    }
    let prospective = |e: &Evidence| {
        criterion
            .lock_registry_revision
            .is_some_and(|r| e.registry_revision > r)
            && criterion
                .locked_at
                .as_ref()
                .is_some_and(|locked| &e.observed_at >= locked)
            && e.observed_at <= e.recorded_at
    };
    for record in &evidence {
        if record.status == EvidenceStatus::Contradicts {
            conflicts.push(format!(
                "Current evidence {} contradicts this criterion.",
                record.id
            ));
        }
    }
    if criterion.requires_independent_work
        && (*assistance != Assistance::None
            || evidence.iter().any(|e| e.assistance != Assistance::None))
    {
        conflicts.push(
            "This criterion requires independent human work; declared assistance is incompatible."
                .into(),
        );
    }
    match criterion.evidence_policy {
        EvidencePolicy::UserAttestation => {
            if !selected.iter().any(|e| {
                matches!(
                    e.kind,
                    EvidenceKind::UserAttestation | EvidenceKind::SelfReport
                ) && e.status == EvidenceStatus::Supports
                    && prospective(e)
            }) {
                missing.push("Record a supporting personal attestation observed after this criterion was locked.".into());
            }
        }
        EvidencePolicy::ReferencedEvidence => {
            if !selected.iter().any(|e| {
                e.kind == EvidenceKind::ArtifactReference
                    && e.reference.is_some()
                    && e.status == EvidenceStatus::Supports
                    && prospective(e)
            }) {
                missing.push("Review a supporting artifact reference observed after the criterion was locked.".into());
            }
        }
        EvidencePolicy::TrustedValidation => {
            if criterion.required_validators.is_empty() {
                missing.push(
                    "No trusted validator is registered for this required-validation policy."
                        .into(),
                );
            }
            for required in &criterion.required_validators {
                let definition = validators.iter().find(|v| &v.id == required && v.available);
                let results = evidence
                    .iter()
                    .filter(|e| {
                        e.validator_id.as_ref() == Some(required)
                            && e.kind == EvidenceKind::RegisteredValidator
                            && prospective(e)
                    })
                    .collect::<Vec<_>>();
                if definition.is_none()
                    || !results.iter().any(|e| e.status == EvidenceStatus::Supports)
                    || results
                        .iter()
                        .any(|e| e.status == EvidenceStatus::Uncertain)
                {
                    missing.push(format!("Required validator {required} has no current trustworthy supporting result."));
                }
            }
        }
    }
    // All applicable registered results are examined, including IDs omitted by the claimant.
    for validator in &validators {
        let current = evidence
            .iter()
            .filter(|e| e.validator_id.as_ref() == Some(&validator.id))
            .collect::<Vec<_>>();
        if current.iter().any(|e| e.status == EvidenceStatus::Supports)
            && current
                .iter()
                .any(|e| e.status == EvidenceStatus::Contradicts)
        {
            conflicts.push(format!(
                "Registered validator {} has conflicting current results.",
                validator.id
            ));
        }
    }
    let mut evidence_refs = ids.to_vec();
    evidence_refs.sort();
    let mut result = ReviewSnapshot {
        id: new_id(),
        parent_id: criterion.parent_id.clone(),
        criterion_lineage_id: criterion.criterion_lineage_id.clone(),
        criterion_version: criterion.criterion_version,
        criterion_id: criterion.id.clone(),
        registry_revision: revision,
        criterion: criterion.clone(),
        evidence_refs,
        applicable_evidence: evidence,
        applicable_validators: validators,
        eligible: missing.is_empty() && conflicts.is_empty(),
        missing,
        conflicts,
        factual_summary: summary.into(),
        assistance: assistance.clone(),
        digest: String::new(),
        reviewed_at: now(),
        disposition: ReviewDisposition::Pending,
        rejection_reason: None,
    };
    result.digest = snapshot_digest(&result)?;
    Ok(result)
}
pub(crate) fn review(
    connection: &Connection,
    input: ReviewProgressInput,
) -> Result<ReviewSnapshot, CommandError> {
    generation(connection, &input.expected_generation)?;
    let state = read(connection)?;
    let criterion = exact(
        &state,
        &input.parent_id,
        &input.criterion_lineage_id,
        input.criterion_version,
    )?;
    if criterion.state != CriterionState::Locked {
        return Err(invalid(
            "Lock the criterion before requesting an accomplishment review.",
        ));
    }
    let snapshot = evaluate(
        &state,
        criterion,
        &input.evidence_refs,
        &input.factual_summary,
        &input.assistance,
        state.registry_revision,
    )?;
    insert(connection, "review_snapshots", &snapshot.id, &snapshot)?;
    audit(connection, "review_requested", &snapshot.id)?;
    changed(connection)?;
    Ok(snapshot)
}
fn canonical_command(mut input: AcceptProgressInput) -> Result<AcceptProgressInput, CommandError> {
    if input.schema_version != 1 {
        return Err(CommandError::new(
            "unsupported_schema",
            "This acceptance command version is unsupported.",
        ));
    }
    for id in [
        &input.command_id,
        &input.expected_generation,
        &input.parent_id,
        &input.criterion_lineage_id,
        &input.reviewed_snapshot_id,
    ] {
        valid_id(id)?;
    }
    if input.criterion_version == 0 || input.expected_parent_revision == 0 {
        return Err(invalid(
            "Criterion versions and parent revisions must be positive integers.",
        ));
    }
    if input.reviewed_snapshot_digest.len() != 64
        || !input
            .reviewed_snapshot_digest
            .bytes()
            .all(|c| c.is_ascii_digit() || (b'a'..=b'f').contains(&c))
    {
        return Err(invalid("The review digest is invalid."));
    }
    refs(&input.evidence_refs)?;
    input.evidence_refs.sort();
    text(&input.factual_summary, 2000, "Factual summary")?;
    Ok(input)
}
pub(crate) fn accept(
    connection: &Connection,
    input: AcceptProgressInput,
) -> Result<SettlementResult, CommandError> {
    let input = canonical_command(input)?;
    let digest = hash(&input)?;
    let committed: Option<String> = db(connection
        .query_row(
            "SELECT payload FROM committed_commands WHERE id=?1",
            [&input.command_id],
            |row| row.get(0),
        )
        .optional())?;
    if let Some(payload) = committed {
        let command: CommittedCommand =
            serde_json::from_str(&payload).map_err(|_| CommandError::storage())?;
        if command.payload_digest != digest {
            return Err(CommandError::new(
                "idempotency_conflict",
                "This command ID was already committed with a different payload.",
            ));
        }
        return Ok(command.result);
    }
    // Exact committed retries must win over these potentially stale checks.
    generation(connection, &input.expected_generation)?;
    let revision: u32 = db(connection.query_row(
        "SELECT revision FROM objectives WHERE id=?1",
        [&input.parent_id],
        |row| row.get(0),
    ))?;
    if revision != input.expected_parent_revision {
        return Err(CommandError::conflict());
    }
    let state = read(connection)?;
    let criterion = exact(
        &state,
        &input.parent_id,
        &input.criterion_lineage_id,
        input.criterion_version,
    )?;
    let mut snapshot = state
        .snapshots
        .iter()
        .find(|s| s.id == input.reviewed_snapshot_id)
        .ok_or_else(|| invalid("The reviewed snapshot does not exist."))?
        .clone();
    if snapshot.criterion_id != criterion.id
        || snapshot.digest != input.reviewed_snapshot_digest
        || snapshot.disposition == ReviewDisposition::Rejected
        || snapshot.factual_summary != input.factual_summary
        || snapshot.assistance != input.assistance
        || snapshot.evidence_refs != input.evidence_refs
    {
        return Err(invalid(
            "The acceptance does not match its reviewed factual claim.",
        ));
    }
    let current = evaluate(
        &state,
        criterion,
        &input.evidence_refs,
        &input.factual_summary,
        &input.assistance,
        state.registry_revision,
    )?;
    if current.digest != snapshot.digest {
        return Err(CommandError::new(
            "stale_review",
            "The evidence registry or criterion changed. Request a fresh review.",
        ));
    }
    if !current.eligible {
        return Err(CommandError::new(
            "evidence_ineligible",
            "Required evidence is missing or contradictory. This review remains pending.",
        ));
    }
    let previous = state.acceptances.iter().find(|a| {
        a.reward_slot_id == criterion.reward_slot_id && a.status == AcceptanceStatus::Accepted
    });
    if previous.is_none() && snapshot.disposition != ReviewDisposition::Pending {
        return Err(invalid(
            "A corrected or reopened accomplishment requires a fresh review before reacceptance.",
        ));
    }
    if previous.is_some_and(|old| {
        old.criterion_id != criterion.id
            || old.snapshot_id != snapshot.id
            || old.factual_summary != input.factual_summary
            || old.assistance != input.assistance
    }) {
        return Err(invalid(
            "This lineage already has an accepted accomplishment. Correct or reopen that historical acceptance before accepting a different claim.",
        ));
    }
    let acceptance = previous.cloned().unwrap_or_else(|| Acceptance {
        id: new_id(),
        parent_id: criterion.parent_id.clone(),
        criterion_lineage_id: criterion.criterion_lineage_id.clone(),
        criterion_version: criterion.criterion_version,
        criterion_id: criterion.id.clone(),
        reward_slot_id: criterion.reward_slot_id.clone(),
        snapshot_id: snapshot.id.clone(),
        actor: "local_user".into(),
        method: "user_confirmed".into(),
        assistance: input.assistance.clone(),
        factual_summary: input.factual_summary.clone(),
        accepted_at: now(),
        status: AcceptanceStatus::Accepted,
        correction_id: None,
    });
    if previous.is_none() {
        insert(connection, "acceptances", &acceptance.id, &acceptance)?;
        audit(connection, "accomplishment_accepted", &acceptance.id)?;
    }
    let previous_grant = state
        .grants
        .iter()
        .find(|g| g.reward_slot_id == criterion.reward_slot_id);
    let mut issued = 0;
    let mut grant_id = previous_grant.map(|g| g.id.clone());
    if previous_grant.is_none() {
        let slot = state
            .slots
            .iter()
            .find(|s| s.id == criterion.reward_slot_id)
            .ok_or_else(CommandError::storage)?;
        if !slot.allocation_locked
            || state
                .grants
                .iter()
                .filter(|g| g.parent_id == criterion.parent_id)
                .map(|g| g.amount_bc)
                .sum::<u32>()
                + slot.allocation_bc
                > 4
        {
            return Err(invalid(
                "The parent's lifetime reward allocation is unavailable.",
            ));
        }
        let grant = RewardGrant {
            id: new_id(),
            parent_id: criterion.parent_id.clone(),
            reward_slot_id: slot.id.clone(),
            criterion_lineage_id: criterion.criterion_lineage_id.clone(),
            acceptance_id: acceptance.id.clone(),
            reward_policy_version: "bc-v1".into(),
            amount_bc: slot.allocation_bc,
            created_at: now(),
        };
        insert(connection, "reward_grants", &grant.id, &grant)?;
        let entry = LedgerEntry {
            id: new_id(),
            grant_id: grant.id.clone(),
            sequence: state.ledger.len() as u32 + 1,
            amount_bc: grant.amount_bc,
            created_at: now(),
        };
        insert(connection, "ledger_entries", &entry.id, &entry)?;
        audit(connection, "reward_issued", &grant.id)?;
        issued = grant.amount_bc;
        grant_id = Some(grant.id);
    }
    snapshot.disposition = ReviewDisposition::Accepted;
    update(connection, "review_snapshots", &snapshot.id, &snapshot)?;
    let result = SettlementResult {
        acceptance_id: acceptance.id,
        grant_id,
        issued_bc: issued,
        balance_bc: state.wallet_bc + issued,
        retained_reward: previous_grant.is_some(),
    };
    let command = CommittedCommand {
        id: input.command_id.clone(),
        ledger_sequence: state.ledger.len() as u32 + u32::from(issued > 0),
        payload_digest: digest,
        payload: input,
        result: result.clone(),
    };
    insert(connection, "committed_commands", &command.id, &command)?;
    changed(connection)?;
    Ok(result)
}
pub(crate) fn correct(
    connection: &Connection,
    input: CorrectAcceptanceInput,
) -> Result<(), CommandError> {
    generation(connection, &input.expected_generation)?;
    valid_id(&input.acceptance_id)?;
    text(&input.reason, 2000, "Correction reason")?;
    let state = read(connection)?;
    let mut acceptance = state
        .acceptances
        .iter()
        .find(|a| a.id == input.acceptance_id)
        .ok_or_else(|| invalid("The acceptance does not exist."))?
        .clone();
    if acceptance.status != AcceptanceStatus::Accepted {
        return Err(invalid(
            "This historical acceptance was already corrected or reopened.",
        ));
    }
    let correction = Correction {
        id: new_id(),
        acceptance_id: acceptance.id.clone(),
        // A correction changes acceptance authority even if evidence is unchanged.
        // All earlier pending reviews must be reviewed again before reacceptance.
        registry_revision: next_registry(connection)?,
        kind: input.kind.clone(),
        reason: input.reason,
        created_at: now(),
    };
    insert(connection, "corrections", &correction.id, &correction)?;
    acceptance.status = match input.kind {
        CorrectionKind::IncorrectAcceptance => AcceptanceStatus::Retracted,
        CorrectionKind::HistoricalReopened => AcceptanceStatus::Historical,
    };
    acceptance.correction_id = Some(correction.id.clone());
    update(connection, "acceptances", &acceptance.id, &acceptance)?;
    audit(connection, "accomplishment_corrected", &correction.id)?;
    changed(connection)
}
pub(crate) fn reject(
    connection: &Connection,
    input: RejectReviewInput,
) -> Result<(), CommandError> {
    generation(connection, &input.expected_generation)?;
    valid_id(&input.snapshot_id)?;
    text(&input.reason, 2000, "Rejection reason")?;
    let state = read(connection)?;
    let mut snapshot = state
        .snapshots
        .iter()
        .find(|s| s.id == input.snapshot_id)
        .ok_or_else(|| invalid("The review does not exist."))?
        .clone();
    if snapshot.disposition != ReviewDisposition::Pending {
        return Err(invalid("Only a pending review may be rejected."));
    }
    snapshot.disposition = ReviewDisposition::Rejected;
    snapshot.rejection_reason = Some(input.reason);
    update(connection, "review_snapshots", &snapshot.id, &snapshot)?;
    audit(connection, "review_rejected", &snapshot.id)?;
    changed(connection)
}

/// Validate the entire portable authority graph before either restore or commit.
/// A digest alone is insufficient: historical snapshots are independently rebuilt
/// from the registered evidence that existed at their recorded registry revision.
pub(crate) fn validate(
    state: &ProgressState,
    objectives: &[Objective],
) -> Result<(), CommandError> {
    if state.record_count() > MAX_RECORDS || state.registry_revision > MAX_REGISTRY_REVISION {
        return Err(invalid(
            "The progress record or revision limit was exceeded.",
        ));
    }
    // Only mutation events consume registry revisions. Legacy drafts use zero;
    // multiple reviews may share a revision, but distinct authoritative changes
    // cannot borrow each other's position in the correction/evidence history.
    let mut mutation_revisions = HashSet::new();
    for revision in state
        .criteria
        .iter()
        .map(|record| record.registry_revision)
        .chain(state.evidence.iter().map(|record| record.registry_revision))
        .chain(
            state
                .validators
                .iter()
                .map(|record| record.registry_revision),
        )
        .chain(
            state
                .corrections
                .iter()
                .map(|record| record.registry_revision),
        )
        .filter(|revision| *revision != 0)
    {
        if !mutation_revisions.insert(revision) {
            return Err(invalid(
                "Distinct authority changes cannot share an evidence-registry revision.",
            ));
        }
    }
    let mut ids = objectives
        .iter()
        .map(|o| o.id.clone())
        .collect::<HashSet<_>>();
    let mut unique = |id: &str| -> Result<(), CommandError> {
        valid_id(id)?;
        if !ids.insert(id.into()) {
            return Err(invalid("Progress record identities must be unique."));
        }
        Ok(())
    };
    macro_rules! identities {($($field:ident),*)=>{$(for record in &state.$field { unique(&record.id)?; })*};}
    identities!(
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
    let parents = state
        .parents
        .iter()
        .map(|v| (v.parent_id.as_str(), v))
        .collect::<BTreeMap<_, _>>();
    let slots = state
        .slots
        .iter()
        .map(|v| (v.id.as_str(), v))
        .collect::<BTreeMap<_, _>>();
    let criteria = state
        .criteria
        .iter()
        .map(|v| (v.id.as_str(), v))
        .collect::<BTreeMap<_, _>>();
    let evidence = state
        .evidence
        .iter()
        .map(|v| (v.id.as_str(), v))
        .collect::<BTreeMap<_, _>>();
    let snapshots = state
        .snapshots
        .iter()
        .map(|v| (v.id.as_str(), v))
        .collect::<BTreeMap<_, _>>();
    let acceptances = state
        .acceptances
        .iter()
        .map(|v| (v.id.as_str(), v))
        .collect::<BTreeMap<_, _>>();
    let corrections = state
        .corrections
        .iter()
        .map(|v| (v.id.as_str(), v))
        .collect::<BTreeMap<_, _>>();
    let grants = state
        .grants
        .iter()
        .map(|v| (v.id.as_str(), v))
        .collect::<BTreeMap<_, _>>();
    if parents.len() != state.parents.len() || parents.len() != objectives.len() {
        return Err(invalid(
            "Every objective needs exactly one lifetime reward parent.",
        ));
    }
    for objective in objectives {
        let parent = parents
            .get(objective.id.as_str())
            .ok_or_else(|| invalid("A reward parent is missing."))?;
        valid_id(&parent.primary_lineage_id)?;
        if parent.reward_policy_version != "bc-v1"
            || parent.lifetime_budget_bc != 4
            || !matches!(
                parent.provenance.as_str(),
                "legacy_stage3a" | "local_created"
            )
        {
            return Err(invalid(
                "The reward parent policy or provenance is invalid.",
            ));
        }
        let canonical = current(state, &objective.id, &parent.primary_lineage_id)?;
        if canonical.description != objective.criterion {
            return Err(invalid(
                "The objective criterion projection does not match its canonical version.",
            ));
        }
        if state
            .slots
            .iter()
            .filter(|s| s.parent_id == objective.id)
            .map(|s| u64::from(s.allocation_bc))
            .sum::<u64>()
            > 4
        {
            return Err(invalid(
                "A parent allocation exceeds its four-credit lifetime budget.",
            ));
        }
    }
    let mut lineages = HashSet::new();
    for slot in &state.slots {
        valid_id(&slot.criterion_lineage_id)?;
        if !parents.contains_key(slot.parent_id.as_str())
            || !lineages.insert(&slot.criterion_lineage_id)
            || !(1..=4).contains(&slot.allocation_bc)
        {
            return Err(invalid(
                "A reward slot has an invalid parent, lineage, or allocation.",
            ));
        }
        let mut versions = state
            .criteria
            .iter()
            .filter(|c| c.reward_slot_id == slot.id)
            .collect::<Vec<_>>();
        versions.sort_by_key(|c| c.criterion_version);
        if versions.is_empty()
            || slot.allocation_locked != versions.iter().any(|c| c.state == CriterionState::Locked)
        {
            return Err(invalid(
                "A slot's locked allocation does not match its criterion history.",
            ));
        }
        for (index, criterion) in versions.iter().enumerate() {
            if criterion.criterion_version as usize != index + 1
                || (criterion.state == CriterionState::Draft && index + 1 != versions.len())
            {
                return Err(invalid(
                    "Criterion versions must be contiguous, with only the latest version editable.",
                ));
            }
        }
    }
    for criterion in &state.criteria {
        text(&criterion.description, 2000, "Criterion")?;
        text(&criterion.scope, 500, "Scope")?;
        artifact(&criterion.artifact_id, &criterion.artifact_version)?;
        refs(&criterion.required_validators)?;
        timestamp(&criterion.created_at)?;
        let slot = slots
            .get(criterion.reward_slot_id.as_str())
            .ok_or_else(|| invalid("A criterion references a missing reward slot."))?;
        if slot.parent_id != criterion.parent_id
            || slot.criterion_lineage_id != criterion.criterion_lineage_id
            || criterion.registry_revision > state.registry_revision
        {
            return Err(invalid(
                "A criterion has inconsistent parent, lineage, or registry metadata.",
            ));
        }
        if criterion.evidence_policy != EvidencePolicy::TrustedValidation
            && !criterion.required_validators.is_empty()
        {
            return Err(invalid(
                "Validator requirements conflict with the evidence policy.",
            ));
        }
        match criterion.state {
            CriterionState::Draft => {
                if criterion.locked_at.is_some() || criterion.lock_registry_revision.is_some() {
                    return Err(invalid("A draft criterion cannot carry a lock."));
                }
            }
            CriterionState::Locked => {
                timestamp(
                    criterion
                        .locked_at
                        .as_deref()
                        .ok_or_else(|| invalid("The criterion lock time is missing."))?,
                )?;
                if criterion.lock_registry_revision != Some(criterion.registry_revision)
                    || criterion.registry_revision == 0
                {
                    return Err(invalid("The criterion lock revision is invalid."));
                }
            }
        }
    }
    for validator in &state.validators {
        let criterion = state
            .criteria
            .iter()
            .find(|c| {
                c.criterion_lineage_id == validator.criterion_lineage_id
                    && c.criterion_version == validator.criterion_version
            })
            .ok_or_else(|| invalid("A validator's criterion is missing."))?;
        text(&validator.source_id, 500, "Validator source")?;
        text(&validator.source_provenance, 500, "Validator provenance")?;
        if !validator_applicable(validator, criterion)
            || validator.registry_revision == 0
            || validator.registry_revision > state.registry_revision
        {
            return Err(invalid(
                "A validator definition has inconsistent scope or revision.",
            ));
        }
        // Stage 3b ships no external validator adapter. Synthetic fixtures are
        // explicitly labeled; importing unknown metadata never creates trust.
        if validator.available && !supported_validator(validator) {
            return Err(invalid(
                "This application has no trusted adapter for the imported validator provenance.",
            ));
        }
    }
    let mut superseded = HashSet::new();
    for record in &state.evidence {
        let criterion = criteria
            .get(record.criterion_id.as_str())
            .ok_or_else(|| invalid("Evidence references a missing criterion."))?;
        if !applicable(record, criterion)
            || record.registry_revision == 0
            || record.registry_revision > state.registry_revision
        {
            return Err(invalid(
                "Evidence has inconsistent identity, scope, or registry revision.",
            ));
        }
        timestamp(&record.observed_at)?;
        timestamp(&record.recorded_at)?;
        text(&record.description, 2000, "Evidence description")?;
        optional_text(&record.reference, 2000, "Evidence reference")?;
        optional_text(&record.uncertainty, 2000, "Evidence uncertainty")?;
        if record.kind == EvidenceKind::ArtifactReference && record.reference.is_none() {
            return Err(invalid("Artifact evidence lacks its reference."));
        }
        if record.kind == EvidenceKind::RegisteredValidator {
            let validator = state
                .validators
                .iter()
                .find(|v| Some(&v.id) == record.validator_id.as_ref())
                .ok_or_else(|| invalid("Registered evidence lacks its validator definition."))?;
            if !validator_applicable(validator, criterion)
                || record.source_id != validator.source_id
                || record.source_provenance != validator.source_provenance
                || record.registry_revision <= validator.registry_revision
            {
                return Err(invalid(
                    "Registered evidence does not match its actual source authority.",
                ));
            }
        } else if record.validator_id.is_some()
            || record.source_id != "local_user"
            || record.source_provenance
                != if matches!(
                    record.kind,
                    EvidenceKind::ImportedMetadata | EvidenceKind::ThirdPartyClaim
                ) {
                    "user_entered_unverified"
                } else {
                    "local_user_report"
                }
        {
            return Err(invalid(
                "User-reported evidence may not acquire validator privileges.",
            ));
        }
        if let Some(previous) = &record.supersedes {
            let old = evidence
                .get(previous.as_str())
                .ok_or_else(|| invalid("Superseded evidence is missing."))?;
            if !superseded.insert(previous)
                || old.registry_revision >= record.registry_revision
                || old.criterion_id != record.criterion_id
                || old.source_id != record.source_id
                || old.validator_id != record.validator_id
            {
                return Err(invalid("An evidence supersession chain is inconsistent."));
            }
        }
    }
    for snapshot in &state.snapshots {
        let criterion = criteria
            .get(snapshot.criterion_id.as_str())
            .ok_or_else(|| invalid("A snapshot's criterion is missing."))?;
        timestamp(&snapshot.reviewed_at)?;
        if snapshot.criterion != **criterion
            || snapshot.registry_revision > state.registry_revision
            || snapshot.registry_revision < criterion.registry_revision
            || criterion.state != CriterionState::Locked
        {
            return Err(invalid(
                "A review snapshot has inconsistent criterion history.",
            ));
        }
        let expected = evaluate(
            state,
            criterion,
            &snapshot.evidence_refs,
            &snapshot.factual_summary,
            &snapshot.assistance,
            snapshot.registry_revision,
        )?;
        if snapshot.digest != snapshot_digest(snapshot)? || snapshot.digest != expected.digest {
            return Err(invalid(
                "A review snapshot omits or changes authoritative evidence.",
            ));
        }
        match snapshot.disposition {
            ReviewDisposition::Rejected => text(
                snapshot
                    .rejection_reason
                    .as_deref()
                    .ok_or_else(|| invalid("A rejection reason is missing."))?,
                2000,
                "Rejection reason",
            )?,
            _ => {
                if snapshot.rejection_reason.is_some() {
                    return Err(invalid(
                        "A non-rejected review carries an inconsistent rejection.",
                    ));
                }
            }
        }
    }
    let mut active = HashSet::new();
    let mut accepted_snapshots = HashSet::new();
    let mut corrected = HashSet::new();
    for acceptance in &state.acceptances {
        if !accepted_snapshots.insert(&acceptance.snapshot_id) {
            return Err(invalid(
                "A factual acceptance must have its own explicit review snapshot.",
            ));
        }
        let criterion = criteria
            .get(acceptance.criterion_id.as_str())
            .ok_or_else(|| invalid("An acceptance criterion is missing."))?;
        let snapshot = snapshots
            .get(acceptance.snapshot_id.as_str())
            .ok_or_else(|| invalid("An accepted snapshot is missing."))?;
        timestamp(&acceptance.accepted_at)?;
        if criterion.state != CriterionState::Locked
            || acceptance.parent_id != criterion.parent_id
            || acceptance.criterion_lineage_id != criterion.criterion_lineage_id
            || acceptance.criterion_version != criterion.criterion_version
            || acceptance.reward_slot_id != criterion.reward_slot_id
            || acceptance.actor != "local_user"
            || acceptance.method != "user_confirmed"
            || snapshot.criterion_id != criterion.id
            || !snapshot.eligible
            || snapshot.disposition != ReviewDisposition::Accepted
            || snapshot.factual_summary != acceptance.factual_summary
            || snapshot.assistance != acceptance.assistance
        {
            return Err(invalid(
                "An acceptance is not supported by its locked criterion and complete eligible review.",
            ));
        }
        match acceptance.status {
            AcceptanceStatus::Accepted => {
                if acceptance.correction_id.is_some() || !active.insert(&acceptance.reward_slot_id)
                {
                    return Err(invalid(
                        "A reward slot has conflicting current acceptance state.",
                    ));
                }
            }
            _ => {
                let correction =
                    corrections
                        .get(acceptance.correction_id.as_deref().ok_or_else(|| {
                            invalid("A corrected acceptance lacks its correction.")
                        })?)
                        .ok_or_else(|| invalid("A correction is missing."))?;
                if correction.acceptance_id != acceptance.id
                    || (correction.kind == CorrectionKind::IncorrectAcceptance)
                        != (acceptance.status == AcceptanceStatus::Retracted)
                {
                    return Err(invalid("A correction does not match the factual status."));
                }
            }
        }
    }
    for snapshot in &state.snapshots {
        if (snapshot.disposition == ReviewDisposition::Accepted)
            != accepted_snapshots.contains(&snapshot.id)
        {
            return Err(invalid(
                "An accepted review must retain its exactly one factual acceptance.",
            ));
        }
    }
    let mut correction_revisions = HashSet::new();
    for correction in &state.corrections {
        timestamp(&correction.created_at)?;
        text(&correction.reason, 2000, "Correction reason")?;
        let acceptance = acceptances
            .get(correction.acceptance_id.as_str())
            .ok_or_else(|| invalid("A correction references a missing acceptance."))?;
        if acceptance.correction_id.as_ref() != Some(&correction.id)
            || !corrected.insert(&correction.acceptance_id)
            || !correction_revisions.insert(correction.registry_revision)
            || correction.registry_revision > state.registry_revision
            || correction.registry_revision
                <= snapshots[acceptance.snapshot_id.as_str()].registry_revision
        {
            return Err(invalid("A correction history is inconsistent."));
        }
    }
    let mut consumed = HashSet::new();
    let mut parent_issued = BTreeMap::<&str, u64>::new();
    for grant in &state.grants {
        timestamp(&grant.created_at)?;
        let acceptance = acceptances
            .get(grant.acceptance_id.as_str())
            .ok_or_else(|| invalid("A reward grant lacks its acceptance."))?;
        let slot = slots
            .get(grant.reward_slot_id.as_str())
            .ok_or_else(|| invalid("A reward grant lacks its slot."))?;
        if !consumed.insert(&grant.reward_slot_id)
            || grant.parent_id != slot.parent_id
            || grant.criterion_lineage_id != slot.criterion_lineage_id
            || grant.amount_bc != slot.allocation_bc
            || !slot.allocation_locked
            || grant.reward_policy_version != "bc-v1"
            || acceptance.reward_slot_id != slot.id
            || acceptance.parent_id != grant.parent_id
            || acceptance.criterion_lineage_id != grant.criterion_lineage_id
        {
            return Err(invalid(
                "A reward grant has inconsistent entitlement or issuance.",
            ));
        }
        *parent_issued.entry(&grant.parent_id).or_default() += u64::from(grant.amount_bc);
    }
    if parent_issued.values().any(|total| *total > 4) {
        return Err(invalid("Issued credits exceed a lifetime parent budget."));
    }
    // Reconstruct causal acceptance history without trusting wall-clock order.
    // The original grant belongs to the first acceptance; every later acceptance
    // requires a snapshot reviewed at or after the preceding correction revision.
    for slot in &state.slots {
        let mut history = state
            .acceptances
            .iter()
            .filter(|acceptance| acceptance.reward_slot_id == slot.id)
            .collect::<Vec<_>>();
        if history.is_empty() {
            continue;
        }
        history.sort_by_key(|acceptance| {
            acceptance
                .correction_id
                .as_ref()
                .map(|id| corrections[id.as_str()].registry_revision)
                .unwrap_or(u64::MAX)
        });
        let grant = state
            .grants
            .iter()
            .find(|grant| grant.reward_slot_id == slot.id)
            .ok_or_else(|| invalid("An acceptance history has no original reward grant."))?;
        if history[0].id != grant.acceptance_id {
            return Err(invalid(
                "A reward grant does not belong to the first acceptance of its entitlement.",
            ));
        }
        for pair in history.windows(2) {
            let correction_id = pair[0].correction_id.as_deref().ok_or_else(|| {
                invalid("Reacceptance requires a preceding factual correction or reopening.")
            })?;
            let cutoff = corrections[correction_id].registry_revision;
            if snapshots[pair[1].snapshot_id.as_str()].registry_revision < cutoff {
                return Err(invalid(
                    "Reacceptance reused a review from before its preceding correction.",
                ));
            }
        }
    }
    let mut ledger_grants = HashSet::new();
    let mut balance = 0u64;
    let mut ordered_ledger = state.ledger.iter().collect::<Vec<_>>();
    ordered_ledger.sort_by_key(|entry| entry.sequence);
    let mut balances = BTreeMap::new();
    let mut grant_sequences = BTreeMap::new();
    for (index, entry) in ordered_ledger.iter().enumerate() {
        timestamp(&entry.created_at)?;
        let grant = grants
            .get(entry.grant_id.as_str())
            .ok_or_else(|| invalid("A ledger entry references a missing grant."))?;
        if entry.sequence as usize != index + 1
            || !ledger_grants.insert(&entry.grant_id)
            || entry.amount_bc != grant.amount_bc
        {
            return Err(invalid("Ledger issuance does not match its unique grant."));
        }
        balance += u64::from(entry.amount_bc);
        balances.insert(entry.sequence, balance);
        grant_sequences.insert(entry.grant_id.as_str(), entry.sequence);
    }
    if ledger_grants.len() != grants.len() || balance != u64::from(state.wallet_bc) {
        return Err(invalid(
            "The wallet projection does not match the complete issuance ledger.",
        ));
    }
    let mut audited = HashSet::new();
    for event in &state.audit {
        timestamp(&event.created_at)?;
        valid_id(&event.subject_id)?;
        if event.actor != "local_user" {
            return Err(invalid("An audit event has an unauthorized actor."));
        }
        let exists = match event.kind.as_str() {
            "criterion_saved" | "criterion_locked" => {
                criteria.contains_key(event.subject_id.as_str())
            }
            "evidence_recorded" => evidence.contains_key(event.subject_id.as_str()),
            "review_requested" | "review_rejected" => {
                snapshots.contains_key(event.subject_id.as_str())
            }
            "accomplishment_accepted" => acceptances.contains_key(event.subject_id.as_str()),
            "accomplishment_corrected" => corrections.contains_key(event.subject_id.as_str()),
            "reward_issued" => grants.contains_key(event.subject_id.as_str()),
            _ => false,
        };
        if !exists {
            return Err(invalid(
                "An audit event references an unknown event or record.",
            ));
        }
        audited.insert((event.kind.as_str(), event.subject_id.as_str()));
    }
    for acceptance in &state.acceptances {
        if !audited.contains(&("accomplishment_accepted", acceptance.id.as_str())) {
            return Err(invalid("An acceptance lacks its audit event."));
        }
    }
    for correction in &state.corrections {
        if !audited.contains(&("accomplishment_corrected", correction.id.as_str())) {
            return Err(invalid("A correction lacks its audit event."));
        }
    }
    for grant in &state.grants {
        if !audited.contains(&("reward_issued", grant.id.as_str())) {
            return Err(invalid("A reward lacks its audit event."));
        }
    }
    let mut commands_acceptances = HashSet::new();
    let mut issued_commands = HashSet::new();
    for command in &state.commands {
        let canonical = canonical_command(command.payload.clone())?;
        let acceptance = acceptances
            .get(command.result.acceptance_id.as_str())
            .ok_or_else(|| invalid("A committed command references a missing acceptance."))?;
        let snapshot = snapshots
            .get(command.payload.reviewed_snapshot_id.as_str())
            .ok_or_else(|| invalid("A command snapshot is missing."))?;
        let grant =
            grants
                .get(command.result.grant_id.as_deref().ok_or_else(|| {
                    invalid("A settled command lacks its retained or issued grant.")
                })?)
                .ok_or_else(|| invalid("A command grant is missing."))?;
        if canonical != command.payload
            || command.id != canonical.command_id
            || hash(&canonical)? != command.payload_digest
            || snapshot.digest != canonical.reviewed_snapshot_digest
            || snapshot.parent_id != canonical.parent_id
            || snapshot.criterion_lineage_id != canonical.criterion_lineage_id
            || snapshot.criterion_version != canonical.criterion_version
            || snapshot.evidence_refs != canonical.evidence_refs
            || snapshot.factual_summary != canonical.factual_summary
            || snapshot.assistance != canonical.assistance
            || !snapshot.eligible
            || snapshot.disposition != ReviewDisposition::Accepted
            || acceptance.reward_slot_id != grant.reward_slot_id
            || canonical.parent_id != acceptance.parent_id
            || canonical.criterion_lineage_id != acceptance.criterion_lineage_id
            || canonical.criterion_version != acceptance.criterion_version
            || canonical.reviewed_snapshot_id != acceptance.snapshot_id
            || objectives
                .iter()
                .find(|o| o.id == canonical.parent_id)
                .is_none_or(|o| canonical.expected_parent_revision > o.revision)
            || command.result.balance_bc < command.result.issued_bc
            || command.result.balance_bc < grant.amount_bc
            || command.result.balance_bc > state.wallet_bc
            || balances.get(&command.ledger_sequence) != Some(&u64::from(command.result.balance_bc))
            || grant_sequences
                .get(grant.id.as_str())
                .is_none_or(|sequence| *sequence > command.ledger_sequence)
        {
            return Err(invalid(
                "A committed idempotency result is inconsistent with its acceptance and ledger.",
            ));
        }
        if command.result.issued_bc > 0 {
            if command.result.issued_bc != grant.amount_bc
                || grant_sequences.get(grant.id.as_str()) != Some(&command.ledger_sequence)
                || grant.acceptance_id != acceptance.id
                || command.result.retained_reward
                || !issued_commands.insert(&grant.id)
            {
                return Err(invalid(
                    "A grant has inconsistent or duplicate issuance commands.",
                ));
            }
        } else if !command.result.retained_reward {
            return Err(invalid(
                "A zero-issuance settlement must preserve its consumed entitlement.",
            ));
        }
        commands_acceptances.insert(&acceptance.id);
    }
    if issued_commands.len() != grants.len() || commands_acceptances.len() != acceptances.len() {
        return Err(invalid(
            "Acceptance or reward history is missing its committed idempotency result.",
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests;
