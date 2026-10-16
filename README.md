# Ascension

Ascension is a Windows-first, local-first workspace for resuming a concrete next action. Development is limited to the [Stage 3a offline vertical slice](docs/specs/STAGE_3A.md): a project, bounded parent objective, explicit criterion, next action, stopping condition, and optional resumption note, with durable local storage and versioned export/restore.

**Stage 3a is implemented and passed its documented Windows checks.** SQLite persistence, the Work screen, editing, versioned export, and explicit restore are implemented. Automated checks passed: 22 Rust tests (20 real temporary-SQLite integration tests and two file-command tests), 22 UI/client tests, TypeScript checks, frontend build, and Rust formatting. The installed smoke package passed the full native workflow under a verified WebView2 proxy-blocked offline simulation, including native rechecks of two error-visibility fixes. The final normal-configuration installer also built, installed, opened a readable Rust-backed Work view, and closed successfully. See the [executed verification log](docs/implementation/STAGE_3A_VERIFICATION.md) for the exact evidence and its limits.

Stage 3a must let someone save an action and note, close the app, and recover them immediately after reopening offline. It includes safe, explicit restoration of versioned records with stable identities. It excludes AI, a game economy, completion validation, accounts, cloud sync, telemetry, payments, and automatic ingestion.

## Architecture

The accepted stack is React + TypeScript + Vite, a Tauri 2 desktop shell, and private SQLite persistence through Rust/rusqlite. Rust owns record identifiers, validation, state transitions, and durable writes. The frontend receives typed commands; it receives no arbitrary SQL, shell, or filesystem API.

- [Architecture boundaries](docs/specs/ARCHITECTURE_BOUNDARIES.md)
- [ADR-0001: initial scope](docs/decisions/ADR-0001-initial-scope.md)
- [ADR-0002: records, selection, and explicit restore](docs/decisions/ADR-0002-stage-3a-records-and-restore.md)
- [Record, command, and interchange contracts](contracts/README.md)

The contract defines the implemented command shapes, stable record identities, validation limits, and schema-version-1 interchange format. Database migration versions are separate from export schema versions.

## Work flow

1. Create a project and a bounded parent objective with a criterion describing the recognizable outcome.
2. Save one next action, its observable stopping condition, and an optional resumption note. The screen confirms a save only after Rust commits it.
3. Reopen the app to load the saved selection, action, and note from SQLite. The Work view puts the action and note first; the objective and criterion can be expanded below them.
4. Edit the saved records as needed. Failed saves keep the draft and the previously saved state visible. Switching away from an unsaved draft requires an explicit discard choice.

Each objective has at most one action and one note. Clearing a note removes that optional record; adding a note later creates a new identity. No click or timer marks an objective accomplished.

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
npm.cmd run tauri -- build -- --locked
```

The automated test, typecheck, frontend build, and formatting commands passed for the implemented slice. The Rust tests exercise real temporary databases, migration backups including committed WAL data, restart persistence, stale writes, exact export/restore, import rejection, and rollback. UI tests cover saved-state display, editing, keyboard interaction, failure recovery, explicit replacement, and error feedback.

The final `-- --locked` forwards the lockfile requirement to Cargo. Tauri runs the frontend build and creates the configured NSIS installer under `src-tauri/target/release/bundle/nsis/`. The normal package is named `Ascension_0.1.0_x64-setup.exe`; the isolated native test package is named `Ascension Smoke_0.1.0_x64-setup.exe`.

The installed smoke package passed keyboard operation, save/restart persistence, exact export round trips, explicit empty/restore/replay flows, and rejection of malformed JSON, unsupported versions, duplicate IDs, and broken references. The final rebuild also passed native checks for visible criterion validation, guarded draft discard, and automatically revealed import errors; all four exported checkpoints remained byte-for-byte identical. Its application identity and data directory were isolated for synthetic testing. WebView2's unavailable-proxy flag was verified on the installed app's child process, and a connection-refusal probe preceded launch/reopen. This was a process-scoped offline simulation; OS networking was unchanged. The final normal package was separately installed and checked for native launch, successful Rust workspace loading, readable display, and close without the proxy. The full workflow was tested in the isolated variant; it was not repeated against the normal data directory.

The final normal installer is 2,253,514 bytes (approximately 2.15 MiB) and is unsigned. It requires the preinstalled WebView2 runtime described above. No signing, public release, or CI deployment was performed.

## Local data, export, and recovery

The normal Windows build stores its SQLite database at `%APPDATA%\io.ascension.work\ascension.sqlite3`. Its `backups` subdirectory holds recovery files. Development and the normal installed build use this identifier unless an explicit isolated test configuration overrides it.

Use **Export records** to save a portable JSON copy of committed records to a chosen location outside the application's data folder. Unsaved drafts are excluded. The export includes stable UUIDs, revisions, timestamps, relationships, and selected objective; it excludes local paths and workspace concurrency tokens. Use the export feature instead of copying an open SQLite database file, whose current data can also reside in its WAL sidecar.

**Import records** opens and validates a file, then displays local and incoming record counts. **Replace local records** is a separate confirmation: the incoming record set replaces the local set, including removal of records absent from the file. The app creates a synchronized `before-restore-<UUID>.json` backup first. If the backup or replacement fails, the existing workspace remains intact. Reimporting equivalent records is a no-op, preserving identities and revisions. Invalid JSON, duplicate identities, broken references, and unsupported schema versions are rejected before replacement.

Restore backups are ordinary schema-version-1 JSON and can be selected through **Import records**. A database upgrade creates a separate verified `before-migration-<UUID>.sqlite3` snapshot; these SQLite recovery files are not JSON imports. Backups remain local and are not automatically pruned. Both exports and backups contain the entered work text.

Imports and saved workspaces are bounded to 10 MiB and 10,000 records; field limits are listed in the [contract](contracts/README.md). [Synthetic sample](contracts/fixtures/sample-v1.json) and [empty](contracts/fixtures/empty-v1.json) exports are included for development. Confirming the empty export intentionally replaces the workspace with an empty set after backing up changed local records.

## Repository safety and contribution scope

Follow [AGENTS.md](AGENTS.md). Use synthetic records in fixtures and diagnostics. Keep personal work, credentials, database files, runtime backups, installer output, and private exports out of Git. This repository includes no CI workflow or release automation; local build commands do not push or publish anything. Scope expansion beyond Stage 3a requires an explicit decision.

Development stops at this Stage 3a boundary. Restore remains replacement-only, backups are retained without automatic pruning, and the Windows package assumes WebView2 is already installed. No Stage 3b/4 work is included or authorized by the completed checks.
