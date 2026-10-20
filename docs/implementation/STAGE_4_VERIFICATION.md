# Stage 4 verification record

Status: implementation and verification in progress. No final PASS or merge recommendation yet. Engineering verification does not establish productivity benefit, human feasibility, commercial readiness, or authorization for Stage 5.

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

Executed Stage 4 test results, migration/portability evidence, native screenshots, performance, accessibility limits, defects, and final commit/status details will be added as their checks complete. Historical Stage 3a/3b/3c reports remain unchanged.
