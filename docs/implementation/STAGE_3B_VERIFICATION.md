# Stage 3b implementation verification

Status: **PASS for the authorized Stage 3b scope**, verified 2026-10-19. All consequential acceptance, issuance, migration and restore gates have executed evidence below. Dates use the owner's America/New_York timezone. Only synthetic records and disposable repository-local native profiles or temporary SQLite directories are used. The normal installed application profile is not opened. This verdict is bounded by the explicitly listed limitations.

## Baseline — 2026-10-19

The working tree was clean on `feat/stage-3a-offline-work` at `11c30c0`. Fetched only this repository's `origin/main`, fast-forwarded local main to merge `6a37ba08839314696c7438040d80ef155a8960ff`, and created `feat/stage-3b-evidence-ledger`. No history rewrite, push, merge to main, release, or publication. Read the owner's assignment, AGENTS, Stage 3a specifications, architectural boundaries, ADRs, contract, source/tests, and native harness. Read the actual PR #1 review; it confirms targeted persistence, criterion locking/versioning, and complete ledger portability as Stage 3b prerequisites.

Environment: Windows PowerShell 5.1; Node 22.23.3; npm 10.9.9; Rust/Cargo 1.99.1, stable x86_64-pc-windows-msvc; Visual Studio Build Tools 17.14.42 (installation 17.14.37806.7); WebView2 155.0.4283.62. Existing Rust tools require adding the standard Cargo bin directory to the process PATH. `npm.ps1` is blocked by the existing execution policy; `npm.cmd` works without changing policy. No toolchain installation or framework switch.

| Executed baseline command | Result |
| --- | --- |
| `npm.cmd test` | 30 passed: 24 UI, 6 client |
| `npm.cmd run typecheck` | Passed |
| `npm.cmd run build` | Passed; 19 modules |
| `cargo test --manifest-path src-tauri/Cargo.toml --locked --offline` | 37 passed: 10 unit/command/service, 27 real-SQLite integration |

## Checkpoint A — targeted storage

Passed before adding dependent Stage 3b tables. Ordinary edits use changed-record INSERT/UPDATE and explicit note clearing; only confirmed restore may replace the workspace. Three real SQLite regressions use dependent foreign keys and audit triggers to detect deletes, reinserts, and unrelated writes, including stale/late-failed mutations and restart. Existing fault-injection triggers aimed at INSERT during ordinary updates were moved to UPDATE so they continue testing the intended transaction failure. See ADR-0003.

| Executed Checkpoint A command | Result |
| --- | --- |
| `cargo test --manifest-path src-tauri/Cargo.toml --locked --offline` | 40 passed: 10 unit, 3 targeted-mutation, 27 workspace |
| `cargo fmt --manifest-path src-tauri/Cargo.toml -- --check` | Passed |
| `cargo clippy --manifest-path src-tauri/Cargo.toml --locked --offline --all-targets -- -D warnings` | Passed |
| `git diff --check` | Passed |

## Gate tracking during implementation

At Checkpoint A, criteria/migration, evidence completeness, atomic settlement, correction/portability, UI, and installed-Windows Stage 3b verification remained unverified. The later executed results below supersede that intermediate status. No routine QA is assigned to the owner.

## Integration review and intermediate verification

Database migration 3 moves canonical criterion definitions out of the legacy objective column into versioned progress records. Interchange v2 includes the full authority graph and historical command replay inputs/results. ADR-0004 records the decisions and explicit production-source authority limits.

Executed intermediate checks: 42 UI/client tests passed (26 App, 9 progress-panel, 7 client), typecheck and frontend build passed (20 modules); the 27 adapted workspace and nine new portability tests passed; all three targeted-mutation regressions passed against migration 3; all ten existing command/service tests passed. These are intermediate runs while the core settlement regressions and further review fixes are being completed, not the final Stage 3b gate.

Failures/findings retained:

- Initial integration compilation found unsupported rusqlite `u64` conversions for registry revisions. Explicit checked integer conversion now bounds revisions to JavaScript's exact integer range. No dependency upgrade was needed.
- Frontend typechecking rejected `.at()` under the existing ES2021 target; replaced it with compatible array indexing. One test's asynchronous dirty-draft callback assertion needed to wait for the effect. A follow-up allocation and focus regression brought the UI suite to 42.
- A corrected acceptance could initially reuse its already accepted review with a new command ID. New factual acceptance now requires a fresh pending review; committed command retry still runs first.
- Initial portable validator validation treated a synthetic provenance string as sufficient authority. Production now has no available trusted validator adapter; that label is accepted only by compiled internal test fixtures. A production-library integration regression rejects the forged import.
- A retained-reward command could initially import an impossible historical zero balance. A failing malformed-import test exposed this. Issuance sequence and committed ledger-prefix checks now validate exact historical balances, rather than only bounding them by the current wallet.
- Independent-work validation initially examined only selected evidence for assistance. It now considers all current applicable evidence, so omission cannot hide registered incompatible assistance.
- Restore ordering was corrected to reconstruct corrected historical acceptances before a current acceptance sharing the consumed reward slot. Cross-work/progress duplicate identities are now rejected before restore.

The extended native harness seeds a fresh populated version-2 SQLite profile using Node's built-in SQLite module, verifies migration/backup, preserves that synthetic profile, then runs normal work, evidence/acceptance/correction, complete restore and legacy-rejection flows through the installed app and actual native dialogs. No normal profile or external project is touched.

Further review and executed intermediate results:

- Core real-SQLite tests initially passed 19/20: the lock-contention assertion expected `storage_error` while the implementation correctly returned its specific bounded `busy` error. Correcting that test expectation produced 20/20. A further direct-SQL regression checked locked definitions, allocations, immutable ledger rows and duplicate entitlements independently of service validation.
- Strict Clippy initially rejected a complex test-case tuple type; a named test-only type alias resolved it. Strict Clippy subsequently passed.
- A full debug test build concurrent with native release compilation hit MSVC `LNK1318` PDB errors before tests executed. After release compilation finished, `cargo test --manifest-path src-tauri/Cargo.toml --locked --offline -j 1` passed all 72 tests (31 library, 11 portability, 3 targeted mutations, 27 workspace). No toolchain change, cache deletion, or application workaround was needed. The precise linker cause was not established.
- The first installed Stage 3b package (`native-22eddc9468f049df8a1f410e3512a57d`) passed 31 lifecycle checks, 25 real Windows dialog operations, and all seven WebView2 offline probes. This was an intermediate package before the subsequent correction-freshness fix; its installer SHA-256 was `F84B73239D79A355C773D1A5FD66ED3398AB5D445AFEB428BC1E9099DF6835E1`.
- Final independent review found that an unused pending snapshot saved before a correction could be reused afterward. Corrections now advance the monotonic authority revision, and imported acceptance chains must prove each reacceptance was reviewed at or after its preceding correction. Regression coverage includes this second-snapshot case and malformed portable history. The installed harness also exercises the rejection directly through the production command boundary.
- Exact command replay can return an original acceptance result whose current factual status was later corrected. The UI now reads the returned authoritative history and reports retracted/reopened status instead of announcing a new acceptance.
- Review also identified an incomplete import that removed all acceptance/reward/command records but left an `accepted` snapshot. Reverse correspondence validation now rejects that orphan, preserving local state; an accepted snapshot cannot silently lose its consumed entitlement history.

## Checkpoints B-F: executed integrity and interface gates

| Checkpoint | Implementation and executed evidence |
| --- | --- |
| B: identities/criteria/migration | Database v3 preserves v1/v2 work IDs, revisions, timestamps, relationships and selection; canonical criteria become deterministic drafts without rewards. Populated WAL fixtures, readable original-version backups, migration failure rollback and backup recovery pass. Version changes preserve lineage, slot and lifetime budget; all two-criterion positive integer splits (1+3, 2+2 and 3+1) are exercised. |
| C: evidence/review | Complete registered applicable evidence is computed in Rust. Unknown, duplicate, superseded, cross-domain/version and omitted conflicting references are tested. Missing validators stay pending; fabricated production validator provenance fails. Internal test-only registered sources exercise positive validation and omitted current contradictory validators. Assistance cannot be hidden by omitting evidence. |
| D: acceptance/issuance | Real two-connection races cover identical IDs, changed payload collisions and distinct concurrent commands. Committed retries survive restart and stale inputs. SQL-trigger failures at grant, ledger and command insertion roll back acceptance, grants, ledger, audit and replay result together. Bounded five-second lock contention returns a controlled error. Direct SQL tests check independent constraints. |
| E: corrections/portability | Both incorrect acceptance and historical reopening preserve credit and consumed slots. Fresh review reaccepts without reminting; every pre-correction review is stale. Complete corrected/versioned histories restore exactly, retain original command results, and replay without new issuance. Malformed histories, impossible balances, borrowed revisions, orphan accepted snapshots, forged digests, late restore failure, backup failure and unsafe legacy replacement preserve valid local records. |
| F: minimal interface | Saved next action remains primary; optional progress shows criterion locking/versioning, concise evidence, complete review, pending/reject/accept decisions, factual history and correction. Rewards start hidden. UI/client tests cover draft preservation on failure, keyboard/focus, navigation guards, allocations, pending/conflict display, exact command retry and corrected-result feedback. |

Final application-source checks after the review fixes:

| Executed command | Actual result |
| --- | --- |
| `npm.cmd test` | 44 passed, 0 failed: 26 App, 11 progress panel, 7 typed client |
| `npm.cmd run typecheck` | Passed |
| `npm.cmd run build` | Passed; 20 modules, final JS 260.66 kB, CSS 5.37 kB |
| `cargo fmt --manifest-path src-tauri/Cargo.toml -- --check` | Passed |
| `cargo test --manifest-path src-tauri/Cargo.toml --locked --offline -j 1` | 75 passed, 0 failed, 0 ignored: 34 library (24 progress), 11 portability, 3 targeted mutation, 27 workspace; 0 doc tests |
| `cargo clippy --manifest-path src-tauri/Cargo.toml --locked --offline -j 1 --all-targets -- -D warnings` | Passed |
| `node --check scripts/native-regression.mjs` and `node --check scripts/native-progress.mjs` | Passed |

File groups: `src-tauri/src/progress.rs` and `progress/tests.rs` contain typed policy and SQLite service regressions; migration `003_progress.sql` adds the relational constraints; storage/domain/service/commands/lib integrate the authority boundary; `tests/portability.rs` and adapted workspace tests verify migration/restore; `src/progress`, App, typed client and their tests implement the UI. Contracts, README, ADR-0003/0004, scope notes and this log document the public model. The native harness is extended in `scripts/native-regression.mjs` and `scripts/native-progress.mjs`.

## Checkpoint G: final installed Windows verification

Application source corresponds to commit `9daea4e38f7c822d3abf98f406e0a8eabcfcdf1c` (same runtime source as `769abbf`; the following commit adds the synthetic UI fixture helper). Subsequent verification documentation/harness changes do not change application source. Built, installed, exercised and uninstalled a fresh isolated NSIS package using:

```powershell
& ([scriptblock]::Create((Get-Content -LiteralPath scripts/verify-native.ps1 -Raw))) -Repository (Get-Location).Path
```

The script forwards `--locked --offline` to Cargo. This invocation avoids changing the machine's PowerShell execution policy. Node's built-in SQLite module emits its experimental-module warning; the harness uses it only to construct and inspect synthetic migration fixtures, while the app uses rusqlite.

| Final native evidence | Executed result |
| --- | --- |
| Run | `.artifacts/native-a2b228e92e7142e586da989bfdd31db4` |
| Installer | `Ascension Verification a2b228e9_0.1.0_x64-setup.exe`, 2,486,897 bytes |
| Installer SHA-256 | `1E151C2EA6A96137B8CE92259AF6CD0EBE9F825E8418269EE41FBD3081F8588C` |
| Installed executable SHA-256 | `FB9A1065CAE4C0E36C7D983ADD798071D073BB1A804CA01B9FFF3DABC39F46E4` |
| Install / test / uninstall | All exited successfully; isolated installed directory removed |
| Lifecycle assertions | **32 passed, 0 failed** |
| Actual Windows file-dialog operations | **25 successful** |
| Controlled offline probes | **7/7 supported and passed**, with native workspace read, unchanged state and offline reload on each launch |
| Visual inspection | Final saved-action and progress screenshots inspected; next action/note remain primary, optional reward display and locked criterion are legible |

The installed run verifies populated Stage 3a v2 migration and its readable original-version SQLite backup; fresh-work keyboard editing and save acknowledgments; native dialog cancellation and malformed/unsupported/oversized import rejection; exact replacement, recovery backup, replay, graceful/abrupt restart; prospective lock and evidence; complete review and exactly four BC issuance; duplicate command protection; persisted accepted history; factual correction retaining credit; rejection of a separate pending review created before correction; fresh reacceptance without reminting; rejection of unsafe legacy replacement; complete modern export, restore into empty state, exact re-export, replay and final offline restart with the saved next action immediately visible.

The harness checks the actual Tauri application-data directory and unique verification identifier before destructive operations. Migration and work profiles, JSON exports, backups, screenshots, `native-result.json` and `package-result.json` remain only under the ignored run directory. No tests touched the normal installed profile or another repository. No OS network/security settings were changed.

The standard configuration also built successfully with `npm.cmd run tauri -- build -- --locked --offline`. Its ignored output is `src-tauri/target/release/bundle/nsis/Ascension_0.1.0_x64-setup.exe` (2,485,318 bytes), SHA-256 `D8E0D5715996082A72A37C285FB8A9D8C3F21E29FFEBEEE7BCE1026124290379`. The corresponding packaged application executable SHA-256 is `C1F4871F01CA0A288A95003835FF30CC735EA23CAF6228ECC8EEED01255E8C43`. Installed lifecycle verification used the isolated configuration above; the standard configuration was built but not installed over the owner's normal profile.

Local checkpoints on `feat/stage-3b-evidence-ledger`: `8375294` targeted storage and authorized scope; `24a011c` authoritative backend/migration/portability and SQLite tests; `769abbf` optional interface/client/tests; `9daea4e` synthetic UI fixture helper. The final verification commit contains this log and the native harness extensions. Staged diffs were checked before each commit for public safety, secrets, personal paths, generated binaries, databases and backups. No push, merge, release or deployment occurred. The branch is ready for independent review and a proposed pull request; publishing it requires separate authorization.

## Scope of the evidence and remaining limitations

- This is a local honest-use evidence registry, not proof of external truth or an anti-tampering system against the database owner. No production trusted-validator adapter, external execution, scanning, AI, account, cloud or game system was introduced. Required unavailable validation remains unsupported/pending.
- Controlled offline denial is scoped to the installed WebView2 target, with a successful owned-loopback positive control, a failing offline request that does not reach that server, and native reload while offline. The harness also uses a refused process proxy. Rust-process and OS-wide networking are not blocked; no firewall/security settings are changed.
- Abrupt process termination, committed WAL recovery, transaction error injection and migration/restore rollback were executed. Physical power failure, disk-controller failure and termination at every machine instruction were not tested.
- The Windows installer is unsigned and requires an existing WebView2 runtime. Only the recorded x64 Windows environment was verified; no broad hardware/OS compatibility or production-grade security claim is made.
- App keyboard/focus behavior was exercised by UI and installed tests. Native Windows file dialogs were driven through bounded Win32/UIA controls; manual keyboard operation and screen-reader behavior of those dialogs were not verified. No full assistive-technology audit is claimed.
- Modern restore is explicit complete replacement, including an intentional rollback to older history if confirmed; it is not merge or cloud reconciliation. Backups persist until manually managed. Complete state is limited to 10,000 records and 10 MiB. No silent truncation occurs.
- The global authority revision conservatively invalidates unrelated pending reviews after a registry/correction change. The UI shows the current review and factual acceptance history; full frozen historical evidence remains inspectable in complete exports. The supported UI locks criteria before evidence entry. If a caller records evidence against a draft through the typed command boundary, later incompatible draft scope/artifact edits are rejected to preserve that recorded context.
- Synthetic UI fixtures intentionally model selected view states rather than full valid portable authority graphs; Rust integration and installed tests verify complete authoritative states. A fixture helper omitted from the first UI staging list was committed immediately afterward; no Git history was rewritten.

No Stage 3c/4 feature was implemented. Independent code review is the next step. Any subsequent milestone, including opportunity tracking or construction presentation, needs separate owner authorization; spending, AI and external integrations remain outside this change.
