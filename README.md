# Ascension

Ascension is a Windows-first, local-first workspace for resuming a concrete next action. Development is limited to the [Stage 3a offline vertical slice](docs/specs/STAGE_3A.md): a project, bounded parent objective, explicit criterion, next action, stopping condition, and optional resumption note, with durable local storage and versioned export/restore.

**Implementation is in progress.** The current scaffold connects a React Work screen to a narrow Rust status command. Four scaffold UI/client tests, TypeScript checks, and the frontend build have passed. The Windows NSIS package built, installed, and opened successfully, showing the status returned by Rust. Persistence, the full Work flow, and export/restore are not yet implemented; full Stage 3a acceptance remains pending. See the [executed verification log](docs/implementation/STAGE_3A_VERIFICATION.md) for actual commands, results, and unresolved gates.

Stage 3a must let someone save an action and note, close the app, and recover them immediately after reopening offline. It includes safe, explicit restoration of versioned records with stable identities. It excludes AI, a game economy, completion validation, accounts, cloud sync, telemetry, payments, and automatic ingestion.

## Architecture

The accepted stack is React + TypeScript + Vite, a Tauri 2 desktop shell, and private SQLite persistence through Rust/rusqlite. Rust owns record identifiers, validation, state transitions, and durable writes. The frontend receives typed commands; it receives no arbitrary SQL, shell, or filesystem API.

- [Architecture boundaries](docs/specs/ARCHITECTURE_BOUNDARIES.md)
- [ADR-0001: initial scope](docs/decisions/ADR-0001-initial-scope.md)
- [ADR-0002: records, selection, and explicit restore](docs/decisions/ADR-0002-stage-3a-records-and-restore.md)
- [Record, command, and interchange contracts](contracts/README.md)

The contract describes the implementation target. It does not establish that every command is already available.

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

Use the MSVC Rust target and install the C++ compiler and Windows SDK components. Rust formatting/lint commands also require rustfmt and Clippy. Open a new terminal after installing Rust so that Cargo is on PATH; check with `rustc --version` and `cargo --version`. No PowerShell execution-policy change is needed: the commands below use `npm.cmd` directly.

The Windows installer is configured to skip the WebView2 bootstrap download, so WebView2 must already be installed on the target computer. Building from source initially requires dependency downloads; the packaged application's intended workflow is offline.

## Install and run locally

Run these commands from the repository root in PowerShell:

```powershell
npm.cmd ci --cache .npm
npm.cmd run tauri -- dev
```

`npm ci` installs the versions in `package-lock.json`; `.npm/` keeps its download cache inside the ignored repository directory. Cargo uses its standard local cache. A first native build also needs the dependencies in `src-tauri/Cargo.lock` and Tauri's packaging tools. Offline builds require those caches to have been populated beforehand.

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

These are local verification commands, not a claim that every gate has passed. Required real temporary-SQLite tests and the full packaged smoke flow are still pending. The final `-- --locked` forwards the lockfile requirement to Cargo. Tauri runs the frontend build and creates the configured NSIS installer under `src-tauri/target/release/bundle/nsis/`. The initial scaffold produced `Ascension_0.1.0_x64-setup.exe` and passed installation and native launch checks.

The [verification log](docs/implementation/STAGE_3A_VERIFICATION.md) contains the native smoke checklist. Stage 3a remains incomplete until an actual packaged Windows app passes offline launch, keyboard operation, save/restart persistence, export/restore/replay, and rejection of malformed or unsupported imports without data loss. Browser results alone cannot satisfy this gate.

## Repository safety and contribution scope

Follow [AGENTS.md](AGENTS.md). Use synthetic records in fixtures and diagnostics. Keep personal work, credentials, database files, runtime backups, installer output, and private exports out of Git. This repository includes no CI workflow or release automation; local build commands do not push or publish anything. Scope expansion beyond Stage 3a requires an explicit decision.
