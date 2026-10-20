# Ascension record and command contract

## Stage 3b extension

The current database version is 3 and the complete interchange version is 2. The Stage 3a section below remains the legacy v1 input contract. Every current `Workspace` also has a required `progress` object. Its exact strict types and validation are in `src-tauri/src/progress.rs`; the renderer types are in `src/platform/client.ts`. The architecture and compatibility policy are recorded in ADR-0004.

Interchange v2 retains the v1 top-level work collections and selection, sets `schema_version: 2`, and requires `progress`. Its collections are `parents`, `slots`, `criteria`, `evidence`, `validators`, `snapshots`, `acceptances`, `corrections`, `grants`, `ledger`, `audit`, and `commands`, plus `registry_revision` and a checked `wallet_bc` projection. All are required. No local workspace generation is added at envelope level; historical acceptance command payloads retain their original expected generation for exact idempotent replay. Objective criterion text is a checked projection of the latest primary-lineage criterion, never a second mutable stored definition.

Each correction carries its monotonic `registry_revision`; subsequent acceptance requires a review at or after that correction. Accepted snapshots and acceptance records must correspond in both directions. Each ledger entry has a contiguous issuance `sequence`; committed commands retain `ledger_sequence` so their original balances can be validated against the exact historical prefix. These are causal and accounting identities, independent of wall-clock ordering.

| New command | Result after durable commit |
| --- | --- |
| `save_criterion` | Complete `Workspace`; edit draft or create a same-lineage prospective version |
| `lock_criterion` | Complete `Workspace`; freeze version and positive integer slot allocation |
| `record_evidence` | Complete `Workspace`; append honestly labeled evidence or explicit supersession |
| `review_progress` | `{ workspace, snapshot }`; complete registered applicable evidence and missing/conflict status |
| `accept_progress` | `{ workspace, result }`; result contains `acceptance_id`, `grant_id`, `issued_bc`, original `balance_bc`, and `retained_reward` |
| `correct_acceptance` | Complete `Workspace`; append incorrect-acceptance or historical-reopened correction |
| `reject_review` | Complete `Workspace`; reject pending review with reason, no issuance |

All use one `input` argument and reject unknown fields. Definitions include exact UUIDs, criterion lineage/version, expected generation, factual summary and assistance where applicable. Acceptance additionally requires command schema version 1, UUID command ID, expected parent revision, snapshot ID/digest, and evidence IDs. It never accepts amount, actor, trust privilege, or policy override. The backend canonicalizes reference order; exact committed replay precedes stale checks. Retry the same logical acceptance with the same command ID and payload. An idempotent result's original balance can be older than the accompanying current workspace; the UI displays the latter's derived wallet.

New controlled error codes include `busy`, `stale_review`, `evidence_ineligible`, `idempotency_conflict`, `unavailable_authority`, and `unsafe_legacy_import`. Required unavailable validators stay missing, never fabricated. No production trusted-validator registration command exists.

Import preview adds original incoming `schema_version`, a clear `warning`, and both current/incoming counts for criteria, evidence, snapshots, acceptances, corrections, grants, ledger, committed commands, and wallet BC. Legacy v1 cannot replace local earned history. Modern whole-state replacement is explicit and backed up. All work and progress collections count toward the shared 10,000-record and 10-MiB complete-export limits. Referenced snapshots contain frozen evidence copies for historical interpretation; their bytes count toward the export limit.

## Historical Stage 3a contract

This is the accepted frontend/backend implementation contract. All field names and command inputs use `snake_case`. Rust validates every input and owns durable state. The record model and replacement policy are explained in [ADR-0002](../docs/decisions/ADR-0002-stage-3a-records-and-restore.md).

## Records

The following TypeScript describes JSON transported over the narrow Tauri command API. `Uuid` is a canonical lowercase, hyphenated UUID string. `UtcTimestamp` is an RFC 3339 UTC timestamp ending in `Z`, with exactly millisecond precision, for example `2026-10-16T12:00:00.000Z`. `revision` is an integer from 1 through 4,294,967,295.

```ts
type Uuid = string;
type UtcTimestamp = string;

interface RecordMetadata {
  id: Uuid;
  revision: number;
  created_at: UtcTimestamp;
  updated_at: UtcTimestamp;
}

interface Project extends RecordMetadata {
  title: string;
}

interface Objective extends RecordMetadata {
  project_id: Uuid;
  title: string;
  criterion: string;
}

interface NextAction extends RecordMetadata {
  objective_id: Uuid;
  description: string;
  stop_condition: string;
}

interface ResumptionNote extends RecordMetadata {
  objective_id: Uuid;
  text: string;
}

interface Workspace {
  generation: Uuid;
  selected_objective_id: Uuid | null;
  projects: Project[];
  objectives: Objective[];
  actions: NextAction[];
  notes: ResumptionNote[];
}
```

All IDs across the four record collections must be globally unique. Every objective references an existing project; every action/note references an existing objective. There is at most one action and at most one note for each objective. A note may exist independently of an action. Selection is null or references an existing objective. The criterion is an explicit conceptual record embedded in its objective and shares its identity and metadata.

Rust assigns IDs and timestamps on creation. Creation starts at revision 1 with equal creation/update times. Changed record fields preserve ID and creation time, update the timestamp, and increment the revision with checked arithmetic. Unchanged fields preserve the complete record. Timestamp ordering is not used for conflict resolution or import authority; local wall clocks can change. UUID relations cannot be changed by update commands.

## Text and resource limits

Lengths count Unicode scalar values, not UTF-8 bytes or UTF-16 code units. Preserve accepted text exactly; do not silently truncate or rewrite user-authored criteria/actions. Required text must contain a non-whitespace character and must not contain U+0000.

| Field | Maximum length |
| --- | ---: |
| Project title | 120 |
| Objective title | 500 |
| Objective criterion | 2,000 |
| Action description | 2,000 |
| Action stop condition | 2,000 |
| Note text | 8,000 |

The workspace/import permits at most 10,000 records total across all four collections. Imports permit at most 10 MiB (10,485,760 bytes), enforced while reading as well as against initial file metadata. JSON is UTF-8. Unknown object fields, missing fields, invalid primitive types and duplicate object keys are rejected. An absent optional note is represented by `null` in a save input and by no note record in an export. A whitespace-only note supplied to `save_work` is treated as clearing the note; stored/imported note records must contain non-whitespace text. The backend rejects any mutation whose export would exceed the import file-size limit, so every accepted workspace remains exportable and reimportable.

## Commands

Each input-bearing command uses one Tauri argument named `input`:

```ts
invoke<Workspace>("create_project", {
  input: { expected_generation: workspace.generation, title: "Sample project" },
});
```

Commands without input use `invoke<Result>("command_name")`. Mutations return a complete authoritative `Workspace` only after commit. The frontend must retain unsaved input on failure and must not announce durability before success. `expected_generation` is a required optimistic-concurrency check; stale state returns `conflict` with no change. No-op mutations preserve generation and revisions.

| Command | `input` shape | Result |
| --- | --- | --- |
| `load_workspace` | No input | `Workspace` |
| `create_project` | `{ expected_generation, title }` | `Workspace` |
| `update_project` | `{ expected_generation, id, title }` | `Workspace` |
| `create_objective` | `{ expected_generation, project_id, title, criterion }` | `Workspace` |
| `update_objective` | `{ expected_generation, id, title, criterion }` | `Workspace` |
| `select_objective` | `{ expected_generation, id: Uuid \| null }` | `Workspace` |
| `save_work` | `{ expected_generation, objective_id, description, stop_condition, note: string \| null }` | `Workspace` |
| `export_workspace` | No input | `ExportResult \| null` |
| `preview_import` | No input | `ImportPreview \| null` |
| `restore_import` | `{ token: Uuid, expected_generation: Uuid }` | `RestoreResult` |

All IDs and `expected_generation` values in the table are UUID strings. `create_objective` selects the new objective in the same transaction. `select_objective` persists the user's selection. `save_work` atomically creates/updates the objective's one action and optional note; editing preserves their existing IDs. Clearing a note removes it; a subsequent new note receives a new ID. There are no project/objective delete or completion commands.

```ts
interface RecordCounts {
  projects: number;
  objectives: number;
  actions: number;
  notes: number;
}

interface ExportResult {
  file_name: string;
}

interface ImportPreview {
  token: Uuid;
  generation: Uuid;
  file_name: string;
  current_counts: RecordCounts;
  incoming_counts: RecordCounts;
}

interface RestoreResult {
  workspace: Workspace;
  backup_created: boolean;
  unchanged: boolean;
}

interface CommandError {
  code:
    | "invalid_input"
    | "conflict"
    | "invalid_import"
    | "unsupported_schema"
    | "storage_error"
    | "backup_failed"
    | "file_error";
  message: string;
}
```

Errors are rejected command results with the `CommandError` shape. Messages describe a field/problem safely without raw input contents, SQL or absolute paths. Filename results contain only the selected basename. Canceling a native file dialog returns `null`. The UI treats it as cancellation.

`export_workspace` exports the committed saved state through a Rust-owned native save dialog. `preview_import` opens a Rust-owned native open dialog, validates the whole chosen export, and retains its parsed contents in backend memory. A new preview supersedes the prior token. `generation` captures the workspace shown as the replacement target. The frontend displays both count sets and explicitly warns that restoration replaces local records; the separate confirmation button says **Replace local records**.

`restore_import` accepts the preview token and generation, not paths or replacement records. If the workspace has changed, preview again. Confirmation creates a durable pre-restore JSON backup before an atomic replacement, and preserves every imported record field. Successful restore consumes the token. If incoming normalized records/selection equal local state, return `{ workspace, backup_created: false, unchanged: true }`; otherwise return `{ workspace, backup_created: true, unchanged: false }`. Failures preserve the existing valid workspace.

## Interchange schema version 1

```ts
interface RecordSet {
  format: "ascension-records";
  schema_version: 1;
  selected_objective_id: Uuid | null;
  projects: Project[];
  objectives: Objective[];
  actions: NextAction[];
  notes: ResumptionNote[];
}
```

All properties are required, including empty arrays and null selection. Every record uses the exact shape above. No local workspace generation, database schema version, app path, source filename or import token appears in this envelope. Export arrays are ordered by ID. Import accepts any array order; equivalence compares complete records by identity plus selection, not input array order or JSON whitespace. IDs, revisions and timestamps are never regenerated during restore. Unsupported schema versions fail before mutation.

A minimal valid empty record set is:

```json
{
  "format": "ascension-records",
  "schema_version": 1,
  "selected_objective_id": null,
  "projects": [],
  "objectives": [],
  "actions": [],
  "notes": []
}
```

Only synthetic fixtures belong in this public repository. Application databases, exports containing real work and runtime backups do not.
