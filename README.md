# Ascension

Ascension is a Windows-first, local-first workspace for resuming a concrete next action. [Stage 3b](docs/specs/STAGE_3B.md) adds prospective criteria, evidence, local acceptance, corrections, and fictional Build Credits. [Stage 3c](docs/specs/STAGE_3C.md) adds optional work opportunities, personal-study infrastructure, and a fixed restoration scene. [Stage 4](docs/specs/STAGE_4.md) adds River Reach, one optional strategy chapter of The Beacon Commonwealth. It requires no AI, account, or network service.

Stages 3a–4 are merged. [Stage 4.5](docs/specs/STAGE_4_5.md) adds objective organization, compact factual checkpoints, voluntary initiation support, easier ordinary planning, and automatic local recovery. Current evidence and limitations are in the [Stage 4.5 verification log](docs/implementation/STAGE_4_5_VERIFICATION.md). Historical [Stage 4](docs/implementation/STAGE_4_VERIFICATION.md), [Stage 3c](docs/implementation/STAGE_3C_VERIFICATION.md), [Stage 3b](docs/implementation/STAGE_3B_VERIFICATION.md), and [Stage 3a](docs/implementation/STAGE_3A_VERIFICATION.md#verification-closure--2026-10-19) reports remain separate.

The Work view keeps the next action and resumption note primary. Progress review, opportunity tracking, and World are optional. No timer, task click, or game action demonstrates accomplishment. There is no external verification runner, cloud sync, telemetry, payment, or automatic ingestion.

## Architecture

The accepted stack is React + TypeScript + Vite, a Tauri 2 desktop shell, and private SQLite persistence through Rust/rusqlite. Rust owns record identifiers, validation, state transitions, and durable writes. The frontend receives typed commands; it receives no arbitrary SQL, shell, or filesystem API.

- [Architecture boundaries](docs/specs/ARCHITECTURE_BOUNDARIES.md)
- [ADR-0001: initial scope](docs/decisions/ADR-0001-initial-scope.md)
- [ADR-0002: records, selection, and explicit restore](docs/decisions/ADR-0002-stage-3a-records-and-restore.md)
- [ADR-0003: targeted ordinary mutations](docs/decisions/ADR-0003-targeted-mutations.md)
- [ADR-0004: progress authority and portability](docs/decisions/ADR-0004-progress-authority-and-portability.md)
- [ADR-0005: opportunities and construction](docs/decisions/ADR-0005-opportunities-and-construction.md)
- [ADR-0006: River Reach capacity and spending](docs/decisions/ADR-0006-river-reach-economy.md)
- [ADR-0007: daily Work and recovery](docs/decisions/ADR-0007-daily-work-and-recovery.md)
- [Record, command, and interchange contracts](contracts/README.md)

The contract defines typed commands, stable identities, validation limits, complete interchange v5, and bounded legacy v1–v4 compatibility. Database migration v6 is separate from interchange versions.

## Work flow

1. Create a project and a bounded parent objective with a criterion describing the recognizable outcome.
2. Save one next action, its observable stopping condition, and an optional resumption note. The screen confirms a save only after Rust commits it.
3. Reopen the app to load the saved selection, action, and note from SQLite. The Work view puts the action and note first; the objective and criterion can be expanded below them.
4. Edit the saved records as needed. Failed saves keep the draft and the previously saved state visible. Switching away from an unsaved draft requires an explicit discard choice.

Each objective has at most one action and one note. Clearing a note removes that optional record; adding a note later creates a new identity. No click or timer marks an objective accomplished.

**Manage objectives and priority** provides explicit ordering, focus, pause, archive, and restoration. Archiving is organization, independent of factual completion; it preserves all history and credits. If the focused objective becomes inactive, Work shows the next active priority, or a clear invitation to restore/create one when none remain. **Resume work** is optional intent only. **Save where I stopped** saves a short factual checkpoint together with the next action and resumption note. Corrections append to history; checkpoints and references do not accept work or award credit.

**Need a different way to start?** offers a smaller user-written action for this visit, another active priority, or rest. The main action and criterion remain saved. Rest creates no penalty or outcome. No timer or questionnaire is required.

Expand **Progress and evidence** to review and lock a criterion before eligible work. Record concise evidence, save a review snapshot, inspect its complete registered basis, then accept, reject, or leave it pending. Locked criteria require prospective new versions; an objective's lifetime allocation never exceeds four fictional BC. Accepted progress and rewards are distinct. Corrections preserve history and retain issued credits while keeping the entitlement consumed. Reacceptance requires fresh review and cannot mint the original allocation again. **Show fictional rewards** reveals the ledger-derived balance; the normal work workflow remains available with it hidden.

## Optional opportunities and construction

Under **Work opportunities**, confirm the meaningful first action and future available work time; ordinary planning needs no manual schedule setup. The backend records its ordinary context and freezes the saved action, definition, cue, eligibility, and observation window. Advanced schedule and study options remain available for deliberate protocol association. No daily opportunity is required. Planned rest remains legitimate; optional energy context does not automatically exclude an opportunity.

After the cue, record a meaningful start, a known nonstart, or uncertainty. A missed observation stays missing, with unknown latency. An approximate time that crosses the window boundary stays uncertain. Later starts and corrections preserve earlier reports. Optional exclusive time estimates, priority advancement, domain-specific quality, and weekly reflection remain separate from reward acceptance.

E00 is explicitly started feasibility logging. The proposed 90% completeness and two-minute capture targets require actual ordinary use; engineering tests do not prove feasibility. E01 configuration freezes a seeded balanced allocation and protocol without activating it. Readiness and activation require explicit assessment and owner acknowledgments. Both arms use identical work support, numeric feedback, and authoritative credits; the additional treatment is a 60-second pre-cue construction preview and a checkpoint after the observation window. Future allocations remain hidden in the ordinary interface. Users may pause, leave, or disable construction. Closed-app cues are not delivered in the background.

**View construction** opens the optional original SVG settlement. One issued BC restores one of 64 fixed units. The scene progresses while hidden, retains courtesy credits after factual correction, and saturates at 64 while legitimate ledger issuance continues. It cannot grant, spend, reserve, or collect credits. There are no strategic choices or absence penalties. Construction is an untested presentation hypothesis, not evidence of increased productivity.

## Optional River Reach chapter

Open **World**, then enter River Reach. Six named parcels support six building types: three two-site specialists and three compact mixed buildings. Compare Engineering, Knowledge, and Diplomacy against the visible Stone Causeway, River Compact, and Beacon Mesh requirements. These are fictional settlement capabilities, not personal attributes.

Choose a building and parcels, inspect its preview, and queue a blueprint. Reserve available credit now or leave the blueprint pending indefinitely. Confirm construction when the needed credit is available. Each four BC permanently funds one unit of province capacity, up to six units and 24 BC. **Issued**, **spent**, **reserved**, and **available** remain distinct. The original 64-unit restoration scene still uses cumulative issuance, so spending never reverses it.

Reconfigure funded capacity freely: move or rotate buildings, remove them, or replace a specialist with compact buildings. Preview the whole proposed layout before committing, and undo the latest configuration if desired. Cancel a blueprint before reconfiguring. These operations never refund investment or issue credit. Factual accomplishment corrections retain the province and show its retained-credit provenance.

When the committed layout meets a chosen route, explicitly preserve its ending in the **Chronicle**. All three endings are legitimate and award no extra credit. After completion, explore alternate configurations in a labeled sandbox while the original ending remains inspectable. Buildings, reservations, and endings do not decay during absence.

Keyboard controls provide every placement action without dragging. The strategic renderer loads only on voluntary navigation. H2 is withheld during active E00/E01; return to Work to explicitly pause or leave a study. H1 and frozen protocols remain separate. Before real E01 activation, its known post-window H1 checkpoint duration needs a separately versioned policy correction. Gameplay and engineering tests establish no productivity benefit.

## Windows prerequisites

The following environment was verified during the initial setup:

| Prerequisite | Verified version |
| --- | --- |
| Node.js / npm | `22.23.3` / `10.9.9` |
| Rust / Cargo | `1.99.1`, stable `x86_64-pc-windows-msvc` |
| Visual Studio Build Tools 2022 | `17.14.37806.7`, Desktop development with C++ |
| MSVC / Windows SDK | Toolset `14.44.35207` / SDK `10.0.26100.0` |
| Microsoft Edge WebView2 runtime | `155.0.4283.45` |
| Windows | x64, version `25H2`, build `26200.9711` |

Use the MSVC Rust target and install the C++ compiler and Windows SDK components. Rust formatting also requires rustfmt. Open a new terminal after installing Rust so that Cargo is on PATH; check with `rustc --version` and `cargo --version`. No PowerShell execution-policy change is needed: the commands below use `npm.cmd` directly.

The Windows installer is configured to skip the WebView2 bootstrap download, so WebView2 must already be installed on the target computer. Building from source initially requires dependency downloads; the packaged application's intended workflow is offline.

## Install and run locally

Run these commands from the repository root in PowerShell:

```powershell
npm.cmd ci --cache .npm
npm.cmd run tauri -- dev
```

`npm ci` installs the versions in `package-lock.json`; `.npm/` keeps its download cache inside the ignored repository directory. Cargo uses its standard local cache. A first native build also needs the dependencies in `src-tauri/Cargo.lock` and Tauri's packaging tools. Offline builds require those caches to have been populated beforehand.

With the npm cache populated, `npm.cmd ci --offline --cache .npm` reinstalls from the lockfile without fetching packages; this command was verified during setup.

For frontend layout work, `npm.cmd run dev` opens the Vite development server. Browser-only use cannot invoke native persistence commands and does not satisfy native acceptance.

## Checks and Windows package

```powershell
npm.cmd test
npm.cmd run typecheck
npm.cmd run build
$env:CARGO_BUILD_JOBS = '1'
cargo fmt --manifest-path src-tauri/Cargo.toml -- --check
cargo test --manifest-path src-tauri/Cargo.toml --locked -- --test-threads=1
cargo clippy --manifest-path src-tauri/Cargo.toml --locked --all-targets -- -D warnings
npm.cmd run tauri -- build -- --locked
```

The automated test, typecheck, frontend build, and formatting commands passed for the implemented slice. The Rust tests exercise real temporary databases, migration backups including committed WAL data, restart persistence, stale writes, exact export/restore, import rejection, and rollback. UI tests cover saved-state display, editing, keyboard interaction, failure recovery, explicit replacement, and error feedback.

The final `-- --locked` forwards the lockfile requirement to Cargo. Tauri runs the frontend build and creates the configured NSIS installer under `src-tauri/target/release/bundle/nsis/`. The normal package is named `Ascension_0.1.0_x64-setup.exe`; the new isolated harness produces `Ascension Verification <run-id>_0.1.0_x64-setup.exe`.

For repeatable installed-app lifecycle checks, run `scripts/verify-native.ps1`. It builds a unique verification package, silently installs into an ignored repository-local directory, verifies its actual disposable data path, drives its WebView2 UI through loopback CDP and real Windows file dialogs through a bounded UIA/Win32 helper, then uninstalls that test package. No normal user profile is opened. The [verification log](docs/implementation/STAGE_4_5_VERIFICATION.md) records the exact invocation for shells that disable `.ps1` files, plus results and limits. The harness covers populated legacy migration, keyboard entry/save, editing, focus/validation, evidence and acceptance, duplicate/stale settlement, correction without reminting, complete export/restore, rejection, backups, graceful restart and abrupt-stop recovery. Its synthetic study checks exercise prospective opportunities, honest missingness, explicit E01 activation, matched presentation, retained corrections, and the 64-unit construction cap. River Reach checks additionally cover migration from populated schema v4, study withholding, partial reservations, construction, free layouts, undo, Chronicle, correction retention, full portability, keyboard controls, and a resized native window. Daily-use checks add lifecycle/priority, atomic checkpoints and replay, voluntary support, setup-free ordinary opportunities, failed-backup retry, full snapshot recovery, and corrupt/zero-byte primary preservation.

The offline helper verifies an ordinary renderer request succeeds online and fails without reaching its owned loopback server while offline, then reloads the app and checks unchanged saved records. It leaves that WebView2 target offline for the native workflows and repeats the probe after each restart. A refused proxy is also set from process launch. OS and Rust-process networking are unchanged. Installers remain unsigned and require preinstalled WebView2. No signing, public release, or CI deployment was performed.

For the targeted restore reliability gate, pass `-RestoreStress` to `scripts/verify-native.ps1`. It retains the complete installed suite and adds repeated fresh-profile restores, restart comparisons, recovery, and explicit failure/retry fixtures. Only the isolated verification package enables bounded, content-free restore diagnostics; the ordinary application emits none. Results and remaining uncertainty are recorded in the dated diagnostic closure in the Stage 4.5 verification log.

## Local data, export, and recovery

The normal Windows build stores its SQLite database at `%APPDATA%\io.ascension.work\ascension.sqlite3`. Its `backups` subdirectory holds recovery files. Development and the normal installed build use this identifier unless an explicit isolated test configuration overrides it.

Use **Export records** to save a complete portable JSON copy of committed work and progress history outside the application's data folder. Unsaved drafts are excluded. Exports preserve UUIDs, revisions, selection, criterion history, evidence, snapshots, corrections, consumed reward slots, grants, ledger and exact command replay identities. They exclude application paths and the current workspace generation; historical commands retain their original concurrency input. Use the export feature instead of copying an open SQLite database file, whose current data can also reside in its WAL sidecar.

**Import records** opens and validates a file, then displays local and incoming record counts. **Replace local records** is a separate confirmation: the incoming record set replaces the local set, including removal of records absent from the file. The app creates a synchronized `before-restore-<UUID>.json` backup first. If the backup or replacement fails, the existing workspace remains intact. Reimporting equivalent records is a no-op, preserving identities and revisions. Invalid JSON, duplicate identities, broken references, and unsupported schema versions are rejected before replacement.

Restore backups are complete schema-version-5 JSON and can be selected through **Import records**. They include lifecycle/order, checkpoint/correction/replay history, frozen studies, and complete River Reach accounting and Chronicle. Legacy v1 cannot erase earned rewards; v1/v2 cannot erase studies; v1/v2/v3 cannot erase World; v1–v4 cannot erase changed daily organization or checkpoints. Database upgrades create verified `before-migration-<UUID>.sqlite3` snapshots; these SQLite files are not JSON imports.

Automatic recovery creates at most one coherent snapshot after changed data on each local day of use. It retains the seven newest daily points plus one point in each of three newest represented months. Migration and pre-restore copies are never automatically pruned. The recovery panel shows the latest verified date and any nonblocking warning. A backup failure does not undo a successful save. **Preview recovery** uses the same explicit replacement confirmation; **Export recovery copy** writes the latest verified snapshot as portable JSON and also works from a storage-error screen. No database is automatically replaced. Backups contain entered work and evidence; keep them private. Save **Export records** to a user-chosen external/off-device location for protection against device loss: snapshots on the same device cannot provide that protection.

Modern imports and saved workspaces are bounded to 12 MiB and 12,500 records across all histories, including organization and checkpoints. Legacy formats retain their 10-MiB/10,000-record limits; the modern headroom accommodates the added migration metadata. Field limits are listed in the [contract](contracts/README.md). Legacy [synthetic sample](contracts/fixtures/sample-v1.json) and [empty](contracts/fixtures/empty-v1.json) fixtures remain for compatibility testing. Confirmed modern replacement can intentionally replace complete history, with a recovery backup; it is not a merge.

## Repository safety and contribution scope

Follow [AGENTS.md](AGENTS.md). Use synthetic records in fixtures and diagnostics. Keep personal work, credentials, database files, runtime backups, installer output, and private exports out of Git. The [CI setup](docs/implementation/CI_SETUP.md) defines read-only pull-request checks for the frontend and Windows Rust suites; installed Windows verification remains a separate local gate. There is no release automation, and local build commands do not push or publish anything. Product scope expansion beyond Stage 4.5 requires separate owner authorization.

Development stops at Stage 4.5. Restore remains replacement-only. Full erasure requires a separately reviewed namespace/tombstone policy; archiving hides work and factual correction preserves earlier accounts. Stage 5 and real E00/E01 enrollment by the implementation agent remain unauthorized. See the [release and recovery guidance](docs/implementation/RELEASE_AND_RECOVERY.md) before any normal-profile upgrade.
