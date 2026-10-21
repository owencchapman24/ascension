# Stage 4.5 — daily Work and local reliability

Owner-authorized implementation after Stage 4 merge `c27c1d728e59b04028ecc2caf4248c8564896526`, on `feat/stage-4-5-daily-use`. Local verified commits are authorized. Push, merge, release, deployment, CI workflows, AI, Stage 5, expansion of River Reach, real study activation, and changes to the normal installed profile are excluded.

## Everyday workflow

Open directly to the persistent focused objective, saved next action, observable stopping point, resumption note, and last confirmed update. Reading the action is sufficient to begin work. Voluntary Resume work changes presentation only; Save where I stopped opens a compact factual checkpoint. Work never requires a timer, questionnaire, opportunity, acceptance claim, World, or network access.

Objective organization has active, paused, and archived states, with explicit keyboard-accessible priority ordering. The existing selected-objective identity is the only focus authority. Archiving is independent of factual completion. Preserve objective IDs/revisions and all criterion, evidence, reward, study, and world links. Legacy objectives retain deterministic prior list order and selection. Inactive focused work has a visible fallback; stale concurrent ordering commands fail.

A checkpoint captures a factual note, optional unverified reference, self-report provenance, timestamp, objective identity, and action/stop/note snapshot. Save it and explicitly included saved-work changes in one Rust transaction. Corrections append with a reason and predecessor identity. Stable command IDs recover exact committed retries before stale checks; changed payloads conflict. Failed saves retain frontend drafts. Checkpoints neither become registered evidence nor accept work or mint BC.

Optional initiation support offers a smaller user-written step with its own stopping point, explicit choice of another active priority, and rest. The smaller step is temporary presentation, retains the parent, and preserves the original saved action. Rest records no failure, debt, outcome, or lifecycle change. None changes locked criteria, reward slots, or experimental initiation.

Ordinary opportunity planning derives the saved action and criterion and asks for a meaningful first action, future cue, timezone context, and available time. The backend may create an honestly timestamped ordinary epoch in the same transaction. It never enrolls, pauses, assigns, or rerandomizes a study. Advanced declaration and formal protocol controls remain available. Existing exact/approximate/unknown timing, nonstarts, late starts, missingness, denominators, and append-only corrections remain authoritative.

## Durability and portability

Add migration 006 after immutable migrations 001–005. Database schema becomes v6; complete interchange becomes v5. Separate daily-use organization/checkpoints from existing reward and study state. Modern export preserves the full graph and exact new history. Compatible legacy files initialize honest empty checkpoint history and deterministic organization; older exports may not erase consequential Stage 4.5 history. Preserve previous reward/study/world legacy restrictions, strict bounded input validation, preview/confirmation, pre-replacement backup, and transactional rollback.

After a day's first successful changed-data operation, schedule one coherent local SQLite snapshot per local calendar day. Use the SQLite backup API, a temporary destination, complete integrity validation, and finalized output. Retain the seven newest daily points plus one point from each of the three newest represented months; a point may serve both categories. Only positively identified subsystem-owned snapshots may be pruned, and never after an unverified backup. Migration, pre-restore, unknown, and user files remain untouched. No snapshots are invented on unused days.

Snapshot failure never undoes or misreports a committed work edit. Display nonblocking status and retry. Loading Work must not wait for backup enumeration or validation. Recovery requires validated preview and explicit confirmation, preserves displaced state when feasible, and cannot silently replace a corrupt database. User-selected complete JSON export remains the off-device protection path; same-device snapshots do not protect against device loss.

## Preserved boundaries

The `bc-v1` four-credit lifetime parent cap and consumed entitlements remain unchanged. Acceptance alone may issue BC; corrections retain courtesy credit without reminting. H1 uses cumulative issuance. River Reach retains `river-reach-v1`, six sites, 24 BC, reservation/spend conservation, free undo, and immutable Chronicle. E00/E01 frozen histories, assignments, and outcome rules remain unchanged. The inherited H1 post-window exposure-duration issue needs a separately versioned protocol correction before real E01 enrollment.

Archiving hides work without deleting history. Factual corrections preserve prior accounts. Full erasure requires a future coherent namespace-reset/tombstone decision; this milestone adds no casual cascading delete or reset.

## Verification gates

Maintain all Stage 3a–4 tests and reports. Add real temporary SQLite migration, stale-write, checkpoint replay/rollback/correction, ordinary protocol-isolation, WAL backup, retention/failure, full recovery, and exact modern/legacy portability tests. Run frontend tests, typecheck/build, Rust tests/fmt/strict Clippy, and isolated installed Windows lifecycle/offline/keyboard/normal-and-narrow visual verification. Synthetic snapshots and exports remain ignored local artifacts.

Measure startup, checkpoint, reorder, ordinary planning, and restore. Targets are roughly three seconds to usable Work and 200 ms for routine operations on the test machine. Engineering timings do not establish human burden: the approximately 60-second checkpoint and two-minute opportunity targets require actual human observation. Do not claim productivity benefit or E00 feasibility from synthetic tests. Record actual results and limitations in the Stage 4.5 verification report.
