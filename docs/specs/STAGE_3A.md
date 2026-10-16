# Stage 3a — first offline vertical slice

Status: initial implementation specification. The repository starts without an application. This document is a public, privacy-safe derivation of a longer internal planning package. It does not establish product effectiveness.

## Purpose

Reduce friction at the moment someone wants to work: immediately show the chosen objective, one executable next action, its observable stopping point, and any saved resumption note. Leave real work uninterrupted. There is **no AI or game** in this stage.

## Required workflow

1. Create a **Project** (long-lived collection of related work).
2. Create a **Parent objective** (a bounded meaningful outcome) linked to that project.
3. Specify an explicit **Criterion** explaining how the outcome would be recognized. Stage 3a records criteria but does not implement evidence acceptance or credit settlement.
4. Save a concrete **Next action** containing a verb/target and observable stop point, linked to that parent.
5. Optionally save a brief **Resumption note** (last result, open question, location/source text supplied by the user). Source-file scanning is out of scope.
6. Close the application. Reopen **offline**, directly into a Work view where the saved next action is available without onboarding, generating advice, or opening a game screen.
7. Export versioned records to a user-selected safe location; preview/import a valid export while preserving IDs, references and revisions. Import must not mint accomplishments or modify reward state (neither exists yet).

## Domain basics

At minimum: Project, ParentObjective, Criterion, NextAction, ResumptionNote and metadata for export/versioning. Use stable UUIDs and an explicit `schema_version` in the interchange format; keep referential integrity in SQLite. These are conceptual records; tables may differ if invariants remain testable.

- A next-action click or timer does **not** demonstrate meaningful work or satisfy a criterion.
- User-defined goals and priority order cannot be silently changed by automation.
- Record `created_at` and `updated_at` or a suitable local revision scheme; do not use timestamps as universal evidence-authority rules.
- Basic data editing must never require an internet connection, login, model key, or game interaction.
- Do not attempt to infer productivity, mood, diagnosis, or cognitive capacity from use.

## Acceptance checklist (execute, do not assume)

- Packaged Windows app opens with networking unavailable and no keys configured.
- A user can create and view a project, bounded parent, criterion, action and optional note.
- The saved action/note survive application close and restart with stable IDs and revisions.
- A valid exported sample can be imported after a clean state and yields equivalent records, links and identity; replay is safe against duplicates.
- Importing invalid JSON, duplicate IDs, inconsistent references, or an unsupported future schema returns a controlled error **without overwriting good data**.
- Tests touch a real temporary SQLite database as well as UI behavior; migrations and restart persistence are tested.
- The critical flow works with keyboard controls, visible focus, readable errors and no forced timer.
- A real packaged Windows build receives a manual/native smoke test. Record exact environment, commands, outcomes, and unresolved limitations.

## Explicitly excluded

Progress acceptance, reward budget, Build Credits, game/World view, experiment randomization, AI calls, repository connectors, model-led verification, broader personal metrics, cloud synchronization, accounts, automatic ingestion, browser extension, commercialization, and performance claims. These later features have separate decision gates.
