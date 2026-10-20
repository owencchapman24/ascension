# Ascension — agent operating rules

This is a **public** repository for a Windows-first, local-first productivity app. Follow this file for all work in this repository. Read `docs/specs/STAGE_3A.md`, `docs/specs/ARCHITECTURE_BOUNDARIES.md`, and `docs/decisions/ADR-0001-initial-scope.md` before implementation.

## Scope and safety

- Work **only in this repository**. Do not inspect, modify, clone, or run other personal repositories or projects. Toolchain inspection may read standard system installation metadata; ask before any machine-wide installation.
- This repo is public. Never commit personal productivity records, private research archives, personal/medical/employment details, employer materials, credentials, tokens, `.env` files, real database files, private documents, user-specific paths, or secrets in diagnostics. Use synthetic fixtures. Inspect `git diff --cached` before each commit.
- Do not import the original private Phase 1–5 research package into the public repository. These public implementation notes intentionally omit personal context.
- Do not access APIs, cloud accounts, external project directories, connectors, or files other than normal public dependency registries without explicit approval.
- Do not add Actions workflows until tests exist and the workflow is deliberately reviewed for permissions, secrets, trigger scope, and artifact retention. Public runners still have concurrency, storage and security considerations.

## Current goal — Stage 3b (owner authorized)

The owner's Stage 3b implementation assignment authorizes progress evidence, prospective criterion locking/versioning, local user acceptance, deterministic `bc-v1` issuance, corrections, and complete portability. Read `docs/specs/STAGE_3B.md` in addition to the Stage 3a baseline documents. Preserve Stage 3a behavior and data. Stage 3c/4 remain excluded. Work on `feat/stage-3b-evidence-ledger`; local verified commits are authorized, but pushing, merging, publishing, and releasing are not.

The following Stage 3a workflow remains a regression requirement:

Build a minimal **packaged Windows app** in which a person can:
1. Create a project and a bounded parent objective with an explicit criterion.
2. Save one concrete next action with a stop condition and optional resumption note.
3. Close/reopen the app offline and immediately recover that action/note.
4. Export and reimport an open, versioned record set while preserving stable IDs/versions.
5. Reject malformed, corrupted, or unsupported-version imports without destroying valid local state.

Do **not** add AI, API calls, a game, credit spending, experiment assignment, broad project-file scanning, cloud sync, accounts, telemetry, payments, autonomous actions, or Stages 3c/4. Evidence authority is bounded to registered sources; no external test runner or fabricated trusted validator. The app must remain useful with fictional progression hidden.

## Technical decisions

- React + TypeScript + Vite frontend; Tauri 2 desktop shell; a **narrow typed Rust command layer**; SQLite using a private `rusqlite` repository. One local app, no backend server.
- The Rust backend owns persistence, identifiers, input validation and state transitions. The frontend must not receive arbitrary SQL, shell or filesystem access.
- Record schema versions and stable IDs (UUIDs); preserve data during migrations, import, export and restart. Do not assume UI state is durable until the backend confirms a write.
- Run local tests first: UI tests, Rust tests against temporary real SQLite databases, export/import cases, plus a packaged Windows smoke test. Do not mark Stage 3a done on browser-only results.
- Prefer accessible semantic controls, keyboard operation, visible errors, and immediate access to the saved next action. No compulsory onboarding questionnaire or timer.

## Working method

Inspect `git status` and the environment first. Report missing Node/npm, Rust MSVC, Windows C++ Build Tools or WebView2 before requesting changes. Use pinned compatible dependencies and commit lockfiles. Make small, tested, reviewable commits; record test commands and real results. If a framework/toolchain change is necessary, propose the smallest alternative and request approval before switching. Maintain an architectural decision record for any departure from these constraints. **Stop at Stage 3b and request approval before expanding scope.**
