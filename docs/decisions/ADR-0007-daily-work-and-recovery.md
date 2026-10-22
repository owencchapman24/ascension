# ADR-0007 — separate daily Work records and coherent local recovery

Status: accepted within the owner's Stage 4.5 assignment.

## Decision

Add an organizational projection and factual checkpoint history to the existing SQLite repository. Keep objective identity and acceptance authority unchanged. Use the existing persistent selection as focus, explicit list ordering, and active/paused/archived organization. Represent smaller-entry assistance as temporary frontend state so it cannot overwrite the main action or alter reward allocation.

Checkpoint commands include a stable command identity, expected workspace generation, factual note/reference, and saved-action/stop/resumption values. One immediate transaction validates and commits the checkpoint, any requested saved-work change, and replay result. Check committed replay before generation conflicts. Corrections append a linked account with an explicit reason. A reference stays an unverified self-report and never becomes a validator or acceptance.

Ordinary planning reuses the existing opportunity validator but creates/selects an ordinary context without the formal schedule-transition path. This is necessary because declaring a formal schedule epoch pauses active E01. The simple route fixes protocol association to null and cannot consume assignments; advanced planning keeps the original command.

Additive database v6 and complete interchange v5 carry daily organization, checkpoints, and command identity. Old formats receive deterministic defaults only when compatible and cannot replace consequential newer history. No migrations or historical protocol meanings are rewritten.

The modern envelope permits 12,500 total records and 12 MiB; legacy envelopes retain 10,000 records and 10 MiB. Stage 4 objective authority requires at least four records per objective (objective, parent, slot, criterion), so at most 2,499 objectives can share a valid 10,000-record workspace with a project. Adding one planning row each fits the bounded extra 2,500 records, with 2 MiB of serialization headroom. Counting the rows preserves resource limits while avoiding rejection of valid Stage 4 data merely because migration adds organization. Boundary fixtures verify count and byte compatibility.

Automatic local snapshots use SQLite's backup API on a background worker, then verify SQLite structure/references and the complete application authority graph before finalizing. Snapshot status is separate from workspace generations and mutation success. At most one verified daily point is finalized per local date. Retention is the union of seven newest days and one point for each of three newest represented months. Only owned, validated files are eligible for pruning. Existing import preview/confirmation is the authority for intentional replacement.

## Consequences

Work remains useful without formal accounting, experiments, or fiction. There is one database and no generic renderer SQL/filesystem capability. Background recovery cannot turn an acknowledged edit into an apparent failed edit. Same-device backups provide rollback points, not protection against total device loss; explicit portable export remains necessary for off-device recovery.

The unchanged 0.1.0 package version is a development build identity, not a promise of downgrade compatibility. A separately reviewed release must choose its version, identify migration support, document unsigned installation and WebView2 prerequisites, and guide backup-first upgrades. Stage 4.5 performs only uniquely identified disposable installs.
