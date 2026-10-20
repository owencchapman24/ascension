-- Stage 3c is an additive observation ledger. No study table can mint or spend BC.
CREATE TABLE schedule_epochs (
 id TEXT PRIMARY KEY NOT NULL,
 payload TEXT NOT NULL CHECK(json_valid(payload)),
 CHECK(json_extract(payload,'$.id')=id)
) STRICT;
CREATE TABLE study_manifests (
 id TEXT PRIMARY KEY NOT NULL,
 payload TEXT NOT NULL CHECK(json_valid(payload)),
 epoch_id TEXT GENERATED ALWAYS AS(json_extract(payload,'$.epoch_id')) STORED NOT NULL REFERENCES schedule_epochs(id),
 phase TEXT GENERATED ALWAYS AS(json_extract(payload,'$.phase')) STORED NOT NULL CHECK(phase IN('e00','e01')),
 CHECK(json_extract(payload,'$.id')=id)
) STRICT;
CREATE TABLE study_protocol_events (
 id TEXT PRIMARY KEY NOT NULL,
 payload TEXT NOT NULL CHECK(json_valid(payload)),
 manifest_id TEXT GENERATED ALWAYS AS(json_extract(payload,'$.manifest_id')) STORED NOT NULL REFERENCES study_manifests(id),
 sequence INTEGER GENERATED ALWAYS AS(json_extract(payload,'$.sequence')) STORED NOT NULL CHECK(sequence>0),
 status TEXT GENERATED ALWAYS AS(json_extract(payload,'$.status')) STORED NOT NULL CHECK(status IN('configured','ready','active','paused','completed','abandoned')),
 CHECK(json_extract(payload,'$.id')=id),
 UNIQUE(manifest_id,sequence)
) STRICT;
CREATE TABLE work_opportunities (
 id TEXT PRIMARY KEY NOT NULL,
 payload TEXT NOT NULL CHECK(json_valid(payload)),
 project_id TEXT GENERATED ALWAYS AS(json_extract(payload,'$.project_id')) STORED NOT NULL REFERENCES projects(id),
 objective_id TEXT GENERATED ALWAYS AS(json_extract(payload,'$.objective_id')) STORED NOT NULL REFERENCES objectives(id),
 action_id TEXT GENERATED ALWAYS AS(json_extract(payload,'$.action_id')) STORED NOT NULL REFERENCES actions(id),
 criterion_id TEXT GENERATED ALWAYS AS(json_extract(payload,'$.criterion_id')) STORED NOT NULL REFERENCES criterion_versions(id),
 epoch_id TEXT GENERATED ALWAYS AS(json_extract(payload,'$.epoch_id')) STORED NOT NULL REFERENCES schedule_epochs(id),
 manifest_id TEXT GENERATED ALWAYS AS(json_extract(payload,'$.manifest_id')) STORED REFERENCES study_manifests(id),
 protocol_event_id TEXT GENERATED ALWAYS AS(json_extract(payload,'$.protocol_event_id')) STORED REFERENCES study_protocol_events(id),
 eligible INTEGER GENERATED ALWAYS AS(json_extract(payload,'$.eligible')) STORED NOT NULL CHECK(eligible IN(0,1)),
 cue_at TEXT GENERATED ALWAYS AS(json_extract(payload,'$.cue_at')) STORED NOT NULL,
 CHECK(json_extract(payload,'$.id')=id),
 CHECK(json_type(payload,'$.eligible') IN('true','false'))
) STRICT;
CREATE INDEX opportunities_cue ON work_opportunities(cue_at);
CREATE INDEX opportunities_protocol ON work_opportunities(manifest_id,epoch_id);
CREATE TABLE study_assignments (
 id TEXT PRIMARY KEY NOT NULL,
 payload TEXT NOT NULL CHECK(json_valid(payload)),
 opportunity_id TEXT GENERATED ALWAYS AS(json_extract(payload,'$.opportunity_id')) STORED NOT NULL UNIQUE REFERENCES work_opportunities(id),
 manifest_id TEXT GENERATED ALWAYS AS(json_extract(payload,'$.manifest_id')) STORED NOT NULL REFERENCES study_manifests(id),
 position INTEGER GENERATED ALWAYS AS(json_extract(payload,'$.position')) STORED NOT NULL CHECK(position>=0),
 condition TEXT GENERATED ALWAYS AS(json_extract(payload,'$.condition')) STORED NOT NULL CHECK(condition IN('d','d_plus_b')),
 CHECK(json_extract(payload,'$.id')=id),
 UNIQUE(manifest_id,position)
) STRICT;
CREATE TABLE opportunity_outcomes (
 id TEXT PRIMARY KEY NOT NULL,
 payload TEXT NOT NULL CHECK(json_valid(payload)),
 opportunity_id TEXT GENERATED ALWAYS AS(json_extract(payload,'$.opportunity_id')) STORED NOT NULL REFERENCES work_opportunities(id),
 sequence INTEGER GENERATED ALWAYS AS(json_extract(payload,'$.sequence')) STORED NOT NULL CHECK(sequence>0),
 supersedes TEXT GENERATED ALWAYS AS(json_extract(payload,'$.supersedes')) STORED REFERENCES opportunity_outcomes(id) DEFERRABLE INITIALLY DEFERRED,
 acceptance_id TEXT GENERATED ALWAYS AS(json_extract(payload,'$.acceptance_id')) STORED REFERENCES acceptances(id),
 state TEXT GENERATED ALWAYS AS(json_extract(payload,'$.state')) STORED NOT NULL CHECK(state IN('pending','observed_start','observed_nonstart','missing')),
 CHECK(json_extract(payload,'$.id')=id),
 UNIQUE(opportunity_id,sequence),
 UNIQUE(supersedes)
) STRICT;
CREATE TABLE study_exposures (
 id TEXT PRIMARY KEY NOT NULL,
 payload TEXT NOT NULL CHECK(json_valid(payload)),
 opportunity_id TEXT GENERATED ALWAYS AS(json_extract(payload,'$.opportunity_id')) STORED REFERENCES work_opportunities(id),
 protocol_event_id TEXT GENERATED ALWAYS AS(json_extract(payload,'$.protocol_event_id')) STORED REFERENCES study_protocol_events(id),
 CHECK(json_extract(payload,'$.id')=id)
) STRICT;
CREATE TABLE study_burdens (
 id TEXT PRIMARY KEY NOT NULL,
 payload TEXT NOT NULL CHECK(json_valid(payload)),
 opportunity_id TEXT GENERATED ALWAYS AS(json_extract(payload,'$.opportunity_id')) STORED REFERENCES work_opportunities(id),
 CHECK(json_extract(payload,'$.id')=id)
) STRICT;
CREATE UNIQUE INDEX study_delivery_identity ON study_exposures(opportunity_id,json_extract(payload,'$.kind'))
 WHERE json_extract(payload,'$.kind') IN('cue_delivered','preview_delivered','checkpoint_delivered');
CREATE TABLE study_reflections (
 id TEXT PRIMARY KEY NOT NULL,
 payload TEXT NOT NULL CHECK(json_valid(payload)),
 manifest_id TEXT GENERATED ALWAYS AS(json_extract(payload,'$.manifest_id')) STORED REFERENCES study_manifests(id),
 CHECK(json_extract(payload,'$.id')=id)
) STRICT;
CREATE TRIGGER epoch_immutable BEFORE UPDATE ON schedule_epochs BEGIN SELECT RAISE(ABORT,'schedule epochs are immutable'); END;
CREATE TRIGGER manifest_immutable BEFORE UPDATE ON study_manifests BEGIN SELECT RAISE(ABORT,'frozen protocols are immutable'); END;
CREATE TRIGGER protocol_event_immutable BEFORE UPDATE ON study_protocol_events BEGIN SELECT RAISE(ABORT,'protocol events are append-only'); END;
CREATE TRIGGER opportunity_immutable BEFORE UPDATE ON work_opportunities BEGIN SELECT RAISE(ABORT,'opportunities are immutable'); END;
CREATE TRIGGER assignment_immutable BEFORE UPDATE ON study_assignments BEGIN SELECT RAISE(ABORT,'assignments are immutable'); END;
CREATE TRIGGER outcome_immutable BEFORE UPDATE ON opportunity_outcomes BEGIN SELECT RAISE(ABORT,'outcomes require appended corrections'); END;
CREATE TRIGGER exposure_immutable BEFORE UPDATE ON study_exposures BEGIN SELECT RAISE(ABORT,'exposures are append-only'); END;
CREATE TRIGGER burden_immutable BEFORE UPDATE ON study_burdens BEGIN SELECT RAISE(ABORT,'burden estimates are append-only'); END;
CREATE TRIGGER reflection_immutable BEFORE UPDATE ON study_reflections BEGIN SELECT RAISE(ABORT,'reflections are append-only'); END;

CREATE TABLE presentation_preferences (
 id TEXT PRIMARY KEY NOT NULL,
 payload TEXT NOT NULL CHECK(json_valid(payload)),
 sequence INTEGER GENERATED ALWAYS AS(json_extract(payload,'$.sequence')) STORED NOT NULL UNIQUE CHECK(sequence>0),
 CHECK(json_extract(payload,'$.id')=id)
) STRICT;
CREATE TRIGGER preference_immutable BEFORE UPDATE ON presentation_preferences BEGIN SELECT RAISE(ABORT,'presentation preferences are append-only'); END;
