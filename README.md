# Ascension

Ascension is a Windows-first, local-first workspace for resuming a concrete next action. The authorized [Stage 3b scope](docs/specs/STAGE_3B.md) extends the saved project/objective/action/note workflow with prospective criteria, progress evidence, local user acceptance, corrections, and fictional Build Credit issuance. It requires no AI, game, account, or network service.

Stage 3a was verified and merged in PR #1. Stage 3b implementation and verification are tracked in the [Stage 3b executed verification log](docs/implementation/STAGE_3B_VERIFICATION.md). Historical Stage 3a results remain in its [verification log](docs/implementation/STAGE_3A_VERIFICATION.md#verification-closure--2026-10-19).

The Work view keeps the next action and resumption note primary. Progress review is optional and fictional rewards start hidden. No timer or task click demonstrates accomplishment. There is no game, spending, external verification runner, cloud sync, telemetry, payment, or automatic ingestion.

## Architecture

The accepted stack is React + TypeScript + Vite, a Tauri 2 desktop shell, and private SQLite persistence through Rust/rusqlite. Rust owns record identifiers, validation, state transitions, and durable writes. The frontend receives typed commands; it receives no arbitrary SQL, shell, or filesystem API.

- [Architecture boundaries](docs/specs/ARCHITECTURE_BOUNDARIES.md)
- [ADR-0001: initial scope](docs/decisions/ADR-0001-initial-scope.md)
- [ADR-0002: records, selection, and explicit restore](docs/decisions/ADR-0002-stage-3a-records-and-restore.md)
- [ADR-0003: targeted ordinary mutations](docs/decisions/ADR-0003-targeted-mutations.md)
- [ADR-0004: progress authority and portability](docs/decisions/ADR-0004-progress-authority-and-portability.md)
- [Record, command, and interchange contracts](contracts/README.md)

The contract defines typed commands, stable identities, validation limits, complete interchange v2 and bounded legacy v1 imports. Database migration versions are separate from export schema versions.

## Work flow

1. Create a project and a bounded parent objective with a criterion describing the recognizable outcome.
2. Save one next action, its observable stopping condition, and an optional resumption note. The screen confirms a save only after Rust commits it.
3. Reopen the app to load the saved selection, action, and note from SQLite. The Work view puts the action and note first; the objective and criterion can be expanded below them.
4. Edit the saved records as needed. Failed saves keep the draft and the previously saved state visible. Switching away from an unsaved draft requires an explicit discard choice.

Each objective has at most one action and one note. Clearing a note removes that optional record; adding a note later creates a new identity. No click or timer marks an objective accomplished.

Expand **Progress and evidence** to review and lock a criterion before eligible work. Record concise evidence, save a review snapshot, inspect its complete registered basis, then accept, reject, or leave it pending. Locked criteria require prospective new versions; an objective's lifetime allocation never exceeds four fictional BC. Accepted progress and rewards are distinct. Corrections preserve history and retain issued credits while keeping the entitlement consumed. Reacceptance requires fresh review and cannot mint the original allocation again. **Show fictional rewards** reveals the ledger-derived balance; the normal work workflow remains available with it hidden.

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
cargo fmt --manifest-path src-tauri/Cargo.toml -- --check
cargo test --manifest-path src-tauri/Cargo.toml --locked
cargo clippy --manifest-path src-tauri/Cargo.toml --locked --all-targets -- -D warnings
npm.cmd run tauri -- build -- --locked
```

The automated test, typecheck, frontend build, and formatting commands passed for the implemented slice. The Rust tests exercise real temporary databases, migration backups including committed WAL data, restart persistence, stale writes, exact export/restore, import rejection, and rollback. UI tests cover saved-state display, editing, keyboard interaction, failure recovery, explicit replacement, and error feedback.

The final `-- --locked` forwards the lockfile requirement to Cargo. Tauri runs the frontend build and creates the configured NSIS installer under `src-tauri/target/release/bundle/nsis/`. The normal package is named `Ascension_0.1.0_x64-setup.exe`; the new isolated harness produces `Ascension Verification <run-id>_0.1.0_x64-setup.exe`.

For repeatable installed-app lifecycle checks, run `scripts/verify-native.ps1`. It builds a unique verification package, silently installs into an ignored repository-local directory, verifies its actual disposable data path, drives its WebView2 UI through loopback CDP and real Windows file dialogs through a bounded UIA/Win32 helper, then uninstalls that test package. No normal user profile is opened. The [verification log](docs/implementation/STAGE_3B_VERIFICATION.md) records the exact invocation for shells that disable `.ps1` files, plus results and limits. The harness covers populated legacy migration, keyboard entry/save, editing, focus/validation, evidence and acceptance, duplicate/stale settlement, correction without reminting, complete export/restore, rejection, backups, graceful restart and abrupt-stop recovery.

The offline helper verifies an ordinary renderer request succeeds online and fails without reaching its owned loopback server while offline, then reloads the app and checks unchanged saved records. It leaves that WebView2 target offline for the native workflows and repeats the probe after each restart. A refused proxy is also set from process launch. OS and Rust-process networking are unchanged. Installers remain unsigned and require preinstalled WebView2. No signing, public release, or CI deployment was performed.

## Local data, export, and recovery

The normal Windows build stores its SQLite database at `%APPDATA%\io.ascension.work\ascension.sqlite3`. Its `backups` subdirectory holds recovery files. Development and the normal installed build use this identifier unless an explicit isolated test configuration overrides it.

Use **Export records** to save a complete portable JSON copy of committed work and progress history outside the application's data folder. Unsaved drafts are excluded. Exports preserve UUIDs, revisions, selection, criterion history, evidence, snapshots, corrections, consumed reward slots, grants, ledger and exact command replay identities. They exclude application paths and the current workspace generation; historical commands retain their original concurrency input. Use the export feature instead of copying an open SQLite database file, whose current data can also reside in its WAL sidecar.

**Import records** opens and validates a file, then displays local and incoming record counts. **Replace local records** is a separate confirmation: the incoming record set replaces the local set, including removal of records absent from the file. The app creates a synchronized `before-restore-<UUID>.json` backup first. If the backup or replacement fails, the existing workspace remains intact. Reimporting equivalent records is a no-op, preserving identities and revisions. Invalid JSON, duplicate identities, broken references, and unsupported schema versions are rejected before replacement.

Restore backups are complete schema-version-2 JSON and can be selected through **Import records**. Legacy v1 imports create unaccepted draft criteria and are rejected when replacement would erase earned history. A database upgrade creates a separate verified `before-migration-<UUID>.sqlite3` snapshot; these SQLite recovery files are not JSON imports. Backups remain local and are not automatically pruned. Both exports and backups contain entered work/evidence text.

Imports and saved workspaces are bounded to 10 MiB and 10,000 records across work and progress; field limits are listed in the [contract](contracts/README.md). Legacy [synthetic sample](contracts/fixtures/sample-v1.json) and [empty](contracts/fixtures/empty-v1.json) fixtures remain for compatibility testing. Confirmed modern replacement can intentionally discard reward history, with a complete recovery backup; it is not a merge.

## Repository safety and contribution scope

Follow [AGENTS.md](AGENTS.md). Use synthetic records in fixtures and diagnostics. Keep personal work, credentials, database files, runtime backups, installer output, and private exports out of Git. This repository includes no CI workflow or release automation; local build commands do not push or publish anything. Scope expansion beyond Stage 3b requires separate owner authorization.

Development stops at Stage 3b. Restore remains replacement-only, backups are retained without automatic pruning, and the unsigned Windows package assumes WebView2 is already installed. No Stage 3c/4 work is authorized.
