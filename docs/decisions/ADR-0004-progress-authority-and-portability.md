# ADR-0004 — Local progress authority and complete record portability

**Status:** Implemented for authorized Stage 3b; executed verification is recorded separately.

## Decision

Retain React, Tauri, typed Rust commands, and a private SQLite repository. Database migration 3 extends the original two migrations without rewriting them. Upgrade under an immediate transaction after a coherent SQLite backup, including committed WAL data. Copy each existing criterion into a deterministic draft lineage/slot, preserve every legacy objective UUID/revision/content value, then drop the old `objectives.criterion` column. The API's criterion text is a derived primary-lineage projection. There is one stored authoritative definition.

Progress records use strict Rust structures serialized as canonical JSON per row, with SQLite generated columns for references, uniqueness, status, and integer budget checks. This avoids two writable copies of the same fields. Individual commands insert/update only affected rows. The repository transaction coordinator validates the complete authority graph and exportability before committing. Explicit restore is the only clear/reinsert path and preserves a synchronized complete pre-restore backup.

## Criteria and evidence

Every bounded objective owns a fixed `bc-v1` four-BC lifetime budget. Positive integer allocations across its unique lineage slots sum to at most four. Draft allocations can change; locking freezes the allocation and criterion version. Subsequent material changes create prospective versions under the same lineage and consumed slot. Legacy criteria are unaccepted drafts. Deterministic legacy metadata makes equivalent legacy imports replayable without manufacturing new identities; later locally created versions and events use fresh UUIDs.

Evidence is append-only with explicit same-source supersession. The ordinary renderer route records local user reports, voluntary self-report, user-supplied artifact references, or explicitly unverified external metadata. It cannot assign actors, trusted source identity, validator privileges, or issuance amounts. Artifact identity/version and scope are bound to the exact criterion. No files are scanned or executed, and a typed reference is not independently verified content.

The production app has no trusted validator adapter. It can represent a required-validation policy, but unavailable required validation remains pending. Synthetic trusted sources exist only in compiled unit-test fixtures; importing a provenance label cannot enable a production adapter. User attestation and reviewed references remain usable without such an adapter.

Review independently enumerates all applicable current registered evidence and validators, including omitted contradictory records. Unknown, duplicated, superseded, cross-parent, cross-version, or cross-scope submitted references fail. Prospective support must be registered after locking and observed within the declared time basis. Independent-work criteria reject incompatible assistance. A snapshot preserves the exact criterion, complete applicable registry, missing/conflict status, summary/assistance, revision, and SHA-256 digest. Digests identify the recorded review basis; they do not prove real-world truth.

## Acceptance and correction

The local user-confirmed command route supplies acceptance authority. Under BEGIN IMMEDIATE, canonical command replay is checked before generation/revision staleness. Same ID/payload returns the original durable result; changed payload conflicts. Then the backend reads the locked current criterion and re-evaluates the registry against the snapshot. Acceptance, unique entitlement grant, ledger issuance, audit, and committed result are one transaction. SQLite independently enforces unique active acceptance, unique slot/lineage grants, positive integer amounts, matching ledger/grant values, and lifetime budget. The displayed wallet is a sum of committed issuance rows. Ledger entries also carry a contiguous issuance sequence; each committed command preserves its ledger prefix, allowing import validation to reconstruct its exact original balance without using timestamps as authority.

Corrections append a reason and link, updating current factual status and advancing the monotonic authority revision. Originally incorrect acceptance is retracted; valid historical work later reopened remains historical. Neither deletes a grant or reduces fictional credit. Explicit fresh review can reaccept corrected work while the original slot remains consumed. All pre-correction pending reviews become stale; portable acceptance chains must also prove each reacceptance snapshot is at or after the preceding correction revision. Wall-clock changes cannot establish this ordering. Exact command replay retains the original result while the returned workspace exposes any subsequent factual correction. There is no spending, reservation, refund, or game state.

## Portability and limits

Interchange v2 contains all work records plus criteria, slots, registry/evidence, snapshots, acceptances, corrections, grants, ledger, audit, and idempotency records. Portable wallet values are checked against ledger totals and recalculated on database load. Import validation rebuilds historical snapshots from their registered evidence revision and checks references, authority, allocations, correction links, and command digests/results. Unsupported or incomplete state fails before replacement. Top-level collections normalize by stable ID.

Legacy v1 imports convert to deterministic unaccepted drafts only in reward-free workspaces. Both preview and the locked restore transaction reject a legacy file when local grants exist. Modern replacement previews consequential counts and explicitly warns that missing reward history will be removed; confirmation creates a complete recovery backup before replacement. Replay is a no-op. Restoring an older complete backup is an explicit rollback, not a merge or a claim of externally immutable history.

All records together remain bounded to 10,000 and the complete serialized export to 10 MiB. Commands that would exceed those limits roll back. SQLite uses bounded busy waits and full synchronization. No network or external execution occurs inside a write transaction. The local command boundary does not defend against the owner directly modifying the database or a compromised operating system. Wall-clock timestamps describe observations and history; UUIDs/revisions and the registered evidence basis carry concurrency and replay identity.

The sole added direct dependency is pinned `sha2 = 0.10.9`, already present transitively in the lockfile. No framework/toolchain switch or later-stage infrastructure.
