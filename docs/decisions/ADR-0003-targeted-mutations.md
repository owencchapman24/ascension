# ADR-0003 - Persist ordinary mutations without replacing workspace rows

**Status:** Accepted for the Stage 3b Checkpoint A prerequisite.

## Context

Stage 3a ordinary writes rebuilt the record tables inside one transaction. Although the committed records retained their IDs and revisions, deleting and reinserting an objective or action would fail against dependent foreign keys or cascade away future related records. A selection change also unnecessarily rewrote every record. This must be corrected before adding progress evidence or ledger tables.

## Decision

Keep the private Rust repository and existing typed service commands. An ordinary mutation reads one authoritative workspace under an immediate transaction, checks its expected generation, applies the service transition, normalizes and validates the complete resulting record set, then persists only its difference from the original snapshot.

- New projects, objectives, actions and notes use `INSERT`.
- Changed records use `UPDATE ... WHERE id = ?`; unchanged records receive no SQL mutation.
- Clearing an optional note uses a targeted `DELETE FROM notes WHERE id = ?`. Ordinary changes cannot remove projects, objectives or actions.
- Selection changes update only the singleton `workspace_state` row. An actual mutation rotates the generation in that same transaction; a no-op executes no writes and preserves its generation.
- Revision increments, text validation, record and export size limits, stable IDs and creation timestamps remain owned by existing service/domain transitions.

The whole-workspace replacement writer is explicitly named `replace_workspace_for_restore` and is called only by confirmed import restore, after validation, generation checks and a durable recovery backup. Restore keeps its existing intentional replacement semantics and atomic rollback behavior. No database or interchange schema changes are required for this prerequisite.

## Verification

Temporary real SQLite integration tests add synthetic dependent rows with restrictive foreign keys and audit triggers on every record table and `workspace_state`. They verify exact insert/update/delete sets for project/objective creation and editing, action/note creation and editing, note clearing and recreation, selection changes and no-op saves. Unrelated dependents and record metadata survive. Stale writes produce no SQL changes; a failure at the final generation update rolls back preceding action/note writes and their audit entries. Reopening recovers the exact committed workspace.

Existing migration, validation, size-limit, export/restore, rollback and restart tests remain applicable. SQL failure-injection tests target `UPDATE` for ordinary edits and retain `INSERT` failure injection for restore. This change does not introduce progress or reward state, automatic actions, network access, or new renderer authority.
