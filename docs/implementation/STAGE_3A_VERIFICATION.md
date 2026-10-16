# Stage 3a implementation verification

This log records executed checks, not assumed outcomes. All examples and test records are synthetic. Stage 3a remains incomplete until the packaged Windows acceptance flow passes.

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

The four tests cover confirmed native status, visible connection failure with keyboard retry, refusal to report a native connection in browser-only use, and invoking only the narrow status command. They do not yet test persistence, import/export, or the full Work workflow. The reported dependency audit is the result of that execution, not a guarantee of future vulnerability status.

The user also approved official Tauri/NSIS packaging-tool downloads. The installed package's native window and Rust command response were actually verified. This completes the scaffold packaging feasibility checkpoint; it does not yet test application data, save/restart durability, import/export, or operation with networking unavailable.

## Packaged Windows smoke checklist

Record the actual package filename, tested application version, commands, and outcomes when each check is executed. Use synthetic data only; keep database files, exports, screenshots containing local paths, and installer logs outside version control.

- [x] Install and launch the generated Windows scaffold package; the installer exited 0 and the native Work window received a successful Rust status response. Repeat this check for the completed Stage 3a implementation.
- [ ] Verify the Work view with networking unavailable and no configured keys.
- [ ] Using keyboard controls, create a project, bounded parent objective, criterion, next action, stop condition, and optional resumption note; verify visible focus and save/error feedback.
- [ ] Close the application completely, reopen offline, and verify immediate recovery with unchanged stable IDs and revisions.
- [ ] Export to an explicitly selected location, preview the export, import into a clean state, and verify equivalent records, relationships, IDs, and revisions.
- [ ] Replay the import and verify the documented policy produces no duplicates.
- [ ] Attempt malformed JSON, duplicate IDs, invalid references, and an unsupported future schema; verify controlled errors and unchanged valid local records.
- [ ] Record the completed temporary-SQLite migration, backup, restart, and import tests alongside the native smoke outcome.

## Remaining gates

- B: passed for the scaffold: UI/client tests, typecheck, frontend build, native release build, NSIS packaging, installation, and native launch.
- C: SQLite migrations and typed backend commands.
- D: accessible Work flow and saved resumption state.
- E: export/import, persistence and migration tests, and packaged offline acceptance.

Do not interpret any frontend-only verification as a passed native gate.
