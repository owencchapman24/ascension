# Stage 3c verification record

Status: **engineering PASS, 2026-10-20**, with 94 frontend tests, 110 Rust tests, the complete installed-Windows verification, the final release performance run, and the standard Windows installer build passing. No known blocking implementation defect remains. The branch is ready for independent review and owner approval before merge. This is engineering evidence, not a claim of human feasibility or behavioral efficacy.

## Baseline — 2026-10-19

- Clean local `main` at merged Stage 3b `b40a7f0`; created `feat/stage-3c-opportunities-h1` without fetching or changing remote state.
- Read the owner-provided Stage 3c assignment, repository rules, Stage 3a/3b specs, architecture boundaries, decisions, verification history, migrations, Rust authority/command/portability code, React Work/progress code, tests, and installed-Windows harness.
- Existing environment: Node 22.23.3, npm 10.9.9, Rust 1.99.1 stable `x86_64-pc-windows-msvc`, Visual Studio Build Tools 2022 with C++ tools, WebView2 155.0.4283.62. `npm.cmd` avoids the blocked PowerShell wrapper. Rust is available through the existing toolchain's bin directory. No machine-wide installation or policy change.
- `npm.cmd test`: **44 passed**, 3 files, no failures (baseline).
- `cargo test --manifest-path src-tauri/Cargo.toml --locked --offline -j 1`: **75 passed**, no failures or ignored tests (34 library, 11 portability, 3 targeted-mutation, 27 workspace; baseline).
- Added an explicit ignored-by-default benchmark (`stage3c_benchmark.rs`) measuring real SQLite opening, loading, save, review, ledger projection, export, import preview, and restore at 10/100/500/1,000/1,500 objectives with eight valid grants and four retained-credit corrections. The baseline predates opportunities; later measurements separately scale opportunities.

## Executed baseline performance

Hardware: Intel Core i7-1065G7 at nominal 1.30 GHz, 8 logical processors, 15.6 GiB visible physical memory; Windows x64 build 26200. Standard system metadata only, with no device/user identifiers recorded. Initial CIM metadata reads were restricted; Node `os` and processor registry metadata provided these values.

The initial release benchmark dependency rebuild was canceled before producing results to avoid blocking additive implementation. The existing debug profile supplied an attributable Stage 3b baseline instead. This is **not release performance** and does not establish frontend or cold process startup speed. Three repetitions per operation; local SQLite FULL synchronization and full integrity/exportability checks remain enabled. Other desktop activity was not controlled, so these are representative observations, not hardware guarantees.

Executed: `cargo test --manifest-path src-tauri/Cargo.toml --locked --offline -j 1 --test stage3c_benchmark -- --ignored --nocapture`. One explicit benchmark passed in 81.18 seconds. The helper's first attempt incorrectly hardcoded parent revision 1; the existing concurrency check rejected it. Reading the real parent revision fixed the fixture, with no application change.

| Objectives | Export bytes | Open ms | Load ms | Save action ms | Review ms | Export ms | Preview ms | Restore to empty ms |
| ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| 10 | 85,669 | 12.704 | 8.466 | 18.533 | 20.636 | 13.333 | 24.045 | 72.694 |
| 100 | 276,379 | 25.984 | 21.181 | 41.980 | 44.268 | 33.222 | 58.192 | 133.649 |
| 500 | 1,124,379 | 111.518 | 83.716 | 165.265 | 161.616 | 120.844 | 232.247 | 401.672 |
| 1,000 | 2,184,379 | 202.548 | 183.712 | 364.036 | 470.049 | 323.958 | 663.339 | 931.508 |
| 1,500 | 3,244,879 | 459.417 | 468.688 | 793.048 | 602.446 | 440.080 | 1,435.452 | 1,758.937 |

Values are medians. At 1,500 objectives the worst observed save was 990.609 ms, preview 1,446.174 ms, and restore 2,233.406 ms. Ledger-to-H1 arithmetic (eight ledger rows) was below 0.003 ms throughout; this measures arithmetic only, not SVG rendering. The debug baseline exceeds the provisional 200 ms routine target at larger objective counts before Stage 3c. Release and opportunity-history results are required before deciding the severity and remediation.

An opt-in `ASCENSION_CAPTURE_BASELINE` fixture capture produced only ignored synthetic SQLite/JSON under `.artifacts/stage3b-baseline` after closing the service. It contains no personal data and is not committed.

## Implemented boundaries and persistence

The implementation preserves the existing React/TypeScript/Vite, Tauri 2, and private Rust/rusqlite architecture. No dependency, backend server, cloud account, external service, AI integration, telemetry, background monitor, notification service, or strategic game system is introduced. Stage 3b remains the only accomplishment and Build Credit authority. Its four-BC lifetime parent budget, immutable reward slots, criterion lineages, acceptance actor, idempotent settlement, correction policy, and ledger remain separate from observation and presentation.

Migration `004_study.sql` advances the database from version 3 to **4**, adding ten tables: `schedule_epochs`, `study_manifests`, `study_protocol_events`, `work_opportunities`, `study_assignments`, `opportunity_outcomes`, `study_exposures`, `study_burdens`, `study_reflections`, and `presentation_preferences`. Earlier migrations remain unchanged. The added STRICT tables contain validated JSON payloads with generated relational keys, foreign keys, bounded enums/positions, unique assignment and observation identities, indexes for opportunity timing/protocol, and UPDATE-rejecting triggers. Lifecycle changes and corrections append rows. Unique delivery identities prevent repeated cue/preview/checkpoint delivery writes for the same opportunity. Ordinary commands use the existing generation check and immediate transaction; failed assignment insertion must roll back the opportunity and generation together.

WorkOpportunity references the saved project, objective, action, and criterion. Rust snapshots action text and revision, stop condition, criterion description/version/registry revision, and the prospective meaningful-start definition. It also locks domain, cue, window, timezone context, epoch, availability/eligibility decision, priority, optional external deadline/context, and active protocol decision. The client does not submit replacement snapshot authority. Later Work edits cannot rewrite the historical declaration. Planned rest requires a pre-assignment unavailability reason; it consumes no E01 assignment. There is no automatic daily opportunity or retrospective ineligibility command.

Outcome rules are implemented in Rust and reflected in the UI:

| Observation | Current interpretation | Timing treatment |
| --- | --- | --- |
| Planned, open window, no observation | Pending | No invented observation or latency |
| Window elapsed, no sufficient observation | Missing | Unknown latency; the original opportunity remains |
| Exact start inside the inclusive declared window | Observed start | Recorded timestamp and known cue-to-start latency |
| Approximate start whose entire uncertainty interval fits | Observed start | Approximate time retained; exact latency remains null |
| Approximate interval crossing the boundary or unknown time | Missing after the window, with uncertainty reason | No manufactured precision |
| User reports no qualifying start after the window | Observed nonstart | Right-censored at the declared window; null exact start latency |
| Known first start after the window | Observed nonstart plus late-start flag | Late timestamp retained separately; primary window stays censored |
| Revised observation | Appended correction with reason and predecessor | Original observation and immutable assignment remain |

Optional priority advancement, domain-specific quality notes, and a same-objective Stage 3b acceptance link do not accept work or mint currency. Burden entries are exclusive user-estimated minutes, separated into planning, logging, review, construction inspection, correction/rework, other administration, and useful work. Weekly reflection is optional personal self-report, with no clinical interpretation.

## E00, E01, and the safe renderer contract

E00 configuration and activation are separate explicit actions. E00 never randomizes. The UI describes its provisional feasibility criteria: at least 90% usable eligible outcomes, median routine capture at most two minutes, meaningful work distinguishable from app interaction, and work/rest preserved. Five to ten ordinary workdays is a planning range. No engineering test, sample fixture, or automatic statistic declares human feasibility achieved.

E01 configuration freezes 60 positions in 15 balanced blocks of four, with two D and two D_PLUS_B in every block. `sha256-fy-v1` uses a local UUID seed, SHA-256 counter stream, rejection sampling, and Fisher-Yates shuffling; the stored allocation hash is checked against the exact sequence. Eligibility and definition lock before allocation; assignment commits before the 60-second pre-cue boundary. Non-overlapping presentation windows, chronological assignment consumption, uniqueness, and restart/restore persistence prevent rerolls. No assignment depends on app opens, earlier outcomes, motivation, or reward balance. Exhaustion requires a new explicit protocol decision.

Lifecycle states are configured, ready, active, paused, completed, and abandoned. E01 direct configured-to-active activation is rejected. Readiness review and activation require explicit user decisions; activation/resumption additionally requires the seven named acknowledgements covering E00 assessment, frozen protocol, epoch, assignment/window, matched support, as-assigned analysis, and voluntary stopping. The UI presents each as an unchecked statement and requires a written assessment/decision. The software records this owner attestation; it cannot independently establish genuine E00 participation. Declaring a new epoch pauses active E01 assignment while retaining earlier assignments. New-epoch opportunities need a protocol configured for that epoch. Users may leave without losing Work or fictional progress.

The typed IPC `Workspace.study` is deliberately different from complete export. It removes the seed/full allocation from manifests, adds derived lifecycle status, and redacts future assignment conditions until the declared preview boundary. It adds current outcome and presentation projections. Renderer state must not be used as a substitute portable record set. `load_workspace` computes missingness without fabricating an outcome event and computes presentation availability without recording exposure.

The backend declares a D_PLUS_B preview from cue minus 60 seconds to cue, hides construction during the active observation window, and permits the full scene at the post-window checkpoint. D has identical Work controls, cue wording, numeric progress, criteria, evidence, and reward accounting. Its automatic construction remains hidden. The frontend refreshes the safe projection at timing boundaries and when the app regains visibility. Construction preference is append-only, durable, and portable. Optional viewing outside E01 is separate ordinary use; E00 viewing is labeled nonexperimental exposure. The UI does not offer unscheduled construction overrides during active E01, although an explicitly reasoned exposure/deviation command exists in the bounded backend contract.

Delivery logging waits until the committed DOM intersects the viewport with nonzero dimensions, the document is visible, and no application modal covers it. Rendering does not imply a meaningful start or that the person noticed the scene. Noticing requires the separate self-report button. Failed delivery writes retain an explicit retry path; retry first reloads authoritative records to detect a committed response that was lost. Opportunity/boundary uniqueness prevents duplicate delivery history. The app has no closed-window cue delivery; unopened assigned opportunities remain in history without a fabricated delivery or seen event.

## H1 presentation and ledger integration

`src/world/HorizonScene.tsx` supplies original SVG artwork for one fixed riverside settlement. Its 64 frozen unit identities and authored order restore paths, structures, terraces, river infrastructure, civic details, and the beacon. Each additional issued BC changes exactly one restoration unit's SVG geometry until saturation. The pure component accepts only the committed issued-BC total and optional compact-preview flag. It has no persistence, reward, task, or acceptance command dependency.

The display is `min(64, workspace.progress.wallet_bc)`, using Stage 3b's ledger-derived total under policy `h1-ledger-64-v1`. It advances while hidden, including issuance in D, and retains fictional progress after factual acceptance correction because retained grants remain in the ledger. A four-BC grant restores exactly four units. At 65 or 100 BC the scene remains at 64; overflow remains in the ledger and no second world is created. Assignment/exposure records preserve the authoritative ledger sequence and issued-BC prefix plus saturation flag. Complete interchange v3 identifies the fixed H1 policy even without a study manifest.

World remains optional and below the saved action/progress workflow. Outside E01, numeric fiction and artwork remain hidden until the user chooses View construction. During E01 the same numeric information is present under both conditions; only the prescribed construction presentation differs. No collection click, spending, strategic choice, production, timer, punishment, or absence decay exists.

## Workflow, descriptive reporting, and burden

The existing saved action, stop condition, and resumption note remain the first Work content. Opportunity controls start collapsed and are independent of World. Ordinary planning reuses the selected objective/action, asks for an observable meaningful start and future work time, and defaults to the 15-minute window. Schedule, study, deadline, and context controls are secondary. Failed commands preserve drafts; switching objectives or reloading asks before discarding them. The observation form offers started, nonstart, and unknown, with optional factual context and a separate correction reason. The history preserves original declarations, every observation/correction, and exposure/deviation records. It loads 50 rows at a time when opened, rather than constructing an unbounded history on initial Work render.

E00 reports planned/eligible/ineligible counts, ascertainable outcomes, completeness, starts/nonstarts/missing/pending, late starts, technical problems, corrections, prospective definitions, and recorded burden. Capture estimates sum recorded planning/logging minutes per opportunity; missing estimates remain unknown. The UI explicitly notes that incomplete categories can understate the median. Useful work stays separate from administration, and domain quality does not become a universal score.

E01's primary descriptive denominator is every originally eligible assigned opportunity in the frozen review prefix. Missingness, technical problems, unrecorded delivery, deviations, and late starts remain visible. Bounds are S/N to (S+M)/N; opposing-arm extremes bound the difference. Pending observations prevent a final upper bound. The active protocol exposes arm comparisons only at the predeclared mature 30/60-assignment checkpoints; completion/abandonment permits a mature final assigned set. Later assignments are explicitly excluded from an earlier frozen checkpoint until the next planned review, with both counts shown. Epochs stay separate, with secondary deadline-within-48-hours and earlier/later-block descriptions. Exact-latency medians disclose their limited exact-report denominator. No significance test, uncontrolled schedule-transition effect, or population-level efficacy claim is generated.

## Portability and authority preservation

Complete interchange advances from v2 to **v3**, preserving Work, the entire Stage 3b authority graph, and all study arrays: epochs, frozen manifests/sequences/hashes, protocol events, opportunities, assignments, observation/correction history, exposure distinctions, burden/reflection, and presentation preferences. Restore validation checks global IDs, relationships, historical lifecycle decisions, chronological assignments, deterministic sequence/hash, timing/classification, immutable snapshots, and authoritative ledger prefixes before any replacement. Migration and restore retain the existing coherent backup and atomic replacement paths. Every accepted mutation remains subject to the shared 10,000-record and 10-MiB portability limits.

Legacy v2 may initialize compatible records with empty study state. Any v1/v2 replacement that would erase existing study history or preferences is rejected; v1's inherited earned-reward protection remains. The UI previews epoch, manifest, opportunity, assignment, outcome-event, exposure, burden, and reflection counts alongside existing reward-authority counts, then requires explicit replacement. Restore does not issue currency or rebuild authority from current factual acceptance counts. The native verification below establishes exact complete export/restore/reopen equivalence independently of mocked UI tests.

## Executed frontend verification

All UI/client tests use synthetic data and mock the desktop boundary; they establish interface and serialization behavior, not native database durability. The pure H1 tests inspect every fixed unit and actual SVG geometry. The Rust/native sections carry their separate authority and installed-app evidence.

| Command | Executed result |
| --- | --- |
| `npm.cmd test` | **94 passed**, 6 test files; includes 27 App, 11 ProgressPanel, 10 client, 14 study/World UI, 7 reporting, and 25 H1 tests. Independently repeated by the root agent after integration in 29.27 seconds. |
| `npm.cmd run typecheck` | Passed against current frontend contracts. |
| `npm.cmd run build` | Passed; 31 modules, JavaScript 342.61 kB (101.42 kB gzip), CSS 10.28 kB (2.90 kB gzip). Bundle size is not an interaction-speed measurement. |
| `npm.cmd test -- src/study/__tests__/StudyPanel.test.tsx` | **14 passed** after final modal/viewport delivery and retry changes. |
| Node syntax checks for `scripts/native-regression.mjs`, `native-progress.mjs`, `native-study.mjs`, and `native-offline.mjs` | Passed; `node --check scripts/native-study.mjs` was repeated after the final visibility-wait correction. The complete installed run also loaded and executed the final modules. |
| PowerShell syntax parsing for `scripts/native-dialog.ps1` and `scripts/verify-native.ps1` | Passed; the installed verification additionally executed both helpers. |
| `git diff --check` | Passed at frontend handoff and final closure; normal Git LF/CRLF notices were not whitespace errors. Every local checkpoint also received a staged-diff review. |

Coverage includes preserved Work/progress behavior; pending save/error handling; draft and modal focus; prospective rest/eligibility; exact, unknown, and late reporting; required correction context; explicit E00 beginning; every E01 acknowledgement; hidden future comparisons; all-assigned rate/difference bounds; epoch/deadline/checkpoint separation; matched D/D_PLUS_B numeric feedback; off-screen/modal delivery suppression; delivery without inferred seen; typed native-only commands; and no reward invocation by construction.

H1 cases include 0, 1, 2, 3, 4, 8, 16, 32, 48, 63, 64, 65, and 100 BC. An additional loop compares actual geometry at each of the 64 individual increments. Negative totals, fractional totals, NaN/infinity, and unsafe integers do not create fictional construction. Saturated geometry, deterministic remounting, textual unit status, keyboard focus, and absence of backend invocation are tested.

## Executed backend verification

An intermediate complete Rust run, `cargo test --manifest-path src-tauri/Cargo.toml --locked --offline`, passed **107 tests**: 65 library tests (including 31 study tests), 11 portability, 27 workspace, 3 targeted-mutation, and 1 bounded-history regression. Two explicit benchmarks were ignored in that ordinary run. Rust formatting and all-target strict Clippy also passed at that checkpoint. This predates final import-hardening regressions and is intermediate evidence, not the final count. The additive real-SQLite study suite exercises snapshots through edits/restart; eligibility/rest; precise boundary and uncertain/late outcomes; append-only corrections; E00 and E01 gates; frozen balanced allocation and redaction; P5T029 matched authority/presentation; P5T030 unopened/technical-failure/missing denominator retention through restore; epoch pause; complete/legacy portability; forged import rejection; database immutability; transactional rollback; concurrent stale plans; populated v3 migration backup; strict input shapes; durable preferences; malformed timestamps; same-millisecond lifecycle/exposure ordering; historical active-interval overlap; chronological assignment/correction integrity; ledger-prefix saturation; and delivery replay without inferred seen.

The subsequent preference-prefix import-hardening pipeline passed **110 tests**: 68 library (34 study), 11 portability, 27 workspace, 3 targeted-mutation, and 1 bounded-history regression; the two explicit benchmarks remained ignored in that ordinary run. `cargo test --manifest-path src-tauri/Cargo.toml --locked --offline`, `cargo fmt --manifest-path src-tauri/Cargo.toml -- --check`, and `cargo clippy --manifest-path src-tauri/Cargo.toml --locked --offline --all-targets -- -D warnings` passed at that checkpoint. The added exposure preference sequence checks the actual enabled/disabled history prefix, preserving causality even for later changes in the same millisecond. Assignment ledger prefixes must also be monotonic.

The final indexed source passed the same complete **110 tests**, strict all-target Clippy, and formatting checks again. The ordinary run intentionally ignores the two explicit timing cases. The 1,000-opportunity regression additionally verifies all 250 start, 500 nonstart, and 250 missing projections, then confirms that an appended correction immediately changes only its corresponding current projection without changing reward authority. Per-call maps replace repeated history scans; they do not cache authority across commands or omit validation.

## Accessibility and inspection limits

Work, opportunity, settings, and observation controls use native buttons, labels, selects, text areas, radio groups, checkboxes, and disclosure summaries. Visible error messages use alert roles; saving waits for backend confirmation. Draft dialogs retain focus, Escape cancellation, and keyboard traversal; new opportunity drafts participate in the inherited discard guard. The collapsed opportunity body is explicitly hidden from the accessibility tree, and modal return-focus targets only a visible active draft. Rendering does not impose timer use, a game interaction, or a daily questionnaire.

H1 has an SVG accessible name/description, explicit completed-unit text, a readable ordered restoration record, and geometric changes as well as color changes. It is static without waiting, with reduced-motion overrides, responsive artwork, and keyboard-operable disclosure. The preview uses the same fixed scene at compact height. The SVG keeps its authored colors under forced-colors mode; the surrounding text/list provides the nonvisual equivalent. Tables can scroll within their own container on narrow windows.

These implementation checks and DOM tests are not a formal WCAG conformance audit. No full screen-reader session or measured contrast audit has been recorded. The installed run separately exercised keyboard creation, modal Tab/Shift+Tab containment, Escape, guarded discard, focus restoration, and disclosure flows; actual native screenshots were inspected at ordinary and narrow sizes. The common file dialogs were real Windows dialogs operated by the native helper, not a comprehensive manual assistive-technology audit. Viewport/document visibility is a delivery approximation: it cannot establish attention or detect another application's window covering Ascension. The UI requires self-report for noticing.

## Defects found and repaired during implementation

- Initial collapsed schedule forms appeared in role queries and could steal return focus from an import modal. Explicit hiding and active, visible draft targeting corrected both; inherited Work tests and the new draft-focus test pass.
- Initial presentation/cue logging could count rendered but off-screen content. Intersection/geometry/document visibility and modal suppression now precede logging; positive and negative tests cover these boundaries without claiming that rendering means seen.
- A failed or competing automatic delivery mutation could leave its local attempt permanently marked. Failed keys are cleared, generation changes permit retry, and the visible retry reloads authoritative history first. Backend delivery uniqueness/replay protects committed responses from duplicate insertion.
- An early visibility preference was session-local. An append-only Rust-owned preference and portable history now preserve hiding through restart and restore.
- Opportunity criterion identity alone did not freeze a mutable draft's text. Criterion description and registry revision snapshots now retain the declaration independently of subsequent draft edits.
- Imported malformed exposure timestamps and historical lifecycle/assignment relationships needed stricter validation. Controlled timestamp checks, protocol-event references, active-history validation, chronological positions/corrections, and ledger-prefix checks address these cases. Required preference-history prefixes also reject forged delivery during an earlier disabled state while preserving valid equal-millisecond ordering.
- The exhaustive 64-step geometry test exceeded Vitest's five-second default while concurrent Rust compilation saturated the machine. Only that test received a 15-second timeout; its 64 geometry comparisons and all assertions remain unchanged. The complete 94-test suite subsequently passed.
- The baseline benchmark's hardcoded parent revision was a fixture defect, corrected by reading authoritative revision. No application concurrency check was weakened.

## Executed native Windows verification — 2026-10-20

The existing `verify-native.ps1`, installed-WebView2 CDP, native-dialog helper, and offline probe are extended by `scripts/native-study.mjs`. The completed run used a disposable package and isolated repository-local profile, verified the actual application identifier/data directory before destructive actions, and used only synthetic work/study fixtures. Time-shifted synthetic study exports exercised historical windows without inventing human participation or waiting through real study periods. Native commands, UI interactions, real file dialogs, storage inspection, and browser-only tests remain separately identified below.

The first installed attempt (`native-3b623728d78e4afbb9bb2eb22caf0f7b`) built successfully in 3m20s and passed 34 checkpoints with 25 native dialogs, including the inherited regression suite and populated Stage 3b migration. It then stopped on a harness timing defect: after opening the outer opportunity disclosure, the helper attempted to click a nested summary before React removed the hidden body. The failure capture showed the correct saved Work data and collapsed nested summary. The helper now waits for a nonzero visible rectangle before clicking. This changed test automation only. The failed run's evidence was retained, its test executable was confirmed removed by the uninstaller, and the complete suite was restarted with a new isolated package/profile. A partial run is not counted as the final native pass.

The complete rerun passed against unchanged application source **`990725a`**, with only the harness visibility wait repaired. Exact invocation from the repository root, using the already-established execution-policy workaround without changing machine policy:

```powershell
& ([scriptblock]::Create((Get-Content -LiteralPath scripts/verify-native.ps1 -Raw))) -Repository (Get-Location).Path
```

The wrapper invokes `npm.cmd run tauri -- build --config $config -- --locked --offline` with its generated isolated configuration. The successful run is **`native-8cce7f95efd54c8b86122e64ed450e44`**. Evidence is retained under the ignored repository-local `.artifacts/` directory: `native-result.json`, `package-result.json`, `presentation-timings.json`, screenshot captures, synthetic exports/backups, and dialog/probe records. No artifact contains real productivity records, and none is committed.

| Installed verification result | Evidence |
| --- | --- |
| Isolated release package build | Passed in **3m23s**; NSIS installer **2,688,729 bytes**. |
| Installer / uninstaller | Both exited **0**; the disposable installed executable was confirmed absent after cleanup. |
| Completed checkpoints | **44 passed**, including 12 separately verified installed launches. |
| Actual Windows dialogs | **41**: 11 Save selections, 1 Save cancellation, 28 Open selections, 1 Open cancellation. |
| Supported offline probes | **12 passed**, each with positive and negative network controls and native workspace reload/readback. |
| Installed process to ready Work | **12 launches**; minimum **904.42 ms**, median **971.84 ms**, maximum **1,354.86 ms**. |
| Full H1 presentation | Exactly **64 units**; warm automation-open-to-DOM observation **21.19 ms**. |

The isolated installer SHA-256 is `27CE5710D8B47FE55541EB872F13C61A90427F242AC123BFA29F2E071561D293`; the installed executable SHA-256 is `713BE080433058C21B4FA16CA25B34010F68FBC75A1BB2FD657A976740D075A5`. These identify the disposable verification package.

After installed verification, `npm.cmd run tauri -- build -- --locked --offline` also passed with the normal application configuration (release compilation 3m21s). The resulting `src-tauri/target/release/bundle/nsis/Ascension_0.1.0_x64-setup.exe` is **2,687,959 bytes**, SHA-256 **`2ECFEA6FFDB0ED466332FDC296193500403C6607EC61CD01A2879B07F6306A88`**. It uses the same verified application source at `990725a`; only the verification package's isolated identity/data-directory override differs. The normal package was built locally, not installed into the normal profile, committed, published, or released. Both packages remain unsigned and require existing WebView2.

Inherited Work checks used keyboard creation and editing, visible validation, modal focus/escape/discard paths, and actual Windows Save/Open dialogs. They verified exact IDs, relationships, revisions, selection, cancellation, import preview interruption, coherent displaced-state backup, replacement, no-op replay, backup recovery, and empty restore. Malformed, unsupported-version, duplicate-ID, dangling-reference, and oversized input rejected without mutation or backup creation. A failed backup also preserved state. Orderly close and abrupt process stop after acknowledged writes both recovered exact state and immediately exposed the saved action/note. The interruption checks do not simulate physical power loss during a transaction.

Populated Stage 3a schema-v2 and Stage 3b schema-v3 migration paths preserved complete legacy/progress data and created readable coherent recovery backups. The Stage 3b installed UI locked a criterion, recorded evidence, reviewed it, accepted work, and issued four BC. UI factual correction and later reacceptance retained four BC without reminting. Direct typed Rust probes checked duplicate acceptance and stale review rejection. Complete modern export/replacement/re-export/restart preserved authority history, command identities, and earned grants; unsafe legacy erasure rejected.

Stage 3c's installed **E00 UI** configured and began feasibility logging, planned a prospective opportunity with the saved action/start-definition snapshot, and preserved it through offline restart with zero assignments or new grants. The installed observation UI recorded a nonstart; direct typed Rust commands exercised exact start, an appended late-start correction, optional quality/burden/reflection, and untouched missing outcomes. None mutated reward authority. E01 accidental activation rejected, while explicit **synthetic** readiness/activation through real Rust IPC was exercised solely in the isolated profile. Four consumed assignments were balanced two per condition; safe renderer output hid future conditions and the seed/full sequence, complete export retained the frozen 60-position allocation and hash, and restart did not reroll assignments.

P5T029/P5T030 installed checks retained identical support, numeric four-BC feedback, ledger, and all assigned opportunities. D hid H1; D_PLUS_B showed the prescribed pre-cue preview and post-window four-unit scene. Delivered events did not imply seen. Technical failure, uncertainty, missing observations, and deviation remained in the assigned history. Full v3 export, empty restore, exact re-export, and offline restart preserved all outcomes/corrections, frozen assignments, and retained H1 progress. Unsafe v2 replacement and a forged allocation hash rejected without mutation.

Authentic synthetic Stage 3b acceptance/grant commands advanced H1 while hidden to 64 units, then to **68 issued BC with 64 visible units**, no spending, and 17 unique grants. Saturation appeared in exposure context. Exact saturated export/restore/restart retained those grants and the saved action. The root agent visually inspected actual installed Work, progress, preview, four-unit, completed-scene, and narrow-window screenshots: saved Work remained first; the warm riverside artwork was coherent and readable; the **660 × 850** window had no horizontal page overflow. These are bounded visual inspections, not a formal contrast or screen-reader audit.

Each offline probe first reached an owned loopback endpoint successfully, then demonstrated failed fetch/no server reach with WebView2 offline and reloaded the native app with unchanged readable Rust workspace data. Production CSP remained unchanged. Denial applies to the disposable **WebView2 target**, not Rust-process or OS-wide networking. The 12 process-to-Work observations were all below the provisional three-second target on this machine, but filesystem caches were not cleared and OS-cold boot was not measured. The 21.19 ms scene observation includes CDP round trips and polling; it is warm installed DOM evidence, not an isolated renderer benchmark or a guarantee for every interaction.

## Executed initial release opportunity performance — before scan indexing

The unchanged debug baseline above remains attributable to Stage 3b. This first release opportunity measurement **predates the targeted outcome-scan indexing optimization** and is retained as the before-change result. The benchmark holds ten objectives, eight evidence records/grants, and four factual acceptance corrections constant at 32 retained BC while scaling opportunity history. Its synthetic histories contain starts, nonstarts, missing and late starts, correction events, burden, and sampled cue exposure. Fixtures derive from the real planning command and pass ordinary import validation and real-SQLite restore. Timings retain integrity/exportability checks and FULL synchronization. The same machine context as the baseline applies.

Executed command:

```text
cargo test --manifest-path src-tauri/Cargo.toml --locked --offline --release --test stage3c_benchmark stage3c_opportunity_scaling -- --ignored --nocapture --test-threads=1
```

The optimized build completed in 4m32s; the explicitly selected benchmark passed in 12.38 seconds. Each cell below is **median / worst milliseconds across three samples**. The ignored local log is `.artifacts/stage3c-opportunity-benchmark.log`. These measurements use the Stage 3c service implementation at the backend checkpoint preceding final preference-prefix and monotonic-assignment-prefix import hardening, plus the benchmark working tree; they are not attributed to a later packaged executable. The ordinary fixture contains no manifests, assignments, or preferences and does not exercise those subsequently hardened historical-study branches.

| Opportunity history | 10 | 100 | 500 | 1,000 | 2,000 |
| --- | ---: | ---: | ---: | ---: | ---: |
| Serialized fixture bytes | 114,375 | 366,414 | 1,486,454 | 2,886,504 | 5,686,604 |
| Outcome events, including corrections | 11 | 110 | 550 | 1,100 | 2,200 |
| Outcome correction events | 1 | 10 | 50 | 100 | 200 |
| Burden entries | 10 | 100 | 500 | 1,000 | 2,000 |
| Cue exposure entries | 2 | 20 | 100 | 200 | 400 |

| Operation, median / worst ms | 10 | 100 | 500 | 1,000 | 2,000 |
| --- | ---: | ---: | ---: | ---: | ---: |
| Fresh repository open | 4.427 / 20.517 | 8.526 / 24.048 | 21.267 / 42.445 | 57.322 / 63.105 | 108.196 / 122.953 |
| Load workspace | 1.488 / 2.927 | 3.821 / 4.321 | 12.181 / 12.776 | 45.426 / 57.317 | 72.925 / 73.658 |
| Serialize renderer view | 0.470 / 0.502 | 1.871 / 2.573 | 10.391 / 11.347 | 26.443 / 26.640 | 64.624 / 67.797 |
| Save next action | 10.622 / 10.697 | 9.957 / 11.537 | 26.222 / 26.474 | 66.885 / 125.135 | 131.274 / 144.575 |
| Create opportunity | 7.532 / 10.282 | 8.503 / 10.334 | 30.773 / 32.940 | 85.480 / 101.498 | 142.197 / 151.799 |
| Record outcome correction | 6.448 / 10.880 | 9.859 / 9.872 | 28.259 / 31.155 | 102.287 / 108.407 | 185.331 / 237.917 |
| Review progress | 7.291 / 8.661 | 11.611 / 12.595 | 24.558 / 24.599 | 63.461 / 72.187 | 138.334 / 148.645 |
| H1 ledger projection | <0.001 / <0.001 | <0.001 / <0.001 | <0.001 / <0.001 | <0.001 / <0.001 | <0.001 / <0.001 |
| Export | 2.018 / 2.410 | 7.807 / 8.443 | 16.736 / 23.842 | 44.990 / 45.512 | 96.791 / 98.268 |
| Import preview/validation | 3.354 / 3.832 | 12.395 / 13.808 | 40.055 / 40.895 | 92.181 / 96.126 | 197.001 / 206.341 |
| Restore into empty | 79.713 / 103.532 | 96.751 / 100.584 | 174.806 / 175.902 | 252.471 / 339.539 | 527.570 / 568.095 |

Fresh repository open means a new SQLite connection with normal operating-system filesystem caching, not an OS-cold boot, full application startup, or WebView render. Service-operation timings exclude IPC and renderer-view serialization, which is separately measured. Outcome timings append reasoned corrections to historical observations and retain originals. The H1 number is an eight-row ledger fold and cap only, not SVG rendering. Three repetitions and uncontrolled desktop activity limit conclusions about variance or tail latency; worst observed is not a guaranteed upper bound.

At 2,000 opportunities the service's worst observed outcome write was **237.917 ms**, exceeding the provisional 200 ms routine target before serialization. Renderer serialization then had a 64.624 ms median. Those initial results did **not** establish that every routine end-to-end interaction met 200 ms. Repeated per-opportunity outcome scans in validation/view serialization motivated the bounded indexing change measured below; integrity checks were not bypassed. Repository open was comfortably below three seconds in these samples, but only native process-to-Work timing can assess the application cold-open target. The loose 30-second regression guard on 1,000 opportunities detects severe regressions and is not a product latency target.

## Executed final release opportunity performance — indexed source `990725a`

The final measurement uses application source commit **`990725a`**, including the complete preference-prefix and monotonic-assignment validation hardening. Bundled application SQLite is **3.53.2**, identified from release build binding metadata; this is the Rust application's bundled SQLite version, not the Node harness's separate SQLite runtime. The same exact release benchmark command, hardware, three samples per operation, synchronization, and validation checks described above were used. Compilation took 4m14s and the selected benchmark passed in 9.69 seconds. The ignored local evidence is `.artifacts/stage3c-opportunity-benchmark-final.log`.

The optimization builds opportunity/latest-outcome maps once per renderer serialization and groups outcome histories once per validation call. It preserves chronology, classification, orphan/reference checks, and generation semantics. These maps live only for the current call; no authority is cached across commands. The final 110-test suite additionally checks the 1,000-opportunity view's 250 starts, 500 nonstarts, and 250 missing outcomes, followed by an immediately reflected correction without reward mutation.

Record counts remain those in the preceding fixture table. Final serialized fixture sizes are **114,443; 367,094; 1,489,854; 2,893,304; and 5,700,204 bytes** respectively; the small increase reflects the final causal preference-prefix field on exposure records. Each timing cell remains median / worst milliseconds.

| Operation, median / worst ms | 10 | 100 | 500 | 1,000 | 2,000 |
| --- | ---: | ---: | ---: | ---: | ---: |
| Fresh repository open | 5.827 / 21.177 | 9.243 / 35.957 | 24.830 / 39.664 | 43.905 / 54.608 | 96.779 / 107.301 |
| Load workspace | 1.518 / 1.841 | 4.047 / 4.561 | 11.722 / 12.169 | 25.127 / 27.256 | 59.215 / 60.686 |
| Serialize renderer view | 0.358 / 0.437 | 2.096 / 2.189 | 8.887 / 9.645 | 18.015 / 18.954 | 38.226 / 41.376 |
| Save next action | 7.007 / 7.036 | 11.103 / 11.935 | 24.408 / 25.103 | 44.847 / 52.948 | 99.778 / 101.854 |
| Create opportunity | 6.351 / 7.107 | 12.640 / 12.802 | 24.932 / 25.808 | 51.296 / 52.123 | 114.492 / 117.259 |
| Record outcome correction | 5.456 / 9.143 | 12.129 / 12.610 | 24.331 / 26.583 | 50.933 / 52.096 | 112.144 / 113.524 |
| Review progress | 6.049 / 7.075 | 11.830 / 12.439 | 21.671 / 21.735 | 41.455 / 43.768 | 94.000 / 94.058 |
| H1 ledger projection | <0.001 / <0.001 | <0.001 / <0.001 | <0.001 / <0.001 | <0.001 / <0.001 | <0.001 / <0.001 |
| Export | 1.997 / 2.580 | 5.602 / 6.585 | 14.638 / 14.793 | 33.086 / 33.395 | 69.052 / 83.601 |
| Import preview/validation | 3.596 / 3.853 | 9.512 / 13.788 | 28.378 / 28.648 | 55.897 / 56.520 | 127.284 / 130.293 |
| Restore into empty | 80.493 / 83.449 | 124.000 / 129.756 | 150.496 / 150.733 | 222.605 / 224.898 | 442.528 / 462.980 |

For 2,000 opportunities, outcome-write median improved from **185.331 to 112.144 ms**, and renderer serialization median from **64.624 to 38.226 ms**. Worst observed outcome-write time fell from **237.917 to 113.524 ms**; worst serialization from **67.797 to 41.376 ms**. All measured routine service operations in the final run were below 200 ms. Final import preview/validation was at most 130.293 ms in these samples, and bulk restore at most 462.980 ms.

The sum of the separately measured worst outcome-write and serialization values is **154.900 ms before IPC and rendering**. This is a component-budget comparison, not a directly observed end-to-end interaction time or a guaranteed upper bound. Fresh repository open's worst observed 107.301 ms also excludes WebView creation, full process startup, and OS-cold filesystem behavior. The small sample count and uncontrolled desktop activity remain limitations. The initial and final tables support the targeted scan optimization while retaining full validation; they do not establish human capture burden, behavioral effectiveness, or every native UI latency target.

Native process-to-Work and scene responsiveness remain separate evidence in the native verification section. Repository timings, arithmetic projection, and renderer bundle size are not substitutes for those measurements.

## Remaining limits, readiness, and final decision

- This is a local single-user instrument. Wall-clock timestamps and supplied timezone context are retained; no external trusted time, behavioral monitoring, cryptographic owner tamper-proofing, or automated evidence validator is claimed.
- Cue delivery requires an open visible app. Missing/undelivered opportunities remain assigned. Experiments cannot be described as matched in actual exposure merely because the assigned rules were matched.
- Approximate timing, self-reported quality, optional capture estimates, incomplete administrative categories, and voluntary exposure constrain interpretation. Missing capture estimates are not zero. No measured human two-minute capture median or five-minute weekly-review burden has been established.
- The fixed scene saturates at 64 units. Legitimate BC continue accumulating, but the visual treatment then has reduced opportunity to change; saturation is part of the historical context, not a new reward policy or automatic study stop.
- The frozen E01 manifest has 60 positions and fixed 30/60 review checkpoints. Different windows, review rules, or a changed schedule require an explicit versioned protocol decision rather than in-place amendment or favorable rerandomization.
- Complete restore is explicit replacement with backup, not automatic record merging. The shared size/count bound remains intentional; larger histories need a separately reviewed scaling effort.
- E00 may begin only when the owner chooses ordinary feasibility logging after engineering gates pass. No genuine E00 participation has been fabricated. E01 additionally requires actual E00 assessment, protocol/epoch/reveal/analysis review, and separate explicit owner authorization. Synthetic activation in a disposable test profile is not a completed human experiment.
- The engineering separation can inform a future Stage 4 review, but neither this implementation nor synthetic tests establish a behavioral justification for strategic gameplay. Stage 4 remains unauthorized.

Changed implementation groups are `src-tauri/src/study.rs` and `study/tests.rs`, migration 004, the existing domain/storage/service/commands/lib integration, updated portability/workspace tests, `src/study/**`, `src/world/**`, App/styles, typed client and regression fixtures, benchmark/native scripts, and public specs/contracts/ADRs/README. No private productivity records, real database, export, installer, credential, or private research package belongs in Git. Generated synthetic files, profiles, screenshots, and packages remain ignored artifacts.

**Engineering decision: PASS.** The final application source passes 94 frontend tests, 110 Rust tests, type checking, frontend build, Rust formatting, strict all-target Clippy, the complete 44-check installed run with 41 dialogs and 12 supported offline probes, and the final release opportunity benchmark. No known blocking defect remains after the documented repairs. Measured routine service operations are below 200 ms in the final samples, and all 12 installed process-to-Work observations are below three seconds. Component measurements and the single warm scene observation do not establish every end-to-end UI latency target or human capture burden.

The branch is ready for independent code/product review and is suitable for merge **conditional on owner review and approval**, with the measurement and accessibility limits above retained. No real E00 or E01 participation occurred. E00 still requires a voluntary ordinary-use feasibility period; E01 still requires its genuine E00 assessment and separate explicit protocol/activation decision. A future Stage 4 decision remains separate and unauthorized. Passing engineering tests does not establish a behavioral benefit from construction.

Verified local checkpoints on `feat/stage-3c-opportunities-h1`:

| Commit | Scope |
| --- | --- |
| `f17f313` | Authorized Stage 3c scope and baseline evidence. |
| `2a39b07` | Durable opportunity and study observation ledger. |
| `a2f18fd` | Optional opportunity workflows and fixed Horizon scene. |
| `990725a` | Historical presentation validation and indexed opportunity projections; final application source used by release/native verification. |

The final documentation/harness checkpoint contains this closure record, the README update, and the completed installed-app suite. Its commit hash and post-commit working-tree check are reported in the owner handoff (a commit cannot include its own hash). Application source remains `990725a`; subsequent changes are documentation and verification scripts. All work remains local on `feat/stage-3c-opportunities-h1`, based on merged `b40a7f0`. No push, merge, publication, release, or deployment occurred.

## Complete changed-file inventory

Relative to the merged Stage 3b base, 43 files are added or modified. `A` means added; `M` means modified. Dependencies and lockfiles, earlier migrations, the Stage 3b reward engine, Tauri capabilities/configuration, and historical verification reports remain unchanged. No GitHub Actions workflow is added.

```text
M AGENTS.md
M README.md
M contracts/README.md
A docs/decisions/ADR-0005-opportunities-and-construction.md
A docs/implementation/STAGE_3C_VERIFICATION.md
M docs/specs/ARCHITECTURE_BOUNDARIES.md
A docs/specs/STAGE_3C.md
M scripts/native-progress.mjs
M scripts/native-regression.mjs
A scripts/native-study.mjs
A src-tauri/migrations/004_study.sql
M src-tauri/src/commands/mod.rs
M src-tauri/src/domain/mod.rs
M src-tauri/src/lib.rs
M src-tauri/src/service.rs
M src-tauri/src/storage/mod.rs
A src-tauri/src/study.rs
A src-tauri/src/study/tests.rs
M src-tauri/tests/portability.rs
A src-tauri/tests/stage3c_benchmark.rs
M src-tauri/tests/workspace.rs
M src/App.css
M src/App.tsx
M src/__tests__/App.test.tsx
M src/platform/__tests__/client.test.ts
M src/platform/client.ts
M src/progress/__tests__/fixtures.ts
A src/study/OpportunityEditor.tsx
A src/study/OutcomeEditor.tsx
A src/study/StudyPanel.tsx
A src/study/StudyReport.tsx
A src/study/StudySettings.tsx
A src/study/WorldPanel.tsx
A src/study/__tests__/StudyPanel.test.tsx
A src/study/__tests__/fixtures.ts
A src/study/__tests__/reporting.test.ts
A src/study/emptyStudy.ts
A src/study/fields.tsx
A src/study/reporting.ts
A src/study/useVisible.ts
A src/world/HorizonScene.css
A src/world/HorizonScene.tsx
A src/world/__tests__/HorizonScene.test.tsx
```
