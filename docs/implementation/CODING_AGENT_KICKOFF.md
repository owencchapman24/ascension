# Copy/paste: Stage 3a coding-agent kickoff

You are working in a freshly created public GitHub repository called **ascension**. This is the beginning of application development, not another research phase. Build only the Stage 3a Windows offline vertical slice.

Read the repository's `AGENTS.md`, `docs/specs/STAGE_3A.md`, `docs/specs/ARCHITECTURE_BOUNDARIES.md`, and `docs/decisions/ADR-0001-initial-scope.md` before acting. Treat them as implementation constraints. Do not read, clone, run, or modify other repositories, private documents or user files. Only synthetic project data is allowed in tests or commits.

**First: perform a read-only inventory.** Report `pwd`, `git status`, contents of the new repository, and installed versions of Node/npm, Git, Rust/Cargo/MSVC, the Windows C++ build tools and WebView2 where checkable. If prerequisites are missing, propose the minimal installation steps and request approval before machine-wide changes. Do not claim native builds succeed until they actually run.

**Then implement in tested slices:**
1. Create the standard Tauri 2 + React/TS/Vite scaffold in this repo, with appropriate `.gitignore` additions and pinned lockfiles. Ensure a minimal packaged Windows app can launch. Prefer a small 4–8h packaging feasibility spike before heavy domain work. If Tauri is blocked, report evidence and propose the smallest fallback; do not implement both Tauri and Electron.
2. Implement a private SQLite Rust repository, schema migrations and narrow typed commands for Project, bounded ParentObjective/Criterion, NextAction, optional ResumptionNote and their read/update operations.
3. Build one simple Work-first screen. Show the saved next action immediately; no login, game, AI, forced onboarding, tracking or network needed.
4. Add versioned export and safe import with stable IDs, preview/explicit replacement-or-merge choice, referential checks, and controlled errors. Ensure corrupt/unknown-schema input does not destroy data.
5. Write and run tests against actual temporary SQLite databases and UI workflows. Perform a native packaged Windows smoke test of save/restart/export/import.
6. Record exact commands/results. Update README with local build/test instructions. Make incremental commits; inspect staged files for private data or secrets. Don't add CI workflows without an explicit, reviewed decision.

**Stop when Stage 3a meets its written acceptance criteria.** Report pass/fail status, outstanding risks, and the next smallest proposed milestone. Do not begin AI integrations, the game, reward settlement, experimental randomization, or external connectors. No fake test successes. Avoid unrelated repository access and machine-wide installs without approval.
