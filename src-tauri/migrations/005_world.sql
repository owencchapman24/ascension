-- River Reach is a separate spending authority. Issuance tables are unchanged.
CREATE TABLE world_chapter (
 id TEXT PRIMARY KEY NOT NULL,
 singleton INTEGER NOT NULL UNIQUE CHECK(singleton=1),
 payload TEXT NOT NULL CHECK(json_valid(payload)),
 revision INTEGER GENERATED ALWAYS AS(json_extract(payload,'$.revision')) STORED NOT NULL CHECK(revision BETWEEN 1 AND 4294967295),
 funded_sites INTEGER GENERATED ALWAYS AS(json_extract(payload,'$.funded_site_count')) STORED NOT NULL CHECK(funded_sites BETWEEN 0 AND 6),
 reserved_bc INTEGER GENERATED ALWAYS AS(COALESCE(json_extract(payload,'$.queue.reserved_bc'),0)) STORED NOT NULL CHECK(reserved_bc BETWEEN 0 AND 24),
 CHECK(json_extract(payload,'$.id')=id),
 CHECK(json_extract(payload,'$.content_version')='river-reach-v1'),
 CHECK(json_type(payload,'$.revision')='integer'),
 CHECK(json_type(payload,'$.funded_site_count')='integer'),
 CHECK(json_array_length(payload,'$.layout')<=6)
) STRICT;
CREATE TABLE world_spending (
 id TEXT PRIMARY KEY NOT NULL,
 payload TEXT NOT NULL CHECK(json_valid(payload)),
 chapter_id TEXT GENERATED ALWAYS AS(json_extract(payload,'$.chapter_id')) STORED NOT NULL REFERENCES world_chapter(id),
 sequence INTEGER GENERATED ALWAYS AS(json_extract(payload,'$.sequence')) STORED NOT NULL UNIQUE CHECK(sequence>0),
 amount_bc INTEGER GENERATED ALWAYS AS(json_extract(payload,'$.amount_bc')) STORED NOT NULL CHECK(amount_bc BETWEEN 4 AND 24 AND amount_bc%4=0),
 command_id TEXT GENERATED ALWAYS AS(json_extract(payload,'$.command_id')) STORED NOT NULL UNIQUE REFERENCES world_commands(id) DEFERRABLE INITIALLY DEFERRED,
 CHECK(json_extract(payload,'$.id')=id),
 CHECK(json_type(payload,'$.amount_bc')='integer'),
 CHECK(json_type(payload,'$.sequence')='integer')
) STRICT;
CREATE TABLE world_commands (
 id TEXT PRIMARY KEY NOT NULL,
 payload TEXT NOT NULL CHECK(json_valid(payload)),
 sequence INTEGER GENERATED ALWAYS AS(json_extract(payload,'$.sequence')) STORED NOT NULL UNIQUE CHECK(sequence>0),
 chapter_id TEXT GENERATED ALWAYS AS(json_extract(payload,'$.result.chapter_id')) STORED NOT NULL REFERENCES world_chapter(id),
 CHECK(json_extract(payload,'$.id')=id),
 CHECK(json_type(payload,'$.sequence')='integer')
) STRICT;
CREATE TRIGGER world_spend_budget BEFORE INSERT ON world_spending
WHEN COALESCE((SELECT SUM(amount_bc) FROM world_spending),0)+NEW.amount_bc>24
 OR COALESCE((SELECT SUM(amount_bc) FROM world_spending),0)+NEW.amount_bc>COALESCE((SELECT SUM(amount_bc) FROM ledger_entries),0)
 OR NEW.sequence<>COALESCE((SELECT MAX(sequence) FROM world_spending),0)+1
BEGIN SELECT RAISE(ABORT,'invalid world spending'); END;
CREATE TRIGGER world_spending_immutable BEFORE UPDATE ON world_spending BEGIN SELECT RAISE(ABORT,'world spending is immutable'); END;
CREATE TRIGGER world_command_immutable BEFORE UPDATE ON world_commands BEGIN SELECT RAISE(ABORT,'world commands are immutable'); END;
CREATE TRIGGER world_capacity_insert BEFORE INSERT ON world_chapter
WHEN NEW.funded_sites<>COALESCE((SELECT SUM(amount_bc) FROM world_spending),0)/4
 OR NEW.reserved_bc+COALESCE((SELECT SUM(amount_bc) FROM world_spending),0)>COALESCE((SELECT SUM(amount_bc) FROM ledger_entries),0)
 OR COALESCE((SELECT SUM(json_array_length(value,'$.parcels')) FROM json_each(NEW.payload,'$.layout')),0)>NEW.funded_sites
BEGIN SELECT RAISE(ABORT,'invalid world capacity'); END;
CREATE TRIGGER world_capacity_update BEFORE UPDATE ON world_chapter
WHEN NEW.funded_sites<>COALESCE((SELECT SUM(amount_bc) FROM world_spending),0)/4
 OR NEW.reserved_bc+COALESCE((SELECT SUM(amount_bc) FROM world_spending),0)>COALESCE((SELECT SUM(amount_bc) FROM ledger_entries),0)
 OR COALESCE((SELECT SUM(json_array_length(value,'$.parcels')) FROM json_each(NEW.payload,'$.layout')),0)>NEW.funded_sites
 OR NEW.id<>OLD.id
 OR json_extract(NEW.payload,'$.created_at')<>json_extract(OLD.payload,'$.created_at')
 OR json_extract(NEW.payload,'$.content_version')<>json_extract(OLD.payload,'$.content_version')
 OR (json_type(OLD.payload,'$.completion')='object' AND json_extract(NEW.payload,'$.completion') IS NOT json_extract(OLD.payload,'$.completion'))
BEGIN SELECT RAISE(ABORT,'invalid world capacity or ending'); END;
