# Ascension — agent operating rules

This is a **public** repository for a Windows-first, local-first productivity app. Follow this file for all work in this repository. Read `docs/specs/STAGE_3A.md`, `docs/specs/ARCHITECTURE_BOUNDARIES.md`, and `docs/decisions/ADR-0001-initial-scope.md` before implementation.

## Scope and safety

- Work **only in this repository**. Do not inspect, modify, clone, or run other personal repositories or projects. Toolchain inspection may read standard system installation metadata; ask before any machine-wide installation.
- This repo is public. Never commit personal productivity records, private research archives, personal/medical/employment details, employer materials, credentials, tokens, `.env` files, real database files, private documents, user-specific paths, or secrets in diagnostics. Use synthetic fixtures. Inspect `git diff --cached` before each commit.
- Do not import the original private Phase 1–5 research package into the public repository. These public implementation notes intentionally omit personal context.
- Do not access APIs, cloud accounts, external project directories, connectors, or files other than normal public dependency registries without explicit approval.
- Do not add Actions workflows until tests exist and the workflow is deliberately reviewed for permissions, secrets, trigger scope, and artifact retention. Public runners still have concurrency, storage and security considerations.

## Current goal — GitHub Actions CI (owner authorized)

The owner authorizes a bounded CI milestone on `chore/github-actions-ci`, based on merged Stage 4.5 `4a6c32d0a84e61d5350baccc1e2640239ab409f7`. Add one reviewed, read-only `pull_request` workflow for the existing frontend and Windows Rust checks, with verified full-SHA action pins, pinned toolchains, bounded timeouts, and cancellation of superseded PR runs. Public upstream action/runner documentation and verification-tool downloads are authorized for this milestone. Use no secrets, write permissions, privileged triggers, self-hosted runners, caches, artifact uploads, installer builds, or installed-app automation. See `docs/implementation/CI_SETUP.md` for coverage, validation, and maintenance.

Verified local commits are authorized; pushing, creating or merging PRs, publishing, releases, deployment, new integrations, machine-wide installation, and changing the normal app/profile are not. Preserve application behavior, tests, dependency contracts, historical verification reports, and main. This authorization supersedes only the prior branch and CI prohibition for this milestone. Stage 4.6A, Stage 5, and other product scope remain excluded; stop after local delivery. Hosted checks must be observed after the owner pushes and creates a PR before any merge recommendation.

## Historical Stage 4.5 authorization

The owner's Stage 4.5 assignment authorizes daily-use Work improvements: objective lifecycle and explicit priority/focus, append-only factual checkpoints with atomic saved-action updates and replay protection, voluntary initiation support, simpler ordinary opportunities, coherent automatic local snapshots, recovery, additive database v6 and complete interchange v5. Read `docs/specs/STAGE_4_5.md` and ADR-0007 in addition to the baseline documents below. Work exclusively on `feat/stage-4-5-daily-use`, based on Stage 4 merge `c27c1d7`. Verified local commits and isolated installed-Windows QA are authorized; pushing, merging, publishing, releasing, deployment, Actions workflows, AI, Stage 5, real study activation, and modifying the normal installation/profile are not. Preserve all prior authority and economy invariants. The specific Stage 4.5 authorization supersedes the historical Stage 4 branch/stop instruction below.

## Historical Stage 4 authorization

The owner's Stage 4 assignment authorizes one River Reach strategy chapter, construction reservations/spending, reusable funded capacity, spatial placement, free reconfiguration/undo, three alternative routes, an immutable Chronicle ending, sandbox alternatives, and complete portability. Read `docs/specs/STAGE_3B.md`, `docs/specs/STAGE_3C.md`, and `docs/specs/STAGE_4.md` in addition to the Stage 3a baseline documents. Preserve Stage 3a/3b/3c behavior, data, evidence authority, `bc-v1` issuance invariants, and the fixed H1 instrument. Work on `feat/stage-4-river-reach`; local verified commits are authorized, but pushing, merging, publishing, deploying, and releasing are not. Do not activate real E00/E01 studies on behalf of the owner; synthetic isolated tests may exercise the engine. Stage 5 remains excluded.

The following Stage 3a workflow remains a regression requirement:

Build a minimal **packaged Windows app** in which a person can:
1. Create a project and a bounded parent objective with an explicit criterion.
2. Save one concrete next action with a stop condition and optional resumption note.
3. Close/reopen the app offline and immediately recover that action/note.
4. Export and reimport an open, versioned record set while preserving stable IDs/versions.
5. Reject malformed, corrupted, or unsupported-version imports without destroying valid local state.

Do **not** add AI, API calls, broad project-file scanning, cloud sync, accounts, telemetry, payments, autonomous actions, notifications, surveillance, or Stage 5. Strategic gameplay and spending are limited to the six-site, 24-BC River Reach chapter under `river-reach-v1`. Gameplay cannot issue BC or change accomplishment authority. Evidence authority is bounded to registered sources; no external test runner or fabricated trusted validator. Withhold H2 while E00/E01 is active. The app must remain useful with fictional progression hidden.

## Technical decisions

- React + TypeScript + Vite frontend; Tauri 2 desktop shell; a **narrow typed Rust command layer**; SQLite using a private `rusqlite` repository. One local app, no backend server.
- The Rust backend owns persistence, identifiers, input validation and state transitions. The frontend must not receive arbitrary SQL, shell or filesystem access.
- Record schema versions and stable IDs (UUIDs); preserve data during migrations, import, export and restart. Do not assume UI state is durable until the backend confirms a write.
- Run local tests first: UI tests, Rust tests against temporary real SQLite databases, export/import cases, plus a packaged Windows smoke test. Do not mark Stage 3a done on browser-only results.
- Prefer accessible semantic controls, keyboard operation, visible errors, and immediate access to the saved next action. No compulsory onboarding questionnaire or timer.

## Working method

Inspect `git status` and the environment first. Report missing Node/npm, Rust MSVC, Windows C++ Build Tools or WebView2 before requesting changes. Use pinned compatible dependencies and commit lockfiles. Make small, tested, reviewable commits; record test commands and real results. If a framework/toolchain change is necessary, propose the smallest alternative and request approval before switching. Maintain an architectural decision record for any departure from these constraints. **Stop at Stage 4.5 and request approval before expanding scope.**
