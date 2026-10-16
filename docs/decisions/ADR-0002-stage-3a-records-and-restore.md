# ADR-0002 - Stage 3a records, durable selection and explicit restore

**Status:** Accepted for Stage 3a implementation, subject to native packaging verification.

## Decision

Keep the accepted React/TypeScript/Vite, Tauri 2 and private Rust/rusqlite stack. Rust owns validation, identifiers, timestamps, revisions, transactions and file access. The renderer receives a small typed command API, never SQL, arbitrary paths or filesystem operations. The exact interchange and command contract is in [contracts/README.md](../../contracts/README.md).

A project contains bounded parent objectives. Each objective stores its explicit criterion in the same row: the criterion shares the objective's identity, timestamps and revision. Each objective has at most one next-action record and at most one optional resumption-note record. These records have stable UUIDs, positive integer revisions, and UTC creation/update timestamps. Editing preserves identity and creation time. A changed record increments its revision; an unchanged edit preserves it. Timestamps describe local bookkeeping and do not settle conflicting edits or establish accomplishment.

Save the selected objective in SQLite. Every launch loads the authoritative workspace from Rust, including the selection, action and note. Creating an objective selects that objective as part of the same transaction. Selecting another objective is an explicit persisted mutation. Do not infer a different selection on restart. There are no completion, reward, deletion or automatic prioritization commands in this slice.

Each workspace also has a persisted UUID generation token. Every mutation supplies its expected generation; a stale request fails without changing local records. A successful mutation returns the complete committed workspace. Actual changes rotate the generation; no-op edits keep it. Generation is local concurrency metadata, not a record revision, and is excluded from exports. The frontend marks a save successful only after receiving this committed result.

## Persistence and migrations

Use SQLite foreign keys, transactions, full synchronization and a bounded busy timeout. Database schema version 1 introduces the record tables; version 2 introduces durable selection and generation. These database versions are distinct from interchange schema version 1. Initialize a fresh database through the migrations. Before upgrading an existing version-1 database, create and verify a SQLite backup through SQLite's backup API so committed WAL contents are included. Upgrade transactionally and reject unsupported future database versions. Do not recreate or silently discard an existing database after an opening or migration error.

Temporary real SQLite tests must exercise the populated version-1 to version-2 migration, preservation of IDs/revisions/timestamps, backup readability, fresh initialization, reopening and rejection of future versions. Runtime data and backups live in the application's OS data directory, outside the source repository. Tests use isolated temporary directories and synthetic records.

## Export and restore

Exports are UTF-8 JSON with an explicit format identifier and schema version. They contain every project, objective, action and note, plus the selected objective. They preserve record IDs, relations, revisions and timestamps. They exclude local generation tokens, storage paths, credentials and machine details. Serialize collections deterministically by ID.

File selection happens through Rust-owned native dialogs. Export writes the saved backend state to a user-selected JSON destination, using a temporary file, flush/sync and atomic replacement. The export operation must not overwrite the active database or its sidecars. A canceled dialog is a normal canceled result, not an error. No dialog should hold the repository lock while waiting for the user.

Import supports **explicit replacement**, not merging. The UI explains that the selected export replaces the local record set and requires a separate confirmation after preview. Preview reads a bounded file and validates the complete document before any mutation. It returns counts and an opaque token. Rust retains the exact validated document and the current workspace generation behind that token. Confirming does not reread a potentially changed source file.

On confirmation, reject a missing/stale token or changed workspace generation. If the incoming normalized records and selection already equal local state, return an unchanged result without duplicates or revision changes. Otherwise write and sync a uniquely named pre-restore JSON backup of the current valid record set before changing the database. If backup creation fails, abort. Then replace records and selection in one SQLite transaction, preserving imported identities and metadata exactly and assigning a new local generation. Any transaction failure rolls back. A failed import never falls back to clearing storage or regenerating IDs. The backup itself is an importable schema-version-1 export.

Validation rejects malformed JSON, unknown fields, missing required fields, invalid text or timestamps, invalid/duplicate UUIDs, nonpositive/out-of-range revisions, dangling references, duplicate actions or notes for one objective, invalid selection and unsupported exchange versions. File size, record count and text limits are explicit in the contract. Errors have stable codes and useful safe messages without raw SQL, record contents or absolute paths.

## Consequences and verification

Restoration can intentionally replace newer local edits, so the preview and explicit confirmation are required. The mandatory backup provides recovery without a speculative conflict-resolution system. Replay is safe and record identity remains stable. Clearing a note deletes that optional record; adding a note later creates a new identity.

Test valid round trips and replay, stale writes/previews, all invalid-import classes, backup failure, rollback and preservation of existing state. Native packaged acceptance must also cover saving, closing/reopening offline, exporting and confirmed restoration. Browser-only results do not satisfy the gate.

This decision does not depart from ADR-0001 or expand Stage 3a. It adds no AI, network services, telemetry, game state, completion authority, accounts, automatic ingestion or generalized merge engine.
