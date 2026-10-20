# Stage 4 verification record

Status: **PASS — Stage 4 engineering implementation and verification complete, 2026-10-20.** The final source passed automated, performance, installed-Windows, portability, and visual gates. No unresolved consequential economic, data-integrity, or installed-app defect was found. Engineering verification does not establish productivity benefit, human feasibility, commercial readiness, or authorization for Stage 5.

## Baseline — 2026-10-20

- Clean local `main` and locally recorded `origin/main` at merged Stage 3c `7b522be`; no remote fetch required to establish the requested base. Created `feat/stage-4-river-reach`. No push or merge.
- Read the owner-provided Stage 4 assignment, repository rules, Stage 3a/3b/3c specifications, architecture decisions/boundaries, Stage 3c verification history, persistence/economy/study/H1/frontend contracts, migrations, tests, and the existing native harness.
- Existing Node 22.23.3/npm 10.9.9, Rust 1.99.1 stable MSVC, Visual Studio Build Tools 2022 17.14.37806.7 with C++ tools, and WebView2 155.0.4283.62. `npm.cmd` avoids the restricted PowerShell wrapper; existing Rust toolchain binaries require adding their directory to the process PATH. No installations or policy changes.
- Fresh `npm.cmd test`: **94 passed**, six files, 45.03 seconds.
- Fresh frontend typecheck/build, Rust formatting, and strict all-target Clippy: passed. The baseline build remains 31 modules and 342.61 kB JavaScript.
- Fresh `cargo test --manifest-path src-tauri/Cargo.toml --locked --offline -j 1`: **110 passed** (68 library, 11 portability, one bounded-history test, three targeted-mutation, 27 workspace); two explicit benchmarks ignored. Compile 29.90 seconds; tests 20.49 seconds. The first default-parallel compile exhausted available Windows commit/page-file memory (OS 1455); the bounded retry passed without changing any system setting.
- Ignored raw baseline logs: `.artifacts/stage4-baseline/`. They are local engineering artifacts, not committed public evidence.

## Design checkpoints

The public rules and authority contract are in [STAGE_4.md](../specs/STAGE_4.md) and [ADR-0006](../decisions/ADR-0006-river-reach-economy.md). Funded capacity is reusable province capacity; placement still uses exact stable geographic site IDs. Investment never refunds through reconfiguration, cancellation, or undo. World reads issuance but cannot mutate it. H1 continues to use cumulative issuance. Active E00/E01 withhold strategic gameplay in UI and Rust.

Historical Stage 3a/3b/3c reports remain unchanged. The current implementation uses the same pinned stack and lockfiles; no dependency, game engine, external service, AI, telemetry, account, notification, or autonomous runtime behavior was added.

## Recovery after the external interruption

The owner's continuation resumed the existing `feat/stage-4-river-reach` branch with `8a7bc93` and `bee10d0` intact. The staged frontend, unstaged native harness and report, and ignored synthetic artifacts were preserved. No reset, clean, automatic Undo, history rewrite, or switch to `main` was performed. Process inspection found no surviving old verification app, runner, or compiler requiring cleanup; the intermediate verification package had been uninstalled.

The three UI findings had already been implemented before recovery: explicit queue funding details, persistent sandbox labeling and suppressed hidden scene rendering, plus preservation of a world draft through normal Work edits. Fresh frontend gates verified them. The continuation's additional requirement for available BC and chapter spending directly in the blueprint added two backend-derived rows and assertions. Remaining work at recovery was final-source native verification, serial release measurements, final regression/build gates, visual inspection, complete reporting and local commits. All were completed on the existing implementation. Visual inspection additionally found and fixed covered parcel labels. The interruption itself was not an application failure.

## Implemented economy, spatial system, and portability

`world.rs` owns the frozen six-building/three-route content and one six-site grid. Completed layout capabilities are derived; partial reservations contribute none. The chapter is explicitly initialized with zero investment. The new spending authority reads Stage 3b issuance and factual corrections, without changing `progress.rs`, grant identity, reward slots, or acceptance commands. `wallet_bc` continues to mean cumulative issued BC.

Database migration 5 adds three STRICT tables: a singleton `world_chapter`, append-only `world_spending`, and append-only `world_commands`. Generated relational fields, uniqueness, chapter foreign keys, amount/capacity bounds, aggregate spend checks, reservation guards, and immutable-ending/update guards supplement Rust validation. Existing migrations are unchanged. One private repository coordinator wraps commands in `BEGIN IMMEDIATE`, complete record validation, export-size validation, and commit.

Canonical world commands bind schema 1, ruleset `river-reach-v1`, chapter/revision, operation, and UUID. Exact replay returns its original result; changed payloads conflict. New commands atomically queue/replace/reserve/cancel/complete, reconfigure, undo, select a route, or commit an ending. Spending and reservation totals are separate from issuance. Deterministic FIFO allocations identify the issuance entries that funded province capacity. Correction-retained attribution is displayed at province level because configurations may freely change.

Six stable geographic sites remain explicit while funded capacity is fungible. All specialist footprints are orthogonally adjacent, overlapping sites and excess copies are rejected, and free configurations fit owned capacity. One durable previous layout supports undo; an active queue must first be canceled. Completing a route requires explicit confirmation against the saved qualifying layout. The first ending retains exact layout, ruleset, capabilities, investment, narrative, timestamp, and command identity; later sandbox layouts do not modify it or spend again.

Interchange v4 retains the complete previous Work/reward/study graph and adds chapter, queue, spend attribution, canonical command history/results, undo, and Chronicle. Validation reconstructs chapter transitions against their historical issuance prefixes and compares all stored outcomes. Modern restore remains explicit replacement with a complete synced backup. Older v1/v2/v3 files cannot erase any local world history; existing earlier legacy protections remain. World commands rotate workspace generation so previously previewed imports become stale.

## Files and delivered behavior

| Files | Responsibility |
| --- | --- |
| `AGENTS.md`, `docs/specs/STAGE_4.md`, `docs/specs/ARCHITECTURE_BOUNDARIES.md`, `docs/decisions/ADR-0006-river-reach-economy.md` | Owner-authorized scope, frozen rules, authority and experimental boundaries |
| `src-tauri/src/world.rs`, `src-tauri/migrations/005_world.sql` | Authoritative world model, transitions, content, spatial legality, accounting, history validation, private database extension |
| `src-tauri/src/domain/mod.rs`, `storage/mod.rs`, `service.rs`, `commands/mod.rs`, `lib.rs` | Required complete records, migration/backup/restore, coherent reads, three typed commands and Tauri registration |
| `src-tauri/src/world/tests.rs`, `src-tauri/tests/stage4_integration.rs` | Real-SQLite adversarial tests, enumeration, migration and explicit release benchmarks |
| `src-tauri/src/study/tests.rs`, `src-tauri/tests/portability.rs`, `workspace.rs` | Existing assertions updated for current schema while retaining legacy fixtures and protections |
| `src/world/{RiverReach.tsx,RiverReach.css,ProvinceMap.tsx,BuildingArt.tsx,types.ts}` | Optional game interface, original SVG province/buildings/route endings, strict transport types |
| `src/App.tsx`, `src/App.css`, `src/platform/client.ts` | Work-first navigation, lazy World loading, draft preservation, study isolation and typed IPC |
| `src/world/__tests__/*`, `src/__tests__/App.test.tsx`, `src/platform/__tests__/client.test.ts`, `src/progress/__tests__/fixtures.ts` | UI, navigation, keyboard, confirmation, failed-request and transport regressions |
| `scripts/native-world.mjs`, `native-regression.mjs`, `native-progress.mjs`, `native-study.mjs`, `verify-native.ps1` | Disposable installed-Windows lifecycle and inherited regressions plus River Reach verification |
| `README.md`, `contracts/README.md`, this report | Play/use instructions, exact authority/portability contracts and executed evidence |

The fixed map uses `a1 a2 a3 / b1 b2 b3`. Foundry, Grand Archive, and Embassy occupy two adjacent sites and provide five Engineering, Knowledge, or Diplomacy respectively. Survey Lodge gives E2/K1; Observatory K2/D1; Caravan House E1/D2, each occupying one site. Specialists represent eight BC of capacity and mixed buildings four; copies are capped at two per type. Stone Causeway requires E10/K4, River Compact D10/K4, and Beacon Mesh E6/K6/D6. All routes and deficits are visible, and no ending awards credit.

The chapter has no passive production, upkeep, construction timer, decay, or absence penalty. Blueprints may wait indefinitely. Gameplay never creates a new reward entitlement or changes real accomplishment authority.

Players can select sites with the illustrated map or ordinary buttons, inspect backend previews, reserve/replace/cancel a blueprint, and explicitly confirm irreversible investment. Free full-layout previews support relocation, specialist orientation, removal, type replacement, and splitting/combining footprints within funded capacity. A failed command retains the draft; lost-response retry preserves the exact command identity. Reconfiguration and one-step durable undo do not change issuance, investment, or capacity. A queue must first be canceled. Route confirmation uses the saved layout and preserves one exact original ending. Labeled sandbox alternatives and the Chronicle remain separate.

H1 still restores `min(64, cumulative issued BC)` units and never reads the spend balance. H2 has separate content and lazy rendering and is withheld by UI, world reads, previews, and new mutations during both active study phases. Exact replay can return a prior immutable receipt without performing a new transition. Explicit existing study pause/leave remains accessible. Synthetic tests exercise study activation in isolated fixtures; no real E00/E01 study was activated. The inherited unbounded post-window H1 checkpoint still requires a separately versioned policy correction before real E01 activation; this chapter does not silently change frozen protocols.

## Executed automated verification

Final backend commands on the implementation source:

```text
cargo test --manifest-path src-tauri/Cargo.toml --locked --offline -j 1
cargo fmt --manifest-path src-tauri/Cargo.toml -- --check
cargo clippy --manifest-path src-tauri/Cargo.toml --locked --offline -j 1 --all-targets -- -D warnings
```

**147 Rust tests passed, zero failed; four explicit benchmarks ignored in the ordinary suite.** Breakdown: 102 library tests (including 34 world tests), 11 portability, one Stage 3c bounded-history, three Stage 4 migration, three targeted-mutation, and 27 workspace tests. Final run after the last parcel-label change: Cargo build 0.66 seconds; groups 9.39/0.70/2.46/0.57/0.20/9.04 seconds. Final formatting and strict all-target Clippy passed (Clippy 0.81 seconds), including the added benchmark targets. Raw log: `.artifacts/stage4-rust-closure.log`. No authority test was removed or weakened for performance.

| Executed category | Evidence |
| --- | --- |
| P5T034 currency conservation | Zero/four/eight/multiple authentic synthetic grants; partial holds; insufficient funds; atomic replacement; repeated cancellation; deficit-only spending; 24-BC cap; no overcommit |
| Concurrency and replay | Concurrent exact completion spends once; distinct competing completion and reservation decisions serialize/reject; canonical same-ID payload conflict; retries across restart and full restore |
| Transaction interruption | Injected spend, chapter, command and generation failures; dropped uncommitted transaction; bounded database-busy behavior; original records and accounting remain intact |
| P5T035 capacity conservation | Zero through six capacity, free specialist/mixed conversion, move/rotation/removal/replacement/undo, no seventh site, no issuance/refund, retained construction after factual correction |
| Spatial and route correctness | All 15 distinct pairs examined, seven orthogonal pairs accepted; overlap/diagonal/copy limits/unknown sites rejected; illegal previews cannot qualify; all three routes completed through legal layouts |
| Chronicle | Explicit first completion, exact duplicate receipt, changed/new completion rejection, immutable original ending after sandbox, restart and restore |
| Portability/adversarial input | 19 malformed-history variants plus recomputed-digest impossible history; strict unknown fields/enums/IDs; v1/v2/v3 local-history rejection; complete queue/reservation/undo/ending replay after restore |
| Restore failure | Backup failure and injected replacement-command/generation failure preserve the original database; complete recovery backup remains usable |
| Migration | Populated schema v4 with 24 genuinely earned synthetic BC, saved Work, configured frozen E01 and opportunities; exact exported graph preserved; coherent SQLite backup, failed backup, transactional DDL rollback and safe retry |
| SQL constraints | Independent direct-SQL tests of budget, capacity and immutable ending/update guards; reservation overcommit exercised through real commands/concurrency; deferred spending-command references; foreign-key and quick-check validation |
| Prior authority/studies | Entire inherited suite retained; grant identity, four-BC parent entitlement, acceptance retry/correction, frozen assignment, opportunity missingness and H1 behavior preserved |

Exhaustive numerical enumeration executed all **729 combinations**, with **163 within six sites**, and qualifying counts **3 Stone Causeway / 3 River Compact / 1 Beacon Mesh**. Every qualifying combination occupies six sites. Spatial validation is tested separately; numeric capability alone cannot authorize completion.

Frontend commands are `npm.cmd test`, `npm.cmd run typecheck`, and `npm.cmd run build`. The final run after the parcel-label refinement passed **118 tests in seven files**, zero failed, 17.56 seconds. Typecheck and production build passed. Tests cover Work-first navigation, preserved Work and world drafts across ordinary editing/saving, hidden-scene suppression, saved-blueprint accounting, persistent sandbox labeling, keyboard parcel choice, invalid placement, authority-only payloads, explicit spend/ending confirmations and modal focus, route previews, immutable Chronicle, exact failed-request retry, active E00/E01 withholding and stale-read races. H2 remains a separate lazy chunk, `RiverReach-jLrZiCHG.js`, 35.18 kB JavaScript (10.61 kB gzip) and 8.22 kB CSS (2.43 kB gzip); main JavaScript is 346.68 kB (102.85 kB gzip), with 36 modules in the production build.

## Native Windows and visual verification

**Final installed run passed: 58 lifecycle checks, 60 real Windows Save/Open dialog interactions, and 18/18 supported offline probes/restarts.** The package compiled in 3 minutes 36 seconds, installed into a checked disposable repository-local directory and used a unique verification identifier/profile. The harness exited zero and uninstalled that package. The normal user profile was not opened or modified.

The full final run is `.artifacts/native-64da835f443c465ea046843c90fb7e70/`. Evidence: `native-result.json`, `package-result.json`, `world-timings.json`, and `source-result.json` (all 54 tested source/dependency/harness hashes matched the final files). Console log: `.artifacts/stage4-final-native-parcel-labels.log`. Invoke the harness without changing PowerShell execution policy using:

```powershell
& ([scriptblock]::Create((Get-Content -Raw -LiteralPath scripts/verify-native.ps1))) -Repository (Get-Location).Path
```

Its build command is `npm.cmd run tauri -- build --config <isolated-run>/tauri.conf.json -- --locked --offline`, with `CARGO_BUILD_JOBS=1`. Installation/uninstallation and bounded UIA/Win32 helpers require the authorized native execution context; no machine-wide installation or security-policy change is made.

The installed application executed inherited Work creation/editing/save, real file dialogs, visible validation and focus, graceful/abrupt restart, coherent backups and failed-restore preservation; authentic synthetic acceptance/correction and no-remint retries; prospective opportunities, frozen E01 assignments, missingness and H1 saturation; and migration from populated schema v4 with 68 BC/four assignments to schema v5 without modifying the record graph. Active E00/E01 rejected H2 reads and visibly withheld gameplay while retaining an explicit return to study settings.

River Reach execution used both keyboard UI and actual native Rust commands. It verified all six sites/buildings, three visible routes, illegal diagonal rejection, an authentic first four-BC building, exact/altered retry behavior, partial eight-BC specialist funding, cancel/release and canceled-command replay, partial reservation export/restore/restart, six funded sites/24 invested BC, rejected unmet endings, free layout/undo, factual corrections, H1 cumulative issuance after spending, old-v3 import rejection, and complete modern replay history portability. The saved blueprint's rendered 0 available/4 spent/8 total capacity/4 missing values were compared to its live backend state.

Stone Causeway was completed through the UI after full-layout preview and undo; its original ending survived a balanced sandbox. River Compact and Beacon Mesh were independently constructed and committed through native commands after explicit coherent synthetic restores, then rendered in the Chronicle and reopened. The combined Mesh record set preserved 17 authentic grants, 68 issued/24 invested/0 reserved/44 available BC, four frozen assignments and every study observation/correction/exposure across export, empty replacement, re-export and restart. No prior ending was edited to simulate another route.

Final installed screenshots were visually inspected. The initial board and partially developed layout read as a compact illustrated strategy board; all six building silhouettes are distinct, parcel identities and capability/route values remain readable, and saved sandbox text explicitly distinguishes the original ending. The larger queue panel and ending confirmation are aligned and unclipped. A real resized native window produced a 647 × 815 CSS-pixel client area with no horizontal overflow; the map and planning controls stack vertically. The artwork is schematic SVG, and longer catalogs/Chronicle pages require scrolling.

Local screenshot references (all beneath the final run directory, ignored rather than published):

| Capture | Local filename |
| --- | --- |
| Initial playable province, before investment | `river-reach-six-sites.png` |
| First developed parcel | `river-reach-first-building.png` |
| Half-funded specialist and full accounting | `river-reach-partial-specialist.png` |
| Building silhouettes and three route requirements | `river-reach-building-catalog.png`, `river-reach-route-requirements.png` |
| Explicit ending and preserved Causeway Chronicle | `river-reach-ending-review.png`, `river-reach-chronicle.png` |
| Saved alternative and actual narrow window | `river-reach-sandbox.png`, `river-reach-narrow-native-window.png` |
| Independent Compact and Mesh endings | `river-reach-river-compact-chronicle.png`, `river-reach-beacon-mesh-chronicle.png` |
| Full route maps with all six parcel labels | `river-reach-river-compact-fullmap-native.png`, `river-reach-beacon-mesh-fullmap-native.png` |
| H1 after spending, still derived from issuance | `river-reach-h1-retained-issuance.png` |

Isolated installer: 2,815,450 bytes; SHA-256 `17EBCCEF5D64E23CE2B436004D61B07036C20E46C57E101B05644C50589A6025`. Installed executable SHA-256: `34027F3D1DF29FDE9F19591F4250ED6C09848619400897A09B27158D64961E54`.

The final normal-identifier installer also passed after the installed timing run: `npm.cmd run tauri -- build -- --locked --offline`, using one Cargo job, release compile 3 minutes 34 seconds. Output: `src-tauri/target/release/bundle/nsis/Ascension_0.1.0_x64-setup.exe`, 2,815,766 bytes, SHA-256 `5DD33BFD0EE0F8FA441A2689A41A2B07A43E9C98CD6501C6B8234BCEBFCA2402`. Log and metadata: `.artifacts/stage4-standard-installer-final.log` and `.artifacts/stage4-standard-package-final-result.json`. This standard package was built locally, not installed over the real profile or published.

The first intermediate harness attempt exposed a synchronization issue: direct synthetic E01 activation did not refresh the renderer's previous E00 label; H2 remained withheld. Refreshing before asserting the new phase corrected the harness. A PowerShell 5 capture issue also treated harmless native stderr as a terminating error; the wrapper now checks actual native exit codes. The intermediate rerun passed 54 checks, but final closure uses the 58-check run above.

An earlier 58-check candidate passed before the parcel-label refinement. A supplemental screenshot helper then failed at a CRLF extraction boundary and overwrote that run's `native-result.json` with the capture failure. Its logs and evidence were preserved in `prior-final-run-before-capture-rerun/`; no success file was reconstructed. Full-map capture is now part of the ordinary route verification harness. A fresh unique installation and complete run against the corrected artwork supplies the definitive closure evidence.

## Performance

Executed serial release command:

```text
cargo test --manifest-path src-tauri/Cargo.toml --locked --offline --release --features tauri/custom-protocol -j 1 --test stage4_integration -- --ignored --nocapture --test-threads=1
```

**Two Stage 4 benchmarks passed**, zero failed; compilation 8 minutes 14 seconds, measurements/fixture execution 23.17 seconds. Final raw log: `.artifacts/stage4-release-benchmark-final.log`. The earlier nonserial run also passed, but the numbers below come from the serial rerun. The ordinary suite's four ignored cases consist of these two explicit Stage 4 benchmarks and two inherited explicit benchmarks; they are not skipped correctness checks.

The first fixture contains the listed number of synthetic objectives with actions, six authentic acceptance-earned grants totaling 24 BC, a configured frozen study with ten opportunities, and a chapter that grows to 14 commands. Complete exports contain 104,901 / 296,691 / 1,149,491 / 2,215,491 bytes respectively. All values below are **milliseconds**; ranges represent the three individual building operations, not confidence intervals.

| Service operation | 10 objectives | 100 | 500 | 1,000 |
| --- | ---: | ---: | ---: | ---: |
| Work load | 0.895 | 2.560 | 9.450 | 25.499 |
| World view read | 0.417 | 1.381 | 3.409 | 7.379 |
| Initialize | 3.676 | 8.575 | 38.211 | 76.269 |
| Queue | 3.387–3.934 | 7.032–8.104 | 28.957–30.918 | 74.207–77.674 |
| Reserve | 3.486–5.201 | 6.818–8.917 | 29.543–30.389 | 74.894–76.143 |
| Complete construction | 3.501–4.797 | 7.196–9.281 | 29.548–30.837 | 76.776–77.486 |
| Placement preview | 0.359 | 0.248 | 0.214 | 0.255 |
| Reconfigure | 5.922 | 7.724 | 31.517 | 78.005 |
| Undo | 4.117 | 7.462 | 31.916 | 74.468 |
| Complete chapter | 4.268 | 8.084 | 29.623 | 74.397 |
| Export | 1.756 | 2.949 | 11.031 | 32.015 |
| Import validation/preview | 2.470 | 5.063 | 27.518 | 64.552 |
| Restore into empty repository | 12.328 | 24.913 | 88.295 | 197.960 |
| Repository reopen | 7.500 | 7.396 | 19.480 | 38.216 |

The second fixture keeps ten objectives and ten opportunities while retaining the listed number of world decisions (initialization followed by route selections). It tests growth of durable history, not only a fresh chapter. Exports range from 95,251 to 1,196,362 bytes.

| Service operation | 10 commands | 100 | 500 | 1,000 |
| --- | ---: | ---: | ---: | ---: |
| Work load with retained history | 2.084 | 2.696 | 5.601 | 9.850 |
| World view read | 0.791 | 1.395 | 2.170 | 3.073 |
| Export | 2.228 | 2.887 | 5.999 | 11.720 |
| Import validation/preview | 4.466 | 5.309 | 14.985 | 23.820 |

Measurements retain all authority, record, history and exportability checks. Command helpers include their initial workspace load as well as the mutation, so these are service-operation timings, not isolated SQL transaction timings. They are individual warm local observations, not medians/P95 or universal latency guarantees. A repository reopen does not clear the operating-system cache. A thousand route selections do not represent a thousand large reconfiguration snapshots or the maximum allowed record graph. The 60-second assertion in the benchmark is a severe-regression guard, not a UX target.

Installed measurements below use the final disposable release application with six authentic grants/24 BC for Causeway, and 17 grants/68 BC plus four historical assignments for Compact/Mesh. No competing Rust compilation ran during these samples. UI measurements include CDP round trips, polling and acknowledgement; file operations include Windows dialog automation where noted. Process startup is a new process, not a reboot or cleared OS cache.

| Installed operation | Observed milliseconds |
| --- | ---: |
| Process launch to Work ready, 18 launches | 902.53–1,161.71; observed median 1,022.45 |
| First World open / first rendered province | 349.02 / 351.31 |
| World open, five observations | 338.20–401.35 |
| Building selection, 14 observations | 9.64–15.30 |
| Invalid placement preview via IPC | 6.29 |
| Queue with reservation through UI | 133.86 |
| Reserve via IPC | 15.19 |
| Construction completion through UI | 139.55 |
| Construction completion via IPC | 14.13–35.27 |
| Full reconfiguration through UI | 140.93–142.48 |
| Undo through UI | 144.27 |
| Chapter completion through UI | 137.57 |
| Sandbox configuration through UI | 157.85 |
| Complete world export, including Save dialog | 1,391.27 |
| Combined world/study export, including Save dialog | 1,362.52 |
| Import preview, including Open dialog | 1,334.51–1,613.14 |
| Confirmed restore after preview | 119.11–138.43 |

The complete individual samples and profile descriptions remain in `world-timings.json`; startup samples are in `native-result.json`. No unmeasured interaction, physical-machine cold startup, or percentile target is inferred from these bounded observations.

## Accessibility and limitations

Semantic selects and buttons support building and parcel selection without dragging or hover. Text names, site IDs, funding values, capabilities, route deficits, saved/draft state, errors and confirmations accompany the artwork. SVG decoration does not replace readable state. Focus rings, modal Escape/Tab handling, return focus, reduced-motion rules and responsive layouts are implemented. Automated keyboard/focus flows passed; installed keyboard parcel selection, layout save, undo and confirmation flows, dialog Tab/Shift+Tab containment, and normal/narrow screenshot inspection passed. No complete screen-reader or assistive-technology audit is claimed.

## Defects identified during implementation

- A placement needing new funding initially shared the same validity flag as an illegal geometry. Separate `placement_legal` from free-layout `legal`, and do not report invalid geometry as satisfying a route.
- SQL null comparison could otherwise allow removal of a completed ending. Use `IS NOT` for immutable completion comparison and exercise direct SQLite corruption attempts.
- Queue replacement now releases/reassigns its reservation within one transaction; an invalid replacement preserves the original blueprint and hold.
- Historical downgrade fixtures must remove the new `world` field, and schema-v3 database fixtures must remove the new tables before replaying additive migrations. These test changes preserve their original compatibility assertions.
- UI refresh ordering now prevents an older successful world read from restoring strategic content after a study-active rejection. Study rejection clears the map and refreshes Work state; focus/visibility refresh also rechecks access.
- Navigation is disabled during irreversible confirmation dialogs and modal focus stays inside their controls.
- Review identified missing saved-blueprint funding details, ambiguous saved sandbox labeling, and hidden scene rendering during Work edits. The saved queue now gives the explicit footprint/value, reusable funded capacity, deficit, reserve, remaining amount, legal status and affordability. Saved alternatives remain labeled sandbox. Hidden H2 skips scene rendering while retaining its draft. A separate world draft epoch prevents ordinary Work editing/saving/selection from discarding that layout; destructive reload/restore still guards it. Four additional regressions passed in the final 118-test UI suite.
- Full-map visual inspection found that vertical specialists could cover top-row parcel labels. All six identities now use consistent foreground badges above the artwork; semantic map buttons retain their accessible labels.

## Known scope and evidence limits

The installer remains unsigned and assumes preinstalled WebView2. Renderer offline probes and a launch-time refused proxy do not demonstrate physical network isolation of Windows or Rust. No physical power-loss test, hostile-OS defense, complete assistive-technology audit, human feasibility result, or productivity benefit is claimed. Local owner edits to SQLite are outside the typed-command boundary.

History remains bounded to 10,000 total records and 10 MiB. Complete restore is explicit replacement, not merge. Recovery backups are retained without automatic pruning. Undo retains one prior layout. One chapter and six sites are intentional scope limits; no Stage 5 work, AI, external evidence runner, CI workflow, push, merge, publication, deployment, or release is authorized or performed.

## Local checkpoints and final review

- Baseline `7b522be` remains on `main`.
- `8a7bc93` — public Stage 4 rules and authority boundary.
- `bee10d0` — verified authoritative economy, migration, complete portability and Rust tests.
- `8b29a25` — playable province, Chronicle, accessible navigation and 118 verified frontend tests.
- `0d0b743` — explicit available/spent blueprint accounting and always-visible parcel badges.
- `3e6180b` — final installed-Windows harness, all route endings and complete portability.

Every staged diff was inspected before its local commit for private paths, credentials and generated artifacts. The documentation closure is the subsequent commit containing this completed report and the README status correction; its hash is provided in the final handoff. No generated databases, exports, screenshots, binaries, logs, or private source package were committed.

## Final outcome

All required Stage 4 gates are complete: 147 Rust correctness tests, 118 frontend/client tests, typecheck, production frontend build, formatting, strict Clippy, two explicit serial release benchmarks, normal and isolated Windows installer builds, 58 installed lifecycle checks, 60 file-dialog interactions, 18 supported offline probes/restarts, and screenshot inspection. Consequential conservation, concurrency, replay, spatial, migration, and restore assertions executed successfully; they are not merely architectural claims.

The branch is `feat/stage-4-river-reach`. Closure leaves a clean tracked working tree with ignored synthetic evidence retained; `main` remains at `7b522be`. No existing checkpoint was rewritten, and no push, merge, publication, deployment, or release was performed.

The branch is ready for independent code review and preparation of a pull request. It is a suitable Stage 4 merge candidate after that review and owner authorization; no known engineering verification gap blocks that review. No pull request was created or merge attempted. The inherited H1 checkpoint-policy issue still blocks real E01 activation until a separately versioned correction, but does not invalidate this bounded Stage 4 implementation.

Stage 5 could technically be considered as a separately specified next stage. It remains outside this authorization, and neither feasibility nor behavioral benefit is established by these engineering results. Development stops at Stage 4.
