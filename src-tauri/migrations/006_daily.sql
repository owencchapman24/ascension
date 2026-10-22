-- Organizational metadata has no completion or reward authority.
CREATE TABLE objective_planning (
 id TEXT PRIMARY KEY NOT NULL REFERENCES objectives(id),
 payload TEXT NOT NULL CHECK(json_valid(payload)),
 status TEXT GENERATED ALWAYS AS(json_extract(payload,'$.status')) STORED NOT NULL CHECK(status IN('active','paused','archived')),
 priority INTEGER GENERATED ALWAYS AS(json_extract(payload,'$.priority')) STORED NOT NULL CHECK(priority>=0),
 revision INTEGER GENERATED ALWAYS AS(json_extract(payload,'$.revision')) STORED NOT NULL CHECK(revision BETWEEN 1 AND 4294967295),
 CHECK(json_extract(payload,'$.objective_id')=id),
 CHECK(json_type(payload,'$.priority')='integer'),
 CHECK(json_type(payload,'$.revision')='integer')
) STRICT;
INSERT INTO objective_planning(id,payload)
 SELECT id,json_object('objective_id',id,'status','active','priority',ROW_NUMBER() OVER(ORDER BY id)-1,'revision',1,'updated_at',updated_at)
 FROM objectives ORDER BY id;

CREATE TABLE work_checkpoints (
 id TEXT PRIMARY KEY NOT NULL,
 payload TEXT NOT NULL CHECK(json_valid(payload)),
 objective_id TEXT GENERATED ALWAYS AS(json_extract(payload,'$.objective_id')) STORED NOT NULL REFERENCES objectives(id),
 action_id TEXT GENERATED ALWAYS AS(json_extract(payload,'$.action_id')) STORED NOT NULL REFERENCES actions(id),
 command_id TEXT GENERATED ALWAYS AS(json_extract(payload,'$.command_id')) STORED NOT NULL UNIQUE REFERENCES checkpoint_commands(id) DEFERRABLE INITIALLY DEFERRED,
 sequence INTEGER GENERATED ALWAYS AS(json_extract(payload,'$.sequence')) STORED NOT NULL UNIQUE CHECK(sequence>0),
 supersedes TEXT GENERATED ALWAYS AS(json_extract(payload,'$.supersedes')) STORED UNIQUE REFERENCES work_checkpoints(id) DEFERRABLE INITIALLY DEFERRED,
 CHECK(json_extract(payload,'$.id')=id),
 CHECK(json_extract(payload,'$.provenance')='self_report'),
 CHECK(json_type(payload,'$.sequence')='integer'),
 CHECK((supersedes IS NULL)=(json_type(payload,'$.correction_reason')='null'))
) STRICT;
CREATE TABLE checkpoint_commands (
 id TEXT PRIMARY KEY NOT NULL,
 payload TEXT NOT NULL CHECK(json_valid(payload)),
 checkpoint_id TEXT GENERATED ALWAYS AS(json_extract(payload,'$.checkpoint_id')) STORED NOT NULL UNIQUE REFERENCES work_checkpoints(id) DEFERRABLE INITIALLY DEFERRED,
 CHECK(json_extract(payload,'$.id')=id)
) STRICT;
CREATE TRIGGER checkpoint_immutable BEFORE UPDATE ON work_checkpoints BEGIN SELECT RAISE(ABORT,'checkpoints require append-only correction'); END;
CREATE TRIGGER checkpoint_command_immutable BEFORE UPDATE ON checkpoint_commands BEGIN SELECT RAISE(ABORT,'checkpoint commands are immutable'); END;
