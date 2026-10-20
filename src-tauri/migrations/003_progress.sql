-- JSON is the canonical typed record. Generated columns provide independent
-- relational defenses without a second writable representation of its fields.
CREATE TABLE progress_state (
    singleton INTEGER PRIMARY KEY CHECK(singleton=1),
    registry_revision INTEGER NOT NULL CHECK(registry_revision>=0)
) STRICT;
INSERT INTO progress_state VALUES(1,0);
CREATE TABLE progress_parents (
    id TEXT PRIMARY KEY NOT NULL,
    payload TEXT NOT NULL CHECK(json_valid(payload)),
    primary_lineage_id TEXT GENERATED ALWAYS AS(json_extract(payload,'$.primary_lineage_id')) STORED NOT NULL,
    lifetime_budget_bc INTEGER GENERATED ALWAYS AS(json_extract(payload,'$.lifetime_budget_bc')) STORED NOT NULL CHECK(lifetime_budget_bc=4),
    CHECK(json_extract(payload,'$.parent_id')=id),
    CHECK(json_extract(payload,'$.reward_policy_version')='bc-v1'),
    CHECK(json_type(payload,'$.lifetime_budget_bc')='integer'),
    FOREIGN KEY(id) REFERENCES objectives(id)
) STRICT;
CREATE TABLE reward_slots (
    id TEXT PRIMARY KEY NOT NULL,
    payload TEXT NOT NULL CHECK(json_valid(payload)),
    parent_id TEXT GENERATED ALWAYS AS(json_extract(payload,'$.parent_id')) STORED NOT NULL REFERENCES progress_parents(id),
    criterion_lineage_id TEXT GENERATED ALWAYS AS(json_extract(payload,'$.criterion_lineage_id')) STORED NOT NULL UNIQUE,
    allocation_bc INTEGER GENERATED ALWAYS AS(json_extract(payload,'$.allocation_bc')) STORED NOT NULL CHECK(allocation_bc BETWEEN 1 AND 4),
    allocation_locked INTEGER GENERATED ALWAYS AS(json_extract(payload,'$.allocation_locked')) STORED NOT NULL CHECK(allocation_locked IN(0,1)),
    CHECK(json_extract(payload,'$.id')=id),
    CHECK(json_type(payload,'$.allocation_bc')='integer'),
    CHECK(json_type(payload,'$.allocation_locked') IN('true','false')),
    UNIQUE(id,parent_id,criterion_lineage_id)
) STRICT;
CREATE TRIGGER slot_budget_insert BEFORE INSERT ON reward_slots
WHEN COALESCE((SELECT SUM(allocation_bc) FROM reward_slots WHERE parent_id=NEW.parent_id),0)+NEW.allocation_bc>4
BEGIN SELECT RAISE(ABORT,'parent allocation budget exceeded'); END;
CREATE TRIGGER slot_budget_update BEFORE UPDATE ON reward_slots
WHEN COALESCE((SELECT SUM(allocation_bc) FROM reward_slots WHERE parent_id=NEW.parent_id AND id<>OLD.id),0)+NEW.allocation_bc>4
BEGIN SELECT RAISE(ABORT,'parent allocation budget exceeded'); END;
CREATE TRIGGER locked_slot_immutable BEFORE UPDATE ON reward_slots
WHEN OLD.allocation_locked=1 AND NEW.payload<>OLD.payload
BEGIN SELECT RAISE(ABORT,'locked allocation is immutable'); END;
CREATE TABLE criterion_versions (
    id TEXT PRIMARY KEY NOT NULL,
    payload TEXT NOT NULL CHECK(json_valid(payload)),
    parent_id TEXT GENERATED ALWAYS AS(json_extract(payload,'$.parent_id')) STORED NOT NULL,
    criterion_lineage_id TEXT GENERATED ALWAYS AS(json_extract(payload,'$.criterion_lineage_id')) STORED NOT NULL,
    criterion_version INTEGER GENERATED ALWAYS AS(json_extract(payload,'$.criterion_version')) STORED NOT NULL CHECK(criterion_version BETWEEN 1 AND 4294967295),
    reward_slot_id TEXT GENERATED ALWAYS AS(json_extract(payload,'$.reward_slot_id')) STORED NOT NULL,
    description TEXT GENERATED ALWAYS AS(json_extract(payload,'$.description')) STORED NOT NULL,
    state TEXT GENERATED ALWAYS AS(json_extract(payload,'$.state')) STORED NOT NULL CHECK(state IN('draft','locked')),
    registry_revision INTEGER GENERATED ALWAYS AS(json_extract(payload,'$.registry_revision')) STORED NOT NULL CHECK(registry_revision>=0),
    CHECK(json_extract(payload,'$.id')=id),
    CHECK(json_type(payload,'$.criterion_version')='integer'),
    CHECK(json_type(payload,'$.registry_revision')='integer'),
    FOREIGN KEY(reward_slot_id,parent_id,criterion_lineage_id) REFERENCES reward_slots(id,parent_id,criterion_lineage_id),
    UNIQUE(criterion_lineage_id,criterion_version),
    UNIQUE(id,parent_id,criterion_lineage_id,criterion_version)
) STRICT;
CREATE TRIGGER locked_criterion_immutable BEFORE UPDATE ON criterion_versions
WHEN OLD.state='locked' AND NEW.payload<>OLD.payload
BEGIN SELECT RAISE(ABORT,'locked criterion is immutable'); END;
CREATE TABLE validator_definitions (
    id TEXT PRIMARY KEY NOT NULL,
    payload TEXT NOT NULL CHECK(json_valid(payload)),
    parent_id TEXT GENERATED ALWAYS AS(json_extract(payload,'$.parent_id')) STORED NOT NULL REFERENCES progress_parents(id),
    criterion_lineage_id TEXT GENERATED ALWAYS AS(json_extract(payload,'$.criterion_lineage_id')) STORED NOT NULL,
    criterion_version INTEGER GENERATED ALWAYS AS(json_extract(payload,'$.criterion_version')) STORED NOT NULL,
    registry_revision INTEGER GENERATED ALWAYS AS(json_extract(payload,'$.registry_revision')) STORED NOT NULL CHECK(registry_revision>0),
    CHECK(json_extract(payload,'$.id')=id),
    FOREIGN KEY(criterion_lineage_id,criterion_version) REFERENCES criterion_versions(criterion_lineage_id,criterion_version)
) STRICT;
CREATE TABLE evidence_records (
    id TEXT PRIMARY KEY NOT NULL,
    payload TEXT NOT NULL CHECK(json_valid(payload)),
    parent_id TEXT GENERATED ALWAYS AS(json_extract(payload,'$.parent_id')) STORED NOT NULL,
    criterion_lineage_id TEXT GENERATED ALWAYS AS(json_extract(payload,'$.criterion_lineage_id')) STORED NOT NULL,
    criterion_version INTEGER GENERATED ALWAYS AS(json_extract(payload,'$.criterion_version')) STORED NOT NULL,
    criterion_id TEXT GENERATED ALWAYS AS(json_extract(payload,'$.criterion_id')) STORED NOT NULL,
    registry_revision INTEGER GENERATED ALWAYS AS(json_extract(payload,'$.registry_revision')) STORED NOT NULL CHECK(registry_revision>0),
    validator_id TEXT GENERATED ALWAYS AS(json_extract(payload,'$.validator_id')) STORED REFERENCES validator_definitions(id),
    supersedes TEXT GENERATED ALWAYS AS(json_extract(payload,'$.supersedes')) STORED UNIQUE REFERENCES evidence_records(id),
    CHECK(json_extract(payload,'$.id')=id),
    FOREIGN KEY(criterion_id,parent_id,criterion_lineage_id,criterion_version) REFERENCES criterion_versions(id,parent_id,criterion_lineage_id,criterion_version)
) STRICT;
CREATE TRIGGER evidence_immutable BEFORE UPDATE ON evidence_records BEGIN SELECT RAISE(ABORT,'evidence is append only'); END;
CREATE TRIGGER validator_immutable BEFORE UPDATE ON validator_definitions BEGIN SELECT RAISE(ABORT,'validator registry is append only'); END;
CREATE TABLE review_snapshots (
    id TEXT PRIMARY KEY NOT NULL,
    payload TEXT NOT NULL CHECK(json_valid(payload)),
    criterion_id TEXT GENERATED ALWAYS AS(json_extract(payload,'$.criterion_id')) STORED NOT NULL REFERENCES criterion_versions(id),
    CHECK(json_extract(payload,'$.id')=id)
) STRICT;
CREATE TABLE acceptances (
    id TEXT PRIMARY KEY NOT NULL,
    payload TEXT NOT NULL CHECK(json_valid(payload)),
    parent_id TEXT GENERATED ALWAYS AS(json_extract(payload,'$.parent_id')) STORED NOT NULL,
    criterion_lineage_id TEXT GENERATED ALWAYS AS(json_extract(payload,'$.criterion_lineage_id')) STORED NOT NULL,
    criterion_version INTEGER GENERATED ALWAYS AS(json_extract(payload,'$.criterion_version')) STORED NOT NULL,
    criterion_id TEXT GENERATED ALWAYS AS(json_extract(payload,'$.criterion_id')) STORED NOT NULL,
    reward_slot_id TEXT GENERATED ALWAYS AS(json_extract(payload,'$.reward_slot_id')) STORED NOT NULL,
    snapshot_id TEXT GENERATED ALWAYS AS(json_extract(payload,'$.snapshot_id')) STORED NOT NULL UNIQUE REFERENCES review_snapshots(id),
    status TEXT GENERATED ALWAYS AS(json_extract(payload,'$.status')) STORED NOT NULL CHECK(status IN('accepted','retracted','historical')),
    correction_id TEXT GENERATED ALWAYS AS(json_extract(payload,'$.correction_id')) STORED REFERENCES corrections(id) DEFERRABLE INITIALLY DEFERRED,
    CHECK(json_extract(payload,'$.id')=id),
    FOREIGN KEY(criterion_id,parent_id,criterion_lineage_id,criterion_version) REFERENCES criterion_versions(id,parent_id,criterion_lineage_id,criterion_version),
    FOREIGN KEY(reward_slot_id,parent_id,criterion_lineage_id) REFERENCES reward_slots(id,parent_id,criterion_lineage_id),
    UNIQUE(id,parent_id,reward_slot_id,criterion_lineage_id)
) STRICT;
CREATE UNIQUE INDEX one_active_acceptance ON acceptances(reward_slot_id) WHERE status='accepted';
CREATE TRIGGER acceptance_history_immutable BEFORE UPDATE ON acceptances
WHEN json_remove(NEW.payload,'$.status','$.correction_id')<>json_remove(OLD.payload,'$.status','$.correction_id')
BEGIN SELECT RAISE(ABORT,'acceptance history is immutable'); END;
CREATE TABLE corrections (
    id TEXT PRIMARY KEY NOT NULL,
    payload TEXT NOT NULL CHECK(json_valid(payload)),
    acceptance_id TEXT GENERATED ALWAYS AS(json_extract(payload,'$.acceptance_id')) STORED NOT NULL UNIQUE REFERENCES acceptances(id),
    registry_revision INTEGER GENERATED ALWAYS AS(json_extract(payload,'$.registry_revision')) STORED NOT NULL UNIQUE CHECK(registry_revision>0),
    CHECK(json_type(payload,'$.registry_revision')='integer'),
    CHECK(json_extract(payload,'$.id')=id)
) STRICT;
CREATE TRIGGER correction_immutable BEFORE UPDATE ON corrections BEGIN SELECT RAISE(ABORT,'correction is append only'); END;
CREATE TABLE reward_grants (
    id TEXT PRIMARY KEY NOT NULL,
    payload TEXT NOT NULL CHECK(json_valid(payload)),
    parent_id TEXT GENERATED ALWAYS AS(json_extract(payload,'$.parent_id')) STORED NOT NULL,
    reward_slot_id TEXT GENERATED ALWAYS AS(json_extract(payload,'$.reward_slot_id')) STORED NOT NULL UNIQUE,
    criterion_lineage_id TEXT GENERATED ALWAYS AS(json_extract(payload,'$.criterion_lineage_id')) STORED NOT NULL UNIQUE,
    acceptance_id TEXT GENERATED ALWAYS AS(json_extract(payload,'$.acceptance_id')) STORED NOT NULL UNIQUE,
    amount_bc INTEGER GENERATED ALWAYS AS(json_extract(payload,'$.amount_bc')) STORED NOT NULL CHECK(amount_bc BETWEEN 1 AND 4),
    CHECK(json_type(payload,'$.amount_bc')='integer'),
    CHECK(json_extract(payload,'$.id')=id),
    CHECK(json_extract(payload,'$.reward_policy_version')='bc-v1'),
    FOREIGN KEY(acceptance_id,parent_id,reward_slot_id,criterion_lineage_id) REFERENCES acceptances(id,parent_id,reward_slot_id,criterion_lineage_id)
) STRICT;
CREATE TRIGGER grant_budget BEFORE INSERT ON reward_grants
WHEN COALESCE((SELECT SUM(amount_bc) FROM reward_grants WHERE parent_id=NEW.parent_id),0)+NEW.amount_bc>4
  OR NEW.amount_bc<>(SELECT allocation_bc FROM reward_slots WHERE id=NEW.reward_slot_id)
  OR (SELECT allocation_locked FROM reward_slots WHERE id=NEW.reward_slot_id)<>1
  OR (SELECT status FROM acceptances WHERE id=NEW.acceptance_id)<>'accepted'
BEGIN SELECT RAISE(ABORT,'ineligible reward grant'); END;
CREATE TRIGGER grant_immutable BEFORE UPDATE ON reward_grants BEGIN SELECT RAISE(ABORT,'reward grants are immutable'); END;
CREATE TABLE ledger_entries (
    id TEXT PRIMARY KEY NOT NULL,
    payload TEXT NOT NULL CHECK(json_valid(payload)),
    grant_id TEXT GENERATED ALWAYS AS(json_extract(payload,'$.grant_id')) STORED NOT NULL UNIQUE REFERENCES reward_grants(id),
    sequence INTEGER GENERATED ALWAYS AS(json_extract(payload,'$.sequence')) STORED NOT NULL UNIQUE CHECK(sequence BETWEEN 1 AND 4294967295),
    amount_bc INTEGER GENERATED ALWAYS AS(json_extract(payload,'$.amount_bc')) STORED NOT NULL CHECK(amount_bc BETWEEN 1 AND 4),
    CHECK(json_type(payload,'$.amount_bc')='integer'),
    CHECK(json_type(payload,'$.sequence')='integer'),
    CHECK(json_extract(payload,'$.id')=id)
) STRICT;
CREATE TRIGGER ledger_matches_grant BEFORE INSERT ON ledger_entries
WHEN NEW.amount_bc<>(SELECT amount_bc FROM reward_grants WHERE id=NEW.grant_id)
BEGIN SELECT RAISE(ABORT,'ledger does not match grant'); END;
CREATE TRIGGER ledger_immutable BEFORE UPDATE ON ledger_entries BEGIN SELECT RAISE(ABORT,'issuance ledger is immutable'); END;
CREATE TABLE progress_audit (
    id TEXT PRIMARY KEY NOT NULL,
    payload TEXT NOT NULL CHECK(json_valid(payload)),
    CHECK(json_extract(payload,'$.id')=id)
) STRICT;
CREATE TABLE committed_commands (
    id TEXT PRIMARY KEY NOT NULL,
    payload TEXT NOT NULL CHECK(json_valid(payload)),
    CHECK(json_extract(payload,'$.id')=id)
) STRICT;
CREATE TRIGGER command_immutable BEFORE UPDATE ON committed_commands BEGIN SELECT RAISE(ABORT,'committed command is immutable'); END;
CREATE TRIGGER audit_immutable BEFORE UPDATE ON progress_audit BEGIN SELECT RAISE(ABORT,'audit is append only'); END;
