# Stage 3c verification record

Status: implementation in progress. This document records executed engineering evidence and unresolved gates; it is not a declaration of completion or human behavioral efficacy.

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

## Verification gates

Backend, interface, performance, portability, native lifecycle, visual inspection, and public staged-diff review remain pending until executed results are added here. Historical Stage 3b results are not counted as Stage 3c verification.

All synthetic fixtures and installed test profiles remain disposable and local. No normal profile has been enrolled in E00/E01 by the implementation agent. No human observation or effect estimate has been fabricated.
