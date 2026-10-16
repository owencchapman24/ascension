# ADR-0001 — Start with a local, offline work slice

**Status:** Accepted provisionally for Stage 3a.

## Decision

Create a new, isolated, public-source Windows application with React/TypeScript/Vite, Tauri 2, a narrow Rust command layer and private SQLite storage. First build only parent/criterion/action/note persistence with export/import and native Windows verification. Use local synthetic fixtures; keep research and personal work records outside Git.

## Reason

The first useful capability is resuming a known meaningful action without re-planning, connectivity, or game/AI activity. A narrow vertical slice can validate Windows packaging and reliable records before more complex acceptance/reward/strategy systems.

## Alternatives

A mature existing task manager remains the baseline for behavioral experiments. A custom Super Productivity extension is possible later for display, but it is not selected for the initial standalone codebase. Electron is a contingency if the bounded Tauri feasibility spike exposes an unacceptable toolchain obstacle. No cloud platform, mobile sync or multiuser backend is justified now.

## Gate

Proceed to Stage 3b only when a **real packaged Windows application** reliably creates and restores a next action and note, round-trips structured exports with stable identities, rejects corrupt imports without data loss, and passes documented integration/smoke tests. Ask the repository owner before advancing scope.
