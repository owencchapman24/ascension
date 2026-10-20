# Stage 3b — progress, acceptance, and fictional issuance

Owner-authorized scope following merged Stage 3a PR #1. This public specification summarizes the implementation assignment; it contains no private research or personal records. It extends the historical Stage 3a exclusions only for the features below. It does not establish behavioral effectiveness.

## Required boundaries

Keep the saved next action primary. Distinguish intended action, reported activity, evidence, accepted accomplishment, and fictional reward. React submits typed commands; Rust and one private SQLite database own validation, identity, transitions, and durable settlement. No AI, accounts, network dependency, game, spending, external execution, scanning, experiments, or later-stage infrastructure.

Before dependent tables, replace ordinary whole-workspace rewrites with targeted transactional INSERT/UPDATE operations. Selection must not rewrite parents. Only explicit validated whole-workspace restore may replace records. Preserve optimistic concurrency and changed-only revisions.

## Criteria and rewards

Preserve parent UUIDs, legacy content/revisions, criterion lineage, criterion version, immutable reward slot, policy `bc-v1`, and a four-BC lifetime parent budget. Legacy criteria become unaccepted drafts, never retrospective accomplishments. Maintain one canonical criterion definition. Drafts may change; locked definitions retain history. Prospective new versions keep the same entitlement. Default allocation is four BC; meaningful split criteria may predeclare positive integer allocations totaling at most four. Versions, renames, actions, corrections, and imports cannot renew allocations.

## Evidence and acceptance

Capture concise factual progress with source/provenance, scope/artifact/version, observation/applicability metadata, uncertainty, reference, and assistance. User attestations and references remain honestly labeled. No user-entered text confers trusted-validator authority. A trusted-validation policy remains unsupported when its required validator is unavailable.

Rust independently enumerates the registered applicable evidence/validator set; submitted IDs never define completeness. Reject unknown/duplicate/inconsistent references. Consider parent, lineage/version, artifact/version, scope, supersession, and registry revision. Surface missing evidence and material conflicts. Persist a stable digest-bearing review snapshot; re-evaluate current authority at acceptance. Stale evidence or criterion versions cannot settle.

Acceptance authority comes from the local user-confirmed command route, never a client role field. Strictly validate schemas, numbers, enums, IDs, and unexpected fields. Under BEGIN IMMEDIATE, check canonical command ID/digest replay before stale checks. Identical committed retries return the original result; changed payloads conflict. Atomically commit acceptance, deterministic eligible grant, issuance ledger, audit event, and idempotent result. Ledger-derived wallet; no mutable balance authority. SQLite uniqueness, references, checks, and budget guards independently protect integrity. Use bounded busy waits and no external work inside transactions.

## Corrections and portability

Append factual corrections with reason/time. Distinguish originally incorrect acceptance from valid historical work later reopened. Preserve the historical criterion/evidence basis. Retain granted BC as fictional courtesy credit and keep the entitlement consumed. Reacceptance can restore factual status but cannot remint.

Database migration versions are separate from interchange versions. Preserve old migrations. Back up and validate populated Stage 3a upgrades transactionally; failure cannot recreate or erase data. Interchange v2 must include complete criteria, evidence, snapshots, acceptances, corrections, slots, grants, ledger, audit/idempotency identities, work records, and selection. Validate relationships, history, limits, and totals before restore. Replay preserves IDs and does not award. Legacy v1 can become draft criteria in a compatible reward-free workspace; reject replacement of earned history. Preview consequential counts/warnings, confirm explicitly, and create a complete durable recovery backup. Every successful mutation must remain exportable.

## Verification gates

A: targeted storage and real-SQLite dependent-identity regressions. B: identities, locking, split budgets, populated legacy migration. C: complete applicable evidence and stable snapshots. D: authority, idempotency, stale/conflicting evidence, concurrent settlement, fault rollback. E: corrections/reacceptance and complete modern/legacy portability. F: accessible minimal UI, pending/reject/accept/correct workflow, hidden fictional progression. G: full Rust/UI/client/type/build/fmt/Clippy suites and actual isolated installed-Windows verification.

Test adversarial numbers/references, omitted contradictory validators, missing validators, assistance/domain isolation, criterion/artifact changes, command replay after restart/restore, concurrent connections, bounded lock contention, rollback at transaction boundaries, four-BC invariant, immutable entitlements, ledger reconstruction, migration/backup/restore failures, malformed imports, export size boundaries, keyboard operation, and controlled offline workflows. Use only synthetic disposable data. Record actual commands/counts, failures and fixes, native package identity, limits, and unverified conditions in `docs/implementation/STAGE_3B_VERIFICATION.md`. Never report a consequential integrity gate passed without executed evidence. Stop at Stage 3b.
