# Stage 3a implementation verification

This log records executed checks, not assumed outcomes. All examples and test records are synthetic. The engineering review below supersedes the earlier acceptance summary; the historical checkpoints remain evidence for their specifically recorded builds.

## Engineering review — 2026-10-19

Review began October 18 and concluded October 19 (America/New_York). Reviewed the completed implementation from clean commit `c378ee5` on `feat/stage-3a-offline-work`, against Stage 3a, the architecture boundaries, and ADRs 0001/0002. No Stage 3b work, framework change, network feature, dependency update, push, or merge was performed. Only synthetic temporary databases and uniquely identified disposable installed profiles were used; the normal installed app and its data were not opened.

### Confirmed defects and fixes

1. **Startup retry could never recover.** `AppState` cached a failed initial database open permanently. After a transient Windows file lock cleared, the UI's Retry connection still returned that cached failure. A regression with an exclusive handle on a populated temporary SQLite database failed before the fix. Failed initialization now retries the same guarded open path; healthy services stay open. The regression verifies the complete original workspace, including identity, generation, and revisions, survives recovery. Corrupt and unavailable databases still fail without replacement.
2. **Discard-to-editor keyboard focus was lost.** Dismissing the draft guard to open another editor restored focus to the old trigger, overriding the new editor's field focus. The regression failed before the fix. Dialog cleanup now prefers an enabled field in the active editor. Tab, Shift+Tab, Escape, canceled imports, and pending restore behavior remain covered.

Strict Clippy also exposed a preexisting complex boxed callback type in the malformed-import test. Noncapturing callbacks now use plain function pointers. This is test cleanup, not a domain change.

### Automated verification

Commands run from the repository root. This shell needed the existing toolchain directory added to its process PATH:

```powershell
$env:PATH = (Join-Path $env:USERPROFILE '.cargo\bin') + ';' + $env:PATH
```

| Exact command/check | Executed result |
| --- | --- |
| `git status --short`, required specifications, ADRs, manifests, lockfiles, source and tests | Clean baseline; architectural review completed |
| `node --version`; `npm.cmd --version`; `rustc -Vv`; `cargo --version`; `rustup show` | Node 22.23.3, npm 10.9.9, Rust/Cargo 1.99.1, stable x86_64-pc-windows-msvc |
| `vswhere` requiring `Microsoft.VisualStudio.Component.VC.Tools.x86.x64`; WebView2 registry inventory | Existing VS 2022 Build Tools found; WebView2 155.0.4283.62 installed |
| Baseline `npm.cmd test`; `npm.cmd run typecheck`; `npm.cmd run build` | 22 tests passed; typecheck and production frontend build passed |
| Baseline `cargo test --manifest-path src-tauri/Cargo.toml --locked --offline` | 22 tests passed: 2 command tests and 20 SQLite integration tests |
| `npm.cmd test -- --testNamePattern="keeps keyboard focus\|focuses the replacement"` | Before fix: 1 passed, 1 failed; after fix: both passed |
| `cargo test --manifest-path src-tauri/Cargo.toml --locked --offline retry_recovers_after_a_startup_file_lock -- --test-threads=1` | Failed before fix after releasing the exclusive file handle; passed within the final command suite after fix |
| `cargo test --manifest-path src-tauri/Cargo.toml --locked --offline commands::tests -- --test-threads=1`; `cargo test --manifest-path src-tauri/Cargo.toml --locked --offline --lib -- --test-threads=1` | Intermediate command suite: 7 passed; final library suite: 10 passed |
| `cargo test --manifest-path src-tauri/Cargo.toml --locked --offline --test workspace` | Baseline 20 passed; final expanded suite 27 passed |
| Final `npm.cmd test` | 30 tests passed: 24 Work UI and 6 typed-client tests; no ignored tests |
| Final `npm.cmd run typecheck` | Passed |
| Final `npm.cmd run build` (also executed by package builds) | Passed |
| Final `cargo test --manifest-path src-tauri/Cargo.toml --locked --offline` | 37 tests passed: 10 command/service tests and 27 real-SQLite integration tests; no ignored tests |
| `cargo fmt --manifest-path src-tauri/Cargo.toml -- --check` | Passed |
| `cargo clippy --manifest-path src-tauri/Cargo.toml --locked --offline --all-targets -- -D warnings` | Initially rejected the test callback type; passed after cleanup |
| `npm.cmd audit --json --cache .npm --registry https://registry.npmjs.org --fetch-retries=0 --fetch-timeout=10000` | Public registry reported 0 vulnerabilities across 142 dependencies; point-in-time npm result only |
| `node --check scripts/native-regression.mjs`; PowerShell AST parse of `scripts/verify-native.ps1` | Passed |
| `git diff --check` | Passed |

Added **23 automated regressions** (8 UI, 8 command/service, 7 SQLite integration). Important additional coverage includes:

- Real SQLite primary/foreign keys, singleton/unique constraints and revision bounds; simultaneous connections attempting the same generation commit exactly one winner.
- Action and note updates roll back together on late validation, note revision overflow, and injected SQL failure. Reopening confirms no partial save.
- Corrupt current-version records, invalid selection/generation, broken relations, missing state, and invalid metadata fail closed; future/unknown/physically corrupt databases remain covered.
- Restoring older records with conflicting local IDs requires explicit replacement, preserves the imported metadata, and produces an exact backup that restores the newer local records.
- Migration backups recover into a fresh profile with equivalent export bytes; live-WAL migration backups retain committed data. A subprocess exits without connection cleanup and its acknowledged workspace/export survive WAL recovery.
- Windows locked export destinations keep their prior bytes and leave no temporary export behind; app-data path traversal and case aliases are rejected, while a similarly named sibling export directory is allowed.
- Import reading accepts the exact 10 MiB boundary and rejects oversized files/directories. Existing malformed JSON, unknown/missing fields, duplicate IDs across record types, references, timestamps, revisions, future versions, stale/superseded previews, backup failures, replay and near-limit export tests remain passing.
- Keyboard entry/save and remount recovery, failed reload draft retention, overlong/null input, stale preview feedback, and exports while drafts exist. UI tests mock commands; SQLite and installed-app checks supply separate persistence evidence.

One new corruption fixture initially failed because SQLite correctly enforced foreign keys. The fixture now explicitly disables them only on its synthetic tampering connection; the application remains unchanged. The expanded integration suite then passed.

### Native package verification and repeatability

The Windows computer-use runtime failed before initialization with `failed to write kernel assets: The system cannot find the path specified. (os error 3)`. Reset and retry produced the same error. No native inputs were issued through that failed helper. The fallback harness uses Node 22's built-in WebSocket support to drive the **actual installed Tauri WebView2 renderer**, including real Rust commands and SQLite, over a loopback debug port enabled only for that disposable process. It adds no dependency or test command to the app.

`scripts/verify-native.ps1` builds with `npm.cmd run tauri -- build --config <generated-config> -- --locked --offline`, installs a uniquely named current-user NSIS package into `.artifacts/native-<run-id>/installed`, and overrides all app directories to its dedicated `profile` sibling. It runs `node scripts/native-regression.mjs <run-directory>`, then invokes only that installation's uninstaller. Synthetic result JSON and a screenshot remain ignored under `.artifacts/`. The normal app identity/profile is never used.

The script can be run directly where local script execution is enabled. This machine blocks `.ps1` execution, so the reviewed local script was executed as a script block without changing execution policy:

```powershell
& ([scriptblock]::Create((Get-Content -LiteralPath scripts/verify-native.ps1 -Raw))) -Repository (Get-Location).Path
```

The first full script invocation inside the restricted process sandbox built and installed successfully, but the app did not initialize SQLite or expose its CDP endpoint. It timed out with connection refusal and was uninstalled. This attempt is **not a native workflow pass**. A fresh invocation outside that process sandbox opened the real Rust-backed workspace and verified the WebView2 proxy flags. The harness initially timed out at Save project because its Enter event omitted the character payload; a read-only check confirmed zero saved projects. Adding the standard carriage-return payload to the automation fixed that issue without any app change. Reinstalling the same package and rerunning `node scripts/native-regression.mjs <isolated-run-directory>` passed all eight lifecycle checks; uninstall exited 0.

That successful package was `Ascension Verification be06e6dc_0.1.0_x64-setup.exe`, 2,254,475 bytes, SHA256 `BAD392FCEADA937FEAA409BE4C921E0C60227FF527076410942E330AA1E89762`. A final fresh full-script run then passed end to end, including explicitly isolated WebView2 environment variables and the result/cleanup path:

| Final native run | Executed result |
| --- | --- |
| Script-block invocation above, outside the restricted process sandbox | Exit 0; release compilation 1m56s; all eight native lifecycle checkpoints passed |
| Package | `Ascension Verification 4aa22caa_0.1.0_x64-setup.exe`, version 0.1.0, 2,254,298 bytes |
| Installer SHA256 | `59345D6A5F9E83CD4CED39498156E9AEA759DCD427890BADE883676B86E48FB8` |
| Installed executable SHA256 | `76EE7C6EA8F77B264E37BAED970F18251DC5CBB65E66FF58633C7091CC03A307` |
| Silent install `/S /D=<isolated-run>/installed` and own `uninstall.exe /S` | Both exit 0; installed executable absent after cleanup; no Ascension processes remain |
| Synthetic evidence | `.artifacts/native-4aa22caaf8a8443f930bcd8efd8277fa/`: `native-result.json`, `package-result.json`, `work-view.png`; ignored by Git |

No production-source changes occurred between the passing automated suites and these packages. The final harness was syntax checked and independently reviewed for installation/profile/process isolation. Only the focused code/tests/scripts/documentation are committed; no databases, exports, paths in generated configuration, screenshots, binaries or raw diagnostics are staged.

The native checks drive editor activation with mouse input, field entry/save with Tab/Return, and modal navigation with Tab/Shift+Tab/Escape. Assertions check exact saved field values, relationships, selected objective, IDs, revisions and creation timestamps through the real `load_workspace` command. They check visible/focused validation, draft guard behavior and the repaired replacement-editor focus. Native `WM_CLOSE` followed by relaunch preserves the entire workspace exactly; killing only the harness child process and reopening also preserves it. A captured renderer screenshot was inspected: the saved action, stop condition and resumption note are readable with no overlap. This is native installed-WebView2 evidence, not a browser mock or a screenshot of the native file dialogs.

The refusal probe connects only to loopback port 9. Each launch verifies that the installed application's actual WebView2 child has `--proxy-server=http://127.0.0.1:9` and the `tauri.localhost;ipc.localhost` bypass. The harness clears inherited WebView2 overrides and sets its own run-local profile and loopback debug port. No adapter, firewall, execution-policy or machine security settings change.

The normal configuration was also rebuilt using `npm.cmd run tauri -- build -- --locked --offline`: exit 0, release compilation 2m00s. `Ascension_0.1.0_x64-setup.exe` is 2,252,799 bytes, SHA256 `9758568DE90155F71A2260AEB0276172577888E2310A6B82C6EE0401705A8A04`, Authenticode `NotSigned`. It was **not installed/launched during this review**; native runtime evidence is from the isolated configuration with the same production code.

CDP cannot operate the Rust-owned native file dialogs. Export/import internals and UI confirmation/error behavior are tested separately; a current-build native file-picker-to-restore workflow must not be inferred from those separate tests. The older full native workflow remains recorded under Checkpoint E.

### Architecture, security, and remaining risks

- Rust owns persistence, UUIDs, validation, generation conflicts, transactions and native file access. The renderer invokes ten narrow commands and has no SQL/shell/filesystem plugin. CSP restricts connections to IPC and uses bundled assets. No AI, telemetry, cloud, account, game or completion authority was introduced.
- Restore is intentionally whole-workspace replacement, with validated retained previews, generation checks, a synchronized pre-restore backup and one transaction. It does not implement merge/conflict resolution. Migration snapshots use SQLite backup and include WAL commits.
- Writes reconstruct the bounded workspace transactionally. This is linear in total records and acceptable for this slice; no performance or large-scale claim was established.
- Corrupted storage fails closed and is not automatically deleted or recreated. There is no corrupted-database recovery UI; migration backup recovery was tested by copying a synthetic snapshot to a fresh profile.
- Unsaved drafts are explicitly labeled and guarded during in-app navigation; native window close does not save or guard an unsaved draft. Confirmed saves, rather than drafts, are the durability contract.
- Creating a second project retains the saved objective selection; select that project before creating its objective. This behavior was retained to avoid silently changing persisted selection.
- Proxy simulation applies to WebView2, not OS-wide or Rust networking. Abrupt-process recovery does not simulate physical power loss, disk failure or loss of filesystem directory metadata.
- Native file dialogs and screen reader operation could not be reverified by the available automation. The installer remains unsigned and requires preinstalled WebView2. Backups are local and unpruned. A RustSec advisory database was not fetched; the npm audit is not a complete supply-chain assurance.

**Recommendation: CONDITIONAL PASS for Stage 3a. Not yet cleared for Stage 3b implementation.** No confirmed data-loss or architectural defect remains in the reviewed scope, and both confirmed functional defects were fixed with failing-before/passing-after regressions. The current-build end-to-end native export/import dialog path remains unverified because the computer-use helper could not initialize. Complete that engineering verification with functioning native automation before promoting the Stage 3a gate; the repository owner's separate approval is still required for any Stage 3b scope. Routine QA has not been assigned to the user.

| Required area | Review result |
| --- | --- |
| Project/objective creation and editing | PASS: UI, real SQLite, installed app |
| Next action/note persistence and restart | PASS: exact records after orderly and abrupt process exit |
| Migrations, constraints, transactions and backups | PASS: real temporary SQLite, live WAL, rollback and recovery |
| Versioned export/import, IDs/revisions, replay and conflicts | PASS at backend/file and mocked UI boundaries; current native dialogs unverified |
| Malformed/corrupt/unsupported imports and storage | PASS at tested backend/UI boundaries; valid local state preserved |
| Windows packaging/installed lifecycle | PASS: normal build and isolated installation/lifecycle/uninstall; normal-profile runtime not repeated |
| Offline operation | CONDITIONAL: verified refused-proxy WebView2 simulation, not OS-wide denial |
| Basic keyboard and visible UI behavior | PASS for recorded native controls; screen reader/native dialog checks unverified |

Historical acceptance statements below apply only to their recorded source revisions and test conditions.

## Checkpoint A — repository and environment

Inspected on 2026-10-16. The repository was clean on `main` at `de99137`, with only instructions and planning documents. Read `AGENTS.md`, `CLAUDE.md`, both specifications, the coding-agent kickoff, and all existing ADRs. All required specifications are present. Implementation uses the accepted React/TypeScript/Vite + Tauri 2 + private Rust/rusqlite architecture. No scope or framework change is planned.

Work branch: `feat/stage-3a-offline-work`. No push or release is authorized.

| Check | Executed result |
| --- | --- |
| `git status --short` | Clean before implementation |
| `node --version` | `v22.23.3` |
| `npm --version` | PowerShell wrapper blocked by existing execution policy |
| `npm.cmd --version` | `10.9.9`; use this wrapper without changing execution policy |
| `git --version` | `2.54.0.windows.1` |
| `rustc --version`, `cargo --version`, `rustup show` | Commands absent; standard user installation executables also absent |
| Visual Studio Installer / Build Tools inventory | `vswhere` and standard Build Tools installations not found; no matching installed product metadata |
| Windows SDK inventory | Standard Windows Kits include directory and installed SDK metadata not found |
| WebView2 registry inventory | Runtime `155.0.4283.45` installed |
| OS metadata | x64, Windows build `26200.9711`, display version `25H2` |
| `Get-CimInstance Win32_OperatingSystem` | Access denied; used OS and registry metadata instead |

The user approved installing Rust stable/MSVC and Visual Studio Build Tools with the C++ desktop components and a Windows SDK. Installation and a native toolchain probe subsequently passed as recorded below. Installer artifacts and local build/test output are ignored by Git. No other projects were inspected.

Public npm registry access initially failed under the network sandbox (`EACCES`); the explicit registry query succeeded with approved tool access. Registry metadata reported `create-tauri-app` 4.7.5, Tauri CLI 2.12.3, Vite 8.3.4 (supports this Node version), and React 19.3.1. The selected dependency set is pinned in the manifests and lockfiles.

## Approved toolchain installation

The official installers were downloaded into the ignored `.toolchain/` directory from `https://win.rustup.rs/x86_64` and `https://aka.ms/vs/17/release/vs_BuildTools.exe`. The Microsoft installer had a valid Microsoft Corporation Authenticode signature. The Rust installer SHA256 matched `https://static.rust-lang.org/rustup/dist/x86_64-pc-windows-msvc/rustup-init.exe.sha256`.

The first Rust command used `--profile default` and exited 1: Windows reported a file lock while moving the optional `rust-docs` payload. The following retry omitted offline documentation and installed the compiler, Cargo, standard library, formatter, and linter successfully, with exit 0:

```powershell
.\.toolchain\rustup-init.exe -y --default-host x86_64-pc-windows-msvc --default-toolchain stable --profile minimal --component rustfmt --component clippy
```

The Visual Studio bootstrapper was started with elevation, `-WindowStyle Hidden`, and the following arguments. The process was awaited and its actual exit code was 0:

```text
--quiet --wait --norestart --nocache
--add Microsoft.VisualStudio.Workload.VCTools
--add Microsoft.VisualStudio.Component.VC.Tools.x86.x64
--add Microsoft.VisualStudio.Component.Windows11SDK.26100
```

Only the C++ desktop workload's required dependencies and the explicitly selected MSVC compiler/SDK components were requested; `--includeRecommended` was not used. No execution-policy changes were made.

| Verification | Executed result |
| --- | --- |
| `rustc -Vv` | Rust `1.99.1` (`58cf8f9c3`, 2026-10-13), host `x86_64-pc-windows-msvc`, LLVM `23.1.1` |
| `cargo -V` | Cargo `1.99.1` (`5f94df478`, 2026-08-27) |
| `rustup target list --installed` | `x86_64-pc-windows-msvc` |
| `rustup component list --installed` | Cargo, Clippy, rust-std, rustc, rustfmt for the MSVC host |
| `vswhere` requiring the workload, compiler, and selected SDK | Build Tools 2022 `17.14.37806.7`; complete and launchable; reboot required: false |
| MSVC installation metadata | Toolset `14.44.35207`; compiler file version `19.44.35229.0` |
| Windows SDK installation | `10.0.26100.0`; x64 resource compiler and `kernel32.lib` present |

A synthetic, repository-local native probe was compiled and executed:

```powershell
rustc --target x86_64-pc-windows-msvc .toolchain/native-toolchain-check.rs -o .toolchain/native-toolchain-check.exe
.\.toolchain\native-toolchain-check.exe
```

Both commands exited 0; the program printed `ascension-msvc-toolchain-ok`. This verifies Rust, the MSVC linker, and SDK together. It is not an application package or a Stage 3a acceptance test. Existing shells needed the Cargo binary directory added to their session PATH; no reboot was needed.

## Checkpoint B - scaffold package and native launch passed

The selected stack, confirmed against `package-lock.json`, includes React/React DOM `19.3.1`, TypeScript `6.0.3`, Vite `8.3.4`, Tauri JavaScript API `2.12.4`, Tauri CLI `2.12.3`, Vitest `5.0.5`, jsdom `30.1.3`, Testing Library React `16.3.3`, and jest-dom `7.0.1`. The Rust manifest pins Tauri `2.12.4` and tauri-build `2.7.3`.

| Command/check | Executed result |
| --- | --- |
| Initial npm dependency installation, two attempts | Failed inside npm's `@npmcli/arborist` resolver with `TypeError: Cannot read properties of null (reading 'edgesOut')` |
| npm installation after selecting the compatible pinned test stack above | Passed; 107 packages installed; audit reported 0 vulnerabilities |
| `npm.cmd install --package-lock-only --ignore-scripts --offline` | Passed after the version-pin update; 143 packages audited, 0 vulnerabilities reported |
| `npm.cmd ci --offline --cache .npm` | Passed from the populated repository-local cache; 107 packages installed, 0 vulnerabilities reported |
| `npm.cmd test` | Passed: four scaffold tests across the Work screen and typed desktop client |
| Initial `npm.cmd run typecheck` | Failed with TS6310 because a project reference targeted a configuration with `noEmit` |
| `npm.cmd run typecheck`, after removing the unnecessary project reference | Passed; application and Vite configuration checked separately |
| `npm.cmd run typecheck` and `npm.cmd test`, repeated after the clean lockfile installation | Both passed; all four scaffold tests passed again |
| `npm.cmd run build` | Passed; Vite `8.3.4` transformed 19 modules and produced the frontend bundle |
| `cargo fmt --manifest-path src-tauri/Cargo.toml -- --check` | Passed after formatting the status struct literal |
| Initial native build invocation, `tauri build --no-bundle --locked` | Rejected by CLI argument parsing; `--locked` must be forwarded to Cargo after `--` |
| Corrected `npm.cmd run tauri -- build -- --locked` | Passed, exit 0; release compilation completed in 6m49s and produced the NSIS package |
| Packaging-tool acquisition | Approved NSIS `3.13` and `nsis_tauri_utils` `0.5.3` downloads passed hash validation |
| Generated package | `src-tauri/target/release/bundle/nsis/Ascension_0.1.0_x64-setup.exe`, approximately 1.34 MiB |
| Silent NSIS installation with `/S /D=<repository>\.smoke\installed` | Passed, exit 0; installed `ascension.exe` was 4,262,912 bytes; the destination placeholder denotes the ignored repository-local smoke directory |
| Native launch of the installed executable | Passed through computer-use verification: the actual `Ascension — Work` window at `http://tauri.localhost` rendered `Desktop connection ready` and `Ascension · Record schema 1` from Rust |
| Installed-window visual inspection and close | Screenshot of the activated native window showed a clean, readable Work shell and successful connection status; `Alt+F4` closed it and subsequent native window inventory was empty |

Those four historical scaffold tests covered confirmed native status, visible connection failure with keyboard retry, refusal to report a native connection in browser-only use, and invoking only the narrow status command. They did not test persistence, import/export, or the full Work workflow; the later suites below replace and extend that coverage. The reported dependency audit is the result of that execution, not a guarantee of future vulnerability status.

The user also approved official Tauri/NSIS packaging-tool downloads. The installed package's native window and Rust command response were actually verified. This completes the scaffold packaging feasibility checkpoint; it does not yet test application data, save/restart durability, import/export, or operation with networking unavailable.

## Checkpoint C - persistence and command tests passed

Implemented the private Rust/rusqlite repository, schema migrations, canonical UUIDs and timestamps, checked revisions, persisted selection/generation, typed mutation commands, and native-file export/preview/restore. The dependency pins include rusqlite `0.40.2` with bundled SQLite and backup support, uuid `1.28.0`, chrono `0.4.45`, rfd `0.17.2`, and tempfile `3.27.0`.

| Command/check | Executed result |
| --- | --- |
| Initial Rust compile against pinned rusqlite | Failed because the selected API uses `MAIN_DB` rather than `DatabaseName::Main`; updated to the pinned API |
| `cargo test --manifest-path src-tauri/Cargo.toml --locked` | Passed: 22 tests, consisting of 20 real temporary-SQLite integration tests and two file-command tests |
| `cargo fmt --manifest-path src-tauri/Cargo.toml -- --check` | Passed |
| `node .toolchain/validate-fixtures.cjs` | Passed for `contracts/fixtures/sample-v1.json` (eight synthetic records) and `empty-v1.json` (zero records); checked exact shapes, canonical IDs/timestamps, revisions, limits, selection, and relationships |

The integration tests cover initialization and restart; persisted selection; stable identities and changed-only revisions; note clearing/recreation; cross-connection stale writes; Unicode-scalar validation; exact fixture round trips; safe replay with reordered arrays; empty restore and backup recovery; malformed/unsupported imports; preview lifetime and retained validated contents; backup and transaction failure; revision overflow; clock rollback; required nullable inputs; maximum record count; and preservation of exportability near the 10 MiB limit. A workspace whose export is exactly 10 MiB minus 16 bytes remains reimportable, and a growth mutation exceeding the limit is rejected without changing that workspace.

Migration tests use populated version-1 databases and check preserved records, readable backups, rollback after migration or validation failure, rejected future/unknown/corrupt schemas, and committed data still in a live WAL. The focused WAL test initially exposed recovery backups retaining WAL mode and leaving sidecars. The backup destination now switches to DELETE journal mode before integrity/version verification and synchronization, producing a standalone SQLite snapshot. The final test confirms the latest committed note and revision appear in both the migrated database and the valid version-1 backup. Source records remain protected by the migration write lock.

Read-only review also led to explicit hardening: schema/version checks are repeated under the migration lock; invalid legacy records roll back the upgrade; a new preview invalidates the old token before native selection or file reading, including cancellation/read failure; SQLite command work runs in blocking tasks; and a post-replacement export synchronization failure is reported honestly as written but not confirmed durable. The file-command tests verify selected-file replacement, application-data protection, controlled file errors, and bounded import reading. These tests do not automate native dialog UI interaction.

## Checkpoint D - Work flow and UI tests passed

Implemented the saved Work view, project/objective editing, explicit criterion, one action/stop condition and optional note per objective, persisted objective selection, committed-state-only save feedback, retained failed drafts, keyboard controls, guarded draft discard, native-file import preview/counts, and explicit replacement. The saved action, stop condition, and note appear first; objective/criterion details are expandable. The obsolete scaffold status command/client was removed.

| Command | Executed result |
| --- | --- |
| Initial completed `npm.cmd test` suite | Passed: 20 tests, consisting of 14 Work UI tests and six typed-client tests |
| `npm.cmd test` after native error-visibility fixes | Passed: 22 tests, consisting of 16 Work UI tests and six typed-client tests |
| `npm.cmd run typecheck` | Passed |
| `npm.cmd run build` | Passed |

UI/client coverage includes immediate display of saved records, project/objective relationships, exact update identities, whitespace and Unicode-scalar validation, preserved drafts and saved content on failure, delayed commit acknowledgment, duplicate-submit prevention, authoritative selection, conflict recovery, preview counts/cancellation, guarded explicit restore, failed-restore retention, and keyboard connection/modal handling. Modal transitions and editor close/save now restore usable focus. The two additional tests cover feedback corrections found during native testing. These tests use a mocked command boundary; real backend behavior is covered separately by the SQLite suite and the native results below.

## Checkpoint E - packaged native acceptance passed

The isolated test configuration changes only product name (`Ascension Smoke`), application identifier (`io.ascension.work.smoke`), and Tauri's application-directory override to an ignored repository-local synthetic data directory. It does not introduce a different persistence implementation or test-only command API. The normal-configuration installer was verified separately as recorded below.

The complete create/restart/restore/replay/rejection flow used source commit `e7d0630`. The final feedback, restart, and export-preservation rechecks used `4b516a8`, as did the normal installer. Both builds report application version `0.1.0`; the full flow was not repeated after the feedback-only correction.

| Build/install check | Executed result |
| --- | --- |
| First complete `npm.cmd run tauri -- build --config .smoke/native-test.conf.json -- --locked` | Passed, exit 0; release compilation took 5m38s; the configuration is a local ignored file |
| Generated smoke package | `src-tauri/target/release/bundle/nsis/Ascension Smoke_0.1.0_x64-setup.exe`, approximately 2.15 MiB |
| Smoke package installation | Passed, installer exit 0; installed `ascension.exe` was 6,313,984 bytes |
| Intermediate rebuild with inline validation feedback | Passed in 3m19s, but this intermediate build was not installed or natively tested |
| Rebuild including both final feedback corrections | Passed, exit 0; release build took 3m14s and produced an approximately 2.15 MiB installer |
| Final smoke package update | Passed, silent installer exit 0; the updated installed executable was used for the native rechecks below |
| Final normal `npm.cmd run tauri -- build -- --locked` | Passed, exit 0; release build took 3m14s |
| Final normal installer | `Ascension_0.1.0_x64-setup.exe`, 2,253,514 bytes (approximately 2.15 MiB); Authenticode status `NotSigned` |
| Final normal installation | Passed, installer exit 0, into ignored `.smoke/installed`; installed `ascension.exe` was 6,313,984 bytes |
| Final normal native check, without the proxy | Passed: the actual Work window rendered an empty workspace loaded successfully from Rust; screenshot inspection found it readable; `Alt+F4` closed it and the process was confirmed gone |

The final normal installer's SHA256 was independently read from the generated artifact and matched `9DF06108F3525593F976647606A23068DB9A828D70069AD7AEC1A3153DA174FB`.

### Offline-test attempts and evidence

The first attempt launched the installed smoke executable through the restricted execution sandbox. An HTTPS probe failed with `EACCES`, but the launched process could not be discovered by the native computer-use helper. Only a WebView profile appeared; no SQLite database initialized. The test processes were stopped. A separate desktop was suspected, not established. This attempt does **not** count as a successful application or offline workflow test.

The successful fallback launched the installed executable with this process-only environment value:

```text
WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS=--proxy-server=http://127.0.0.1:9 --proxy-bypass-list=tauri.localhost;ipc.localhost
```

Immediately before launch and reopen, the following probe failed with curl exit 7 because the loopback proxy endpoint refused the connection:

```powershell
curl.exe --proxy http://127.0.0.1:9 --connect-timeout 5 https://registry.npmjs.org/react
```

CIM inspection of the installed application's direct WebView2 child confirmed `proxy_present=true` and `local_bypass_present=true`. The refused-proxy probe and both child-process flag checks were repeated successfully for the final installed smoke rebuild. Raw process command lines and machine paths were not committed. The local Tauri and IPC hosts were bypassed so native assets/commands continued to work. No OS network, firewall, or adapter settings changed.

These results establish a **WebView2 proxy-blocked offline simulation** for the tested installed smoke package. They do not establish an OS-wide network block, physical network disconnection, or blocked Rust-process networking. The isolated package's full-flow results are distinct from the final normal-configuration installer's successful installation, native launch/workspace load, and close check. The full workflow was not repeated against the normal data directory.

### Native workflow results

| Scenario | Executed result |
| --- | --- |
| Create saved work | Passed through the native UI: project, bounded objective, criterion, next action, stop condition, and note |
| Keyboard/save feedback | Tab/Return and visible focus worked; the waiting state appeared before successful save confirmation |
| Close and reopen | `Alt+F4` closed the app and its process disappeared; reopening with the same proxy setup immediately showed the saved action and note |
| Restart identity preservation | The first export and the export after reopen were byte-for-byte identical, including IDs, revisions, timestamps, relationships, and selection |
| Restore empty workspace | Separate count preview and explicit replacement worked; the pre-restore backup was byte-for-byte identical to the original export |
| Restore original into empty state | Passed through preview and explicit confirmation; original records were recovered |
| Replay original export | The UI reported that the export already matched saved records; no duplicates or extra backup were created |
| Invalid imports | Malformed JSON, future schema version 99, duplicate IDs, and a dangling project reference were rejected without changing saved records |
| Final state/backup check | All four exported checkpoints were byte-for-byte identical: original, reopened, after restore/replay and four rejected imports, and after the final installed smoke rebuild. Exactly two backups existed, corresponding to the two changed restores, with none added by replay or rejection |

Native testing initially found two usability failures: a criterion-validation error was outside the visible viewport, and an import-error banner was outside the visible viewport. The frontend now supplies field-adjacent validation feedback and scrolls applicable global errors into view. The updated 22-test UI suite passed, followed by these checks against the final installed smoke rebuild:

- Reopening immediately showed the original saved action and note.
- Clearing the criterion in the objective editor and selecting Save displayed the red validation text beside the focused criterion field; screenshot inspection confirmed it was visible.
- Canceling that edit displayed the dirty-draft guard. Explicitly discarding the draft left the saved records unchanged.
- Importing malformed JSON from the Records footer automatically scrolled the error banner fully into view; native screenshot inspection confirmed visibility.
- The final native export matched all earlier checkpoints byte-for-byte, and the backup count remained exactly two.

Both feedback fixes passed native verification. The subsequent normal-configuration package build, installation, native workspace load, screenshot inspection, and close check also passed. No known Stage 3a functional acceptance failure remains in the tested environment with the test conditions recorded here.

All native data and imports were synthetic. Machine-specific configuration, runtime databases, exports, screenshots with local paths, and raw diagnostic logs remain outside version control.

After verification, the isolated smoke installation's own `.smoke/offline-installed/uninstall.exe /S` exited 0. The normal installed app, final normal installer, and synthetic verification exports were retained; no application windows remained open.

## Packaged Windows smoke checklist

Record the actual package filename, tested application version, commands, and outcomes when each check is executed. Use synthetic data only; keep database files, exports, screenshots containing local paths, and installer logs outside version control.

- [x] Install and launch the complete isolated Windows smoke package; installer exit 0 and native workflow results are recorded above.
- [x] Verify the Work flow under the documented, verified WebView2 proxy-blocked simulation; no model keys or online service configuration are required.
- [x] Using keyboard controls, create a project, bounded parent objective, criterion, next action, stop condition, and note; verify visible focus and save feedback.
- [x] Close completely, reopen under the same simulation, and verify immediate recovery with unchanged exported IDs and revisions.
- [x] Export, preview, restore into an empty state, and verify equivalent records, relationships, and identity.
- [x] Replay the import and verify no duplicates or new backup.
- [x] Reject malformed JSON, duplicate IDs, invalid references, and an unsupported future schema while preserving valid local records.
- [x] Record the completed temporary-SQLite migration, backup, restart, and import tests; 22 Rust tests passed as detailed above.
- [x] Install the final smoke rebuild and natively verify both corrected error-visibility cases, preserved drafts/records, and unchanged exported contents.
- [x] Rebuild and install the normal-configuration package; native workspace loading, readable display, and close passed separately from the isolated full-flow results.

## Completed gates and scope boundary

- B: passed for the scaffold: UI/client tests, typecheck, frontend build, native release build, NSIS packaging, installation, and native launch.
- C: implemented and automated checks passed for private SQLite persistence, migrations, and typed backend commands.
- D: implemented; 22 UI/client tests passed, including the error-feedback corrections identified during native testing.
- E: passed: the installed isolated package completed the full workflow, and its final rebuild passed the feedback and record-preservation rechecks, under verified WebView2 proxy-blocked simulation. The normal-configuration package separately passed build, installation, native launch/workspace load, and close.

The native results above supplement the automated suites; completion is not based on browser-only evidence. Stage 3a work stops here. Any Stage 3b/4 expansion requires the repository owner's explicit approval.

## Known limitations

- Offline evidence uses the documented process-scoped WebView2 proxy simulation, with full-flow testing in an isolated package variant. It is not a physical network-disconnection or OS-wide network-denial test.
- The normal Windows installer is unsigned, uses a per-user NSIS installation, and expects an already installed WebView2 runtime. No signing or public release was performed.
- Import supports explicit whole-workspace replacement. It does not merge conflicting records.
- Recovery backups remain local and are not automatically pruned; repeated changed restores can accumulate backup files.
- Tests use synthetic data on the recorded Windows environment. These checks do not establish product effectiveness, performance claims, or support for untested platforms.
