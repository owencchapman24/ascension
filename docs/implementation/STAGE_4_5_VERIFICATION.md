# Stage 4.5 verification

**Verdict: CONDITIONAL PASS.** Implementation is complete, and the final automated gates and all 76 installed Windows checks passed. The rebuilt package's first run returned a storage error during a fresh-profile restore after 70 checks. The failed transaction preserved valid local state; exact Rust and SQL replays and the unchanged-binary full rerun succeeded, but the cause remains unknown. Independent review must accept or resolve this reliability risk before merge. Human usability and productivity benefits remain unmeasured.

## Baseline — 2026-10-21

Clean `main` was exactly the requested Stage 4 merge `c27c1d728e59b04028ecc2caf4248c8564896526`. Created `feat/stage-4-5-daily-use`; main and merged history remain unchanged. Read the owner assignment, repository rules, Stage 3a–4 specifications/reports, architecture/decisions, complete interchange, all five immutable migrations, Work/progress/opportunity/world integration, recovery and native harness. Only this repository and the explicitly attached request were used; no connected account, external project, private research package, or external API was accessed.

Toolchain: Node 22.23.3, npm 10.9.9, Rust/Cargo 1.99.1 stable MSVC, Visual Studio C++ Build Tools 17.14.37806.7, WebView2 155.0.4283.62. Existing PowerShell script policy blocks `npm.ps1`; `npm.cmd` works without policy changes. `npm.cmd test` passed the baseline 118 tests in seven files. Initial parallel `cargo test --manifest-path src-tauri/Cargo.toml --locked --offline` failed compiling test binaries because Windows could not map metadata (paging file too small). Subsequent Cargo commands use `CARGO_BUILD_JOBS=1`; no machine configuration or dependency changes were made.

## Delivered behavior and boundaries

Work opens on the persisted focused objective, saved next action, stopping condition, resumption note, and last confirmed update. Reading this context requires no setup or Resume click. Resume is presentation only. **Save where I stopped** opens one compact panel with action, stop, and note prefilled; only the factual change needs ordinary new input. Supporting references and historical details remain optional. Objective management, opportunities, evidence/acceptance, study settings, recovery, and World remain reachable through secondary controls. World stays lazy and does not load to display Work.

Organization is separate from accomplishment authority: active, paused, and archived objectives retain all identities and history. Active priorities can move up/down with keyboard controls. Inactive objectives retain their positions; active reordering permutes only active positions. Pausing/archiving the focused objective selects the first remaining active priority, or no focus when none remain. Restoring an objective does not silently focus it. Migration defaults existing objectives to active in their prior stable-ID display order and preserves the saved selection. `selected_objective_id` remains the sole persisted focus authority.

Checkpoints have stable identities, self-report provenance, action/revision snapshots, optional unverified references, and append-only corrections. Rust commits the checkpoint, exact replay input/digest, and included action/note changes in one immediate SQLite transaction with targeted writes. Exact command replay returns the committed checkpoint before stale-generation checks; changed input under the same identity conflicts. Failure leaves saved state intact and the frontend draft available. An uncertain transport response retains the exact command for retry. A refreshed focus does not move an unsaved draft to another objective.

The optional support menu provides a temporary smaller entry tied to the same objective, explicit selection of another active priority, and rest. Temporary entries remain presentation state and do not overwrite the main action. Rest and Resume create no records, penalties, timers, rewards, or study outcomes. Ordinary opportunity planning now works without manual epoch setup: Rust creates/reuses a durable ordinary epoch for the timezone and delegates to existing prospective validation. It never enrolls a study, consumes allocation, or pauses an active E01. Advanced epoch/protocol choices remain explicit. Existing outcome timing, uncertainty, missingness, and correction rules remain intact.

Database schema is **6**, through additive `006_daily.sql`; migrations 001–005 are unchanged. Complete interchange is **5**, including organization, checkpoints, replay identities, and all prior authority. v1–v4 imports retain compatible defaults and prior restrictions, and cannot erase consequential daily history. Current envelopes allow **12,500 records / 12 MiB**; older envelopes retain **10,000 / 10 MiB**, including raw input size. This narrowly sized headroom prevents mandatory planning metadata from making a previously valid Stage 4 workspace unmigratable. All new rows still count. ADR-0007 records the decision.

`bc-v1`, the fixed H1 instrument, frozen E00/E01 definitions and allocations, `river-reach-v1`, six sites, 24-BC spending cap, funded capacity, reservations, undo, and original Chronicle remain unchanged. Daily operations have no grant or evidence-acceptance path. A modern restore preserves issuance identities rather than issuing again.

## Automatic recovery and real-SQLite evidence

The first committed data change of a local day queues a private background worker. Opening, reading, and no-op commands create no snapshot. The worker pins a read transaction and uses SQLite backup, including committed WAL, to a temporary file. Before finalization it checks current schema, quick check, foreign keys, and the complete application authority graph; flushes; and finalizes without overwriting an existing daily point. It never holds the service lock during backup filesystem work.

Owned points carry an internal identity/date marker and `daily-v1-YYYY-MM-DD.sqlite3` filename. Retention is the union of the newest seven used dates and the newest point in each of three newest represented calendar months. A test creates 14 real dated SQLite snapshots and verifies the exact nine-point retained union. Unused dates create nothing. Unknown files, incomplete files, unsupported snapshots, and migration/pre-restore copies are retained. Pruning occurs only after verified protection and rechecks identity immediately before deletion. Symlinks and Windows junctions are refused.

All **11 recovery tests** passed: coherent standalone WAL snapshots; no-op/day immutability; exact retention; failed/partial/mimic rejection without pruning; invalid schema/FK/authority rejection; destination failure with a successful saved edit; truthful status after a formerly verified snapshot becomes corrupt; concurrent same-day requests; Windows exclusive locks; junction refusal; and full cross-profile recovery. The full authority case uses authentic synthetic **8 BC**, a correction, frozen configured E01 history, **4 BC spent / 4 reserved**, and an opportunity; exact recovery and acceptance replay cannot remint. Tampered ledger/world copies that still pass SQLite quick check/FK fail application validation.

Backup warnings are independent of edit success. Retry does not change workspace generation or records. Recovery preview uses existing explicit replacement confirmation and saves displaced readable state first. Recovery export works without opening a corrupt primary and uses a native user-selected Save destination. No automatic restore occurs. Same-device recovery does not protect against device loss; portable off-device export remains explicit.

Three capacity regressions passed actual DB5-to-DB6 migrations: 2,499 objectives expand **9,997 to 12,496 records** with exact preservation; a **10,485,744-byte** old payload (10 MiB minus 16) migrates and its modern export above 10 MiB round-trips. Legacy 10,001 rows remain rejected even when their upgraded count would fit the current cap. Current 12,500 rows pass and 12,501 fail. Raw legacy/current byte boundaries accept the exact limit and reject one extra byte without mutation.

## Automated gates

Final backend commands used the existing lockfile and cached dependencies:

```powershell
$env:CARGO_BUILD_JOBS = '1'
cargo test --manifest-path src-tauri/Cargo.toml --locked --offline -- --test-threads=1
cargo clippy --manifest-path src-tauri/Cargo.toml --locked --offline --all-targets -- -D warnings
cargo fmt --manifest-path src-tauri/Cargo.toml -- --check
```

**180 Rust tests passed, zero failed; four explicit performance benchmarks were ignored in the ordinary correctness run.** Breakdown: 128 library, 3 capacity, 11 portability, 1 Stage 3c bounded-history, 4 Stage 4 integration, 3 storage-opening, 3 targeted-mutation, 27 workspace. No correctness test is ignored. Full command elapsed 163.89 seconds including 45.30 seconds compilation; group times 55.94/28.53/2.73/2.84/2.73/0.49/0.67/17.46 seconds. Strict all-target Clippy passed in 15.19 seconds; formatting passed. Logs: `.artifacts/stage4-5-rust-tests.log`, `.artifacts/stage4-5-clippy.log`. The prior 179-test logs are retained with `-before-wal-preflight` suffixes.

`npm.cmd run typecheck`, `npm.cmd test`, and `npm.cmd run build` passed. **146 frontend/client tests passed across eight files**, zero failures, in 22.76 seconds after the recovery-screen correction. Tests cover lost-response replay, editable rejected drafts, concurrent focus/archive refresh, absent original objectives with copy/discard, lifecycle focus restoration, voluntary support, ordinary/protocol separation, prior verified recovery during warnings, recovery controls surviving successful/cancelled export with failed primary storage, and inherited behavior. Final production build transformed 40 modules in 848 ms; main JS is 367.40 kB (107.68 kB gzip), with River Reach still a separate lazy chunk. Logs: `.artifacts/stage4-5-frontend-tests-final.log`, `.artifacts/stage4-5-recovery-ui-typecheck.log`, and `.artifacts/stage4-5-native-final-verified.log` (includes the production build and first installed attempt).

Interaction counts are structural, not human timings: reaching saved work needs zero navigation after launch; a routine checkpoint needs its opening control, one factual note with prefilled action/stop/note review, and Save; ordinary planning needs the opportunity disclosure, Plan, review of prefilled action/future cue/available time, and Confirm. Neither path requires an acceptance review, World, study configuration, timer, or questionnaire.

## Installed Windows verification

The final package passed **76 recorded installed WebView2 lifecycle checks** on its full unchanged-binary rerun, including 24 successful Work launches, two deliberately failed-primary launches, and 71 native file-dialog operations. All 24 offline probes supported denial and passed. The earlier failed run remains separately reported below. The suite retained the earlier create/edit/save/restart, interrupted import, invalid-import rejection, populated migrations, reward acceptance/correction, frozen study allocation/missingness, H1, and all three River Reach route/Chronicle checks. Stage 4.5 added organization and focus, compact checkpoint/correction/replay, stale-draft preservation, voluntary support, ordinary capture without prior epoch setup, failed-backup retry, exact modern portability, and complete recovery. Native evidence is separate from frontend mocks.

The final recovery record set contains 17 objectives, two checkpoint accounts, 17 legitimate synthetic grants totaling 68 issued BC, four historical assignments with no active study, six funded sites, 24 BC spent, and the original Beacon Mesh Chronicle. Its 230,539-byte complete JSON was restored into distinct disposable profiles, exported for exact structural comparison, recovered through the validated daily SQLite snapshot, and checked again after restart. SQLite quick check, foreign keys, checkpoint count, cumulative ledger, and world command counts were checked independently. Same-day retry preserved the snapshot bytes. The simulated multi-date retention union is verified by the real-SQLite tests described above; the installed run verifies one-day immutability.

Both nonempty invalid bytes and zero-byte primary truncation produced visible failure. Native Save exported exactly the complete recovery record set without changing either failed primary, including after closing the app. Recovery controls remained available after export. The original closed main/WAL/SHM family was then restored together, and the final reopened export matched exactly. No automatic destructive restore occurred.

The unique verification package used its own identifier, installation directory, and repository-local profile. Release compilation took **4m 04s**, and the installer is **2,949,054 bytes** (2.81 MiB). Install and uninstall both returned zero; the test executable and uninstaller were absent afterward. The full rerun used the same package and unmodified harness with fresh disposable data. The normal installation/profile was untouched. Historical Stage 3a–4 verification reports are unchanged.

| Artifact | SHA-256 |
| --- | --- |
| Verification installer | `0E4677D4A8E07826763AA196369520920A49395F93AB7942E00032314347A7CA` |
| Installed executable | `E326250C5F1756F653CDB59E38577C4F314EA2CA453BEFDF5613D1ACF99CC152` |

Ignored synthetic evidence is under `.artifacts/native-82bb27e4bbf6450f9457b585ab3eff0f/`: `native-result.json`, `package-result.json`, `source-result.json`, `daily-timings.json`, complete exports, SQLite snapshots, and screenshots. All **87 source/test/dependency/harness fingerprints** matched the recorded build/test inputs after completion. Build/first-attempt output is `.artifacts/stage4-5-native-final-verified.log`; the successful full rerun is `.artifacts/stage4-5-native-final-rerun.log`. The bounded local reinstall wrapper is `.artifacts/rerun-stage4-5-native.ps1`. Earlier failed attempts and the isolated WAL reproduction were retained for diagnosis; they are not reported as passes.

The repeatable installed gate uses the existing bounded harness. For a shell that blocks script-file execution, the verified invocation is:

```powershell
& ([scriptblock]::Create((Get-Content -LiteralPath scripts/verify-native.ps1 -Raw))) -Repository (Get-Location).Path
```

The desktop test process needs permission to launch its uniquely identified Windows package. No execution policy, firewall, normal profile, or toolchain configuration is changed.

## Performance and accessibility

| Installed measurement | Result |
| --- | --- |
| Process start to usable Work, 24 launches | 927.91–1,199.48 ms; median 1,026.46 ms; first launch 1,199.48 ms |
| Keyboard objective reorder to durable state | 60.68 ms |
| Checkpoint Save to confirmed UI | 132.56 ms |
| Ordinary opportunity Confirm to confirmed UI | 125.88 ms |
| Ordinary opportunity command through IPC | 16.80 ms |
| Saved-action command through IPC | 68.06 ms |
| Confirmed snapshot restore through IPC | 54.27 ms |
| Complete restore including native dialog automation | 1,664.92 / 1,618.39 ms |

The measured Work launches met the approximate three-second target, and measured routine mutations remained below 200 ms. These are synthetic engineering observations on this machine; individual operation rows are single samples, filesystem caches were not controlled, and dialog timings include automation. They are not maximum-capacity latency guarantees or human task times. The background backup worker does not hold the service lock during filesystem work; installed saves remained acknowledged and usable through protection failure and retry. No claim of exhaustive frame-time profiling is made.

Installed screenshots at normal 1080×820 and narrow 660×850 window settings were visually inspected. The saved action, stopping condition, note, and confirmed-update time remain primary and readable, without horizontal overflow. The compact checkpoint panel fits its normal viewport after focus/scroll and exposes the original objective, explicit unsaved state, prefilled context, Save, and Discard. Keyboard activation, lifecycle focus restoration, modal Tab/Shift+Tab containment, Escape, guarded discard, visible errors, and voluntary support were exercised. Reduced-motion preference was enabled for the checkpoint flow. Both recovery error screens visibly retain actionable controls and the export result. No screen-reader audit, full WCAG audit, or physical power-loss test is claimed.

## Problems found, corrected, and investigated

- Parallel Rust compilation exhausted the machine's paging-file budget. Serial builds passed without system changes.
- The first restricted installed launch timed out before WebView2 exposed its test endpoint (zero native checks ran). The same installer/binary launched with desktop execution permissions; the failed disposable profile and result were retained, and the full harness restarted with a fresh disposable profile.
- Mandatory additive planning rows could exceed the old capacity for a valid nearly full workspace. Version-specific headroom and actual boundary migrations now cover both record and byte limits.
- Authoritative focus refresh could remount an unsaved checkpoint. The draft now keeps temporary original-objective context while saved focus remains authoritative.
- Backup inventory could briefly look idle during queued work, stopping UI polling; status now remains running until the worker finishes.
- A failed retry could retain a stale claim about a newly corrupted snapshot. Retry now refreshes verified availability without deleting or replacing it.
- Legacy downgrade fixtures needed to remove the new tables, and migration/pre-restore tests needed to distinguish their copies from automatic snapshots. Original authority assertions remain.
- Native ordinary-epoch reuse must use the same timezone as the UI; the harness now does so rather than assuming UTC.
- The first desktop run reached ordinary backup recovery but missed the next Open dialog: scrolling toward Import exposed the short cue and its existing delivery write disabled/reflowed controls between coordinate measurement and click. The harness now explicitly shows the cue, waits for persisted delivery and enabled controls, and reruns the full suite. No acknowledged application write was lost and the binary was unchanged.
- The next run completed exact snapshot recovery/restart and reached the corrupt-primary recovery Save dialog. Its new exact title was missing from the test helper's allowlist; that title and Save-control mapping were added while preserving the isolated-process/path restrictions. The full suite was rerun against the same binary.

- The next installed run exported the exact complete recovery graph, but its corruption fixture had moved only the primary file while leaving committed WAL/SHM. A minimal SQLite reproduction confirmed that opening/closing this mixed family reconstructs the primary from committed WAL. The fixture now preserves the entire closed database family before injecting corruption, then restores that family together. Strict before/after byte comparisons remain. This was a fixture error, not automatic application restoration.
- Successful recovery export cleared the initial storage error and left an unusable empty screen. The load error now persists independently until a workspace actually loads. Both successful and cancelled exports retain recovery controls; focused tests cover subsequent retry.
- Final review found the inherited initializer accepted a pre-existing zero-byte/schema-zero primary as new storage. A fail-closed guard and real-SQLite regressions distinguish an absent new profile from damaged existing storage. The installed drill now includes both nonempty corruption and zero-byte truncation.
- Final audit added read-only, transaction-pinned schema/quick-check/foreign-key inspection before opening an existing primary for writes, with the existing writable checks retained. This avoids exposing known physical corruption to a writable connection's close-time WAL handling. A real-workspace partial-WAL test verifies repeated failed opens preserve the main file, committed WAL, recovery snapshot, and exact recovery into a separate profile. That smaller Rust fixture also passed before this defensive guard; it is preventive coverage, not a claimed red-to-green reproduction. The independent Node SQLite reproduction and earlier inconsistent-family installed fixture establish the observed checkpoint hazard. SQLite may update SHM coordination metadata; no universal byte-preservation guarantee against concurrent external file damage is claimed.
- The rebuilt package's first full run completed 70 checks, then a confirmed modern import into a fresh profile returned `storage_error`. The same complete file had already restored and re-exported exactly in another profile. A pre-replacement backup was saved; the failed destination retained zero objectives/checkpoints, clean quick-check/foreign-key results, and a visible error. Exact Rust service replays succeeded against a truly absent database and copies of both retained profiles; independent SQLite statement replays also committed successfully. These diagnostics are separate from the 180-test suite. No production or harness change was justified, and the full suite was rerun with the unchanged installer and fresh disposable data. The failed run is retained under `attempt-fresh-import-failure/`, and diagnostics under `.artifacts/restore-diagnostic-20261021-1/`. The underlying error remains unidentified: this is neither a proven harness failure nor a demonstrated data-loss defect, and successful retries do not resolve its cause.

## Files and local history

New implementation: `src/work/` (DailyWork, CheckpointEditor, ObjectiveManager, BackupPanel and tests), `src-tauri/src/daily.rs`, daily tests, `backup.rs`, backup tests, migration 006, capacity and storage-opening integration tests, and `scripts/native-daily.mjs`. Updated integration: App/CSS, typed client and fixtures/tests, opportunity/study components, Rust commands/service/storage/domain/study/world identity validation, existing portability/workspace/integration tests, native regression/progress/study/world scripts, and the bounded native-dialog helper. Documentation: AGENTS, architecture boundaries, Stage 4.5 spec, ADR-0007, contracts, README, this report, and `RELEASE_AND_RECOVERY.md`.

Verified local implementation history: `ff94289` (authorized scope/specification), `4377de8` (daily persistence/recovery), `78f58b8` (daily Work frontend), `1abe406` (failed-primary preservation and usable recovery controls), `de81f31` (76-check installed Windows regression harness), and `f75362f` (read-only physical-integrity checks before writable storage access). This report and the remaining guidance form the final documentation commit; its hash is included in the completion handoff. No dependencies, lockfiles, capability permissions, historical migrations, workflows, package versions, or historical verification reports changed.

Engineering tests establish behavior on synthetic records, not human checkpoint time, cognitive burden, reduced procrastination, or E00 feasibility. The 60-second checkpoint and two-minute ordinary capture targets require human use. E01 still requires a separately versioned H1 exposure-duration protocol correction before genuine enrollment.

## Closure and remaining limits

`feat/stage-4-5-daily-use` is ready for independent review. Merge readiness is conditional on review and explicit acceptance or resolution of the unexplained native restore error above; an unconditional merge recommendation is withheld. The failed transaction preserved state, but its cause remains an unresolved reliability risk. Main remains exactly `c27c1d728e59b04028ecc2caf4248c8564896526`. Final delivery includes only local commits and a clean tracked working tree; ignored synthetic evidence remains local. No push, merge, release, deployment, workflow, AI integration, Stage 5 work, normal-profile upgrade, external account access, or real study activation occurred.

Remaining limitations are explicit: automatic recovery is on the same device and protects the first successfully captured change of each used day, so later same-day work needs explicit export for a newer recovery point. Only verified owned daily snapshots are bounded; migration/pre-restore/unknown files remain retained. The installer is unsigned and needs installed WebView2. Migration is forward-only, and the unchanged development version does not establish downgrade compatibility. Offline denial covered WebView2 with a refused process proxy and verified renderer probe; it did not change OS/Rust network policy. A failed first initialization can leave an uninitialized file that now correctly requires an explicit recovery decision rather than silent reset. Full erasure remains a separately reviewed namespace/tombstone design.

The product is **conditionally ready for a short owner-initiated genuine E00 feasibility observation after independent review accepts or resolves the restore reliability risk and an authorized backup-first installation**. Human observation must not substitute for engineering investigation of that error. A later E00 observation should measure actual checkpoint/capture time, resumption friction, cognitive burden, and voluntary nonuse; it may falsify the usability assumptions. No successful E00 result or reduction in procrastination is claimed. Genuine E01 remains gated on the separately versioned H1 protocol correction. The implementation agent has not enrolled the owner in either study.

## Complete change inventory

47 files: 17 added (A), 30 modified (M). Generated artifacts and private data are excluded.

```text
M	AGENTS.md
M	contracts/README.md
A	docs/decisions/ADR-0007-daily-work-and-recovery.md
A	docs/implementation/RELEASE_AND_RECOVERY.md
A	docs/implementation/STAGE_4_5_VERIFICATION.md
M	docs/specs/ARCHITECTURE_BOUNDARIES.md
A	docs/specs/STAGE_4_5.md
M	README.md
A	scripts/native-daily.mjs
M	scripts/native-dialog.ps1
M	scripts/native-progress.mjs
M	scripts/native-regression.mjs
M	scripts/native-study.mjs
M	scripts/native-world.mjs
A	src-tauri/migrations/006_daily.sql
A	src-tauri/src/backup_tests.rs
A	src-tauri/src/backup.rs
M	src-tauri/src/commands/mod.rs
A	src-tauri/src/daily.rs
A	src-tauri/src/daily/tests.rs
M	src-tauri/src/domain/mod.rs
M	src-tauri/src/lib.rs
M	src-tauri/src/service.rs
M	src-tauri/src/storage/mod.rs
M	src-tauri/src/study.rs
M	src-tauri/src/study/tests.rs
M	src-tauri/src/world.rs
M	src-tauri/src/world/tests.rs
A	src-tauri/tests/daily_capacity.rs
M	src-tauri/tests/portability.rs
M	src-tauri/tests/stage4_integration.rs
A	src-tauri/tests/storage_open.rs
M	src-tauri/tests/workspace.rs
M	src/__tests__/App.test.tsx
M	src/App.css
M	src/App.tsx
M	src/platform/__tests__/client.test.ts
M	src/platform/client.ts
M	src/progress/__tests__/fixtures.ts
M	src/study/__tests__/StudyPanel.test.tsx
M	src/study/OpportunityEditor.tsx
M	src/study/StudyPanel.tsx
A	src/work/__tests__/DailyWork.test.tsx
A	src/work/BackupPanel.tsx
A	src/work/CheckpointEditor.tsx
A	src/work/DailyWork.tsx
A	src/work/ObjectiveManager.tsx
```
