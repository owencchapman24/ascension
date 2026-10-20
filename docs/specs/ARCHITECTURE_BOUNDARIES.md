# Ascension architecture boundaries — Stage 3a

Stage 4 authorization: the owner approved [STAGE_4.md](STAGE_4.md) after merged Stage 3c `7b522be`. A separate Rust world module may reserve and spend existing fictional credit for one six-site River Reach chapter. Stage 3b retains sole issuance and evidence authority. H1 and frozen study protocols remain unchanged; H2 is withheld during active E00/E01. Additive database v5 and complete interchange v4 preserve all earlier records. No AI, remote services, Stage 5, or real study activation by the agent. See [ADR-0006](../decisions/ADR-0006-river-reach-economy.md). Earlier stage exclusions below are historical boundaries extended only by their subsequent explicit authorizations.

Stage 3c authorization: after merged Stage 3b, the owner approved [STAGE_3C.md](STAGE_3C.md). Additive opportunity/study persistence and a fixed presentation-only H1 scene extend the historical exclusions below. Stage 3b alone still owns reward authority; no opportunity or scene operation can mint or spend. Stage 4 and real E01 activation by the implementation agent remain excluded. See ADR-0005.

Stage 3b authorization: the owner approved the extension in [STAGE_3B.md](STAGE_3B.md) after PR #1 merged. The Stage 3a separation below remains the baseline. Stage 3b adds private Rust criterion/evidence/acceptance/issuance transactions and complete interchange v2; ordinary mutations must use targeted writes. Later game, AI, scanning, experiments, and spending remain excluded. The historical "do not implement" section below describes the original Stage 3a boundary, not a prohibition on the now-authorized Stage 3b subset.

## Initial implementation approach

A small local desktop application, **not** microservices. UI: React/TypeScript/Vite. Shell: Tauri 2. Domain and persistence: Rust with typed Tauri commands and `rusqlite` backed by SQLite. The data layer must be private to Rust; no generic SQL plugin exposed to the renderer. The implementation agent must verify installed Windows prerequisites and compatible versions before scaffolding.

```text
User → React Work view → Typed commands → Rust domain / validation
                                          ↓
                                  SQLite repository
                                          ↓
                             Versioned export / import
```

## Initial code layout (illustrative, not a scaffold)

```text
src/work/                      Work-first UI
src/platform/                  Narrow typed Tauri command client
src-tauri/src/commands/        Whitelisted, typed commands
src-tauri/src/domain/          Domain models and validation
src-tauri/src/storage/         SQLite repository and migrations
src-tauri/migrations/          Versioned migration files
contracts/                     Exchange schemas and synthetic fixtures
src-tauri/tests/               Real temporary SQLite integration tests
src/**/__tests__/              UI tests where appropriate
```

No service process, remote account, vector index, model adapter, game engine, generalized plugin system or two-way task synchronization.

## Authority and reliability

- The human selects goals, meaning and stopping criteria. A task click has no completion authority.
- Backend commands validate identifiers, relations, input shape, and permitted state transitions.
- Every mutation is committed durably before reporting a successful save; failures must be visible and recoverable.
- Export is structured and versioned; import parses/validates into a staging representation before changing current data.
- Choose explicit policy for restoring an export vs. merging; do not silently overwrite local records or regenerate IDs. Tests must exercise each supported behavior.
- Failed or unknown-version imports must leave existing records intact.
- Backups and migration behavior must be explicitly tested before storage is relied on for real use.
- Repository tests and examples must use synthetic data; real private content never enters test fixtures.

## Future separation — do not implement in Stage 3a

Later modules may include opportunities/experiment logging, evidence and user acceptance, deterministic reward settlement, optional H1 construction, AI suggestions, and H2 strategic game state. Their interfaces should remain conceptually separable, but **do not prematurely build their schemas, services, mocks or empty frameworks**. The application should support future extension through stable domain boundaries and migrations, not speculative infrastructure.

The longer internal Phase 5 specification calls for typed backend transactions and separately controlled credit/evidence authority in later stages. Stage 3a must **not** implement reward rules yet.
