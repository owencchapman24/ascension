# ADR-0007 — separate daily Work records and coherent local recovery

Status: accepted within the owner's Stage 4.5 assignment.

## Decision

Add an organizational projection and factual checkpoint history to the existing SQLite repository. Keep objective identity and acceptance authority unchanged. Use the existing persistent selection as focus, explicit list ordering, and active/paused/archived organization. Represent smaller-entry assistance as temporary frontend state so it cannot overwrite the main action or alter reward allocation.

Checkpoint commands include a stable command identity, expected workspace generation, factual note/reference, and saved-action/stop/resumption values. One immediate transaction validates and commits the checkpoint, any requested saved-work change, and replay result. Check committed replay before generation conflicts. Corrections append a linked account with an explicit reason. A reference stays an unverified self-report and never becomes a validator or acceptance.

Ordinary planning reuses the existing opportunity validator but creates/selects an ordinary context without the formal schedule-transition path. This is necessary because declaring a formal schedule epoch pauses active E01. The simple route fixes protocol association to null and cannot consume assignments; advanced planning keeps the original command.

Additive database v6 and complete interchange v5 carry daily organization, checkpoints, and command identity. Old formats receive deterministic defaults only when compatible and cannot replace consequential newer history. No migrations or historical protocol meanings are rewritten.

Automatic local snapshots use SQLite's backup API on a background worker, then verify SQLite structure/references and the complete application authority graph before finalizing. Snapshot status is separate from workspace generations and mutation success. At most one verified daily point is finalized per local date. Retention is the union of seven newest days and one point for each of three newest represented months. Only owned, validated files are eligible for pruning. Existing import preview/confirmation is the authority for intentional replacement.

## Consequences

Work remains useful without formal accounting, experiments, or fiction. There is one database and no generic renderer SQL/filesystem capability. Background recovery cannot turn an acknowledged edit into an apparent failed edit. Same-device backups provide rollback points, not protection against total device loss; explicit portable export remains necessary for off-device recovery.

The unchanged 0.1.0 package version is a development build identity, not a promise of downgrade compatibility. A separately reviewed release must choose its version, identify migration support, document unsigned installation and WebView2 prerequisites, and guide backup-first upgrades. Stage 4.5 performs only uniquely identified disposable installs.
