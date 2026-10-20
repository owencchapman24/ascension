# Ascension record and command contract

## Stage 3c extension

The current database version is **4** and complete portable interchange version is **3**. Migration `004_study.sql` is additive; it does not change any Stage 3b reward tables or policy. Every current record set requires the complete `progress` and `study` objects. Exact strict Rust records and command inputs are defined in `src-tauri/src/study.rs`; renderer types are in `src/platform/client.ts`.

Interchange **v3 itself binds every workspace to `h1-ledger-64-v1`**, including workspaces with no study manifest: exactly the first `min(64, issued BC)` fixed authored units are restored. Thus the envelope's preserved schema version carries the H1 policy even when all study collections are empty. Changing this policy requires a new supported interchange version; an unsupported version is rejected. Manifest `presentation_policy` repeats this fixed contract for protocol review and must match it.

`study` contains the required arrays `epochs`, `manifests`, `protocol_events`, `opportunities`, `assignments`, `outcomes`, `exposures`, `burdens`, `reflections`, and `preferences`. All identities are canonical UUIDs and globally unique across Work, progress, and study records. All new rows are immutable; corrections and lifecycle decisions append rows. Ordinary commands perform targeted inserts under the existing immediate transaction and optimistic workspace generation check. Study commands cannot create or change reward grants, ledger entries, acceptance records, or reward slots.

The **IPC Workspace is a safe view**, distinct from the complete export: manifests omit the randomization `seed` and full `allocation`, and include a derived lifecycle `status`. Assignment `condition` is null until its prospective preview boundary. Opportunities include a derived `outcome_state`; no observation after an expired window projects as missing without inventing a report or a latency. The view adds `presentation: { opportunity_id, condition, boundary, world_allowed, enabled }`, where `boundary` is `none`, `preview`, or `checkpoint`. Reading this projection never records delivery or awareness. Complete exports preserve the exact seed, all allocations, raw rows, and original event identities. Do not round-trip a renderer Workspace as an import record set.

| Command | Required fields after `expected_generation` | Result |
| --- | --- | --- |
| `create_schedule_epoch` | `label`, `effective_from`, `timezone` | Workspace |
| `configure_study` | `phase: e00 \| e01`, `epoch_id` | Workspace; configured, never automatically active |
| `transition_study` | `manifest_id`, `status`, `reason`, `acknowledgements: string[]` | Workspace; append lifecycle decision |
| `plan_opportunity` | `objective_id`, `epoch_id`, `manifest_id`, `domain`, `meaningful_start`, `cue_at`, `timezone`, `window_seconds`, `eligible`, `unavailability`, `priority`, `available_minutes`, `deadline_at`, `context` | Workspace; snapshot saved action and criterion identity, lock eligibility, atomically assign if applicable |
| `record_opportunity_outcome` | `opportunity_id`, `report`, `started_at`, `precision`, `uncertainty_seconds`, `note`, `technical_failure`, `uncertainty_reason`, `correction_reason`, `priority_advanced`, `quality_note`, `acceptance_id` | Workspace; append observation or reasoned correction |
| `record_study_exposure` | `opportunity_id`, `kind`, `note` | Workspace; append delivered, seen, or deviation observation |
| `record_study_burden` | `opportunity_id`, `category`, `minutes`, `note` | Workspace; exclusive estimated minutes |
| `record_study_reflection` | `manifest_id`, `note` | Workspace; optional factual weekly reflection |
| `set_construction_enabled` | `enabled: boolean` | Workspace; persistent, versioned optional-view preference |

Nullable fields must be present explicitly as null. Unknown fields, duplicate JSON keys, invalid enums, noninteger counts, and invalid references are rejected. Study timestamps are canonical UTC milliseconds in years 1900–2200; timezone context is separately retained as user/browser-supplied text. Work domain, priority, and context remain self-report, never clinical inference. The backend obtains the project, action identity/revision/text/stop condition, and primary criterion identity/version plus description/registry-revision snapshot from the saved objective; the renderer cannot submit replacement snapshots.

### Prospective observation rules

Planning must precede the cue. The default study window is 900 seconds. Ordinary opportunities support a prospectively chosen 60–86,400-second window; configured E00/E01 protocols freeze 900 seconds. Availability is 1–1,440 estimated minutes. Eligible opportunities cannot simultaneously declare known unavailability; ineligible opportunities require it. Nothing is created on an unscheduled day. No scheduled opportunity can be deleted or retrospectively made ineligible through an ordinary command.

Reports are `started`, `nonstart`, or `unknown`, with `exact`, `approximate`, or `unknown` timing. An exact first start in the inclusive cue/window boundary yields `observed_start` and exact latency. An approximate start carries a positive uncertainty interval; it establishes a within-window start only when the entire interval fits, and never creates an exact latency. If the interval crosses a boundary, the result remains missing with an uncertainty reason after the window. An unresolved report while the window is open is rejected with a pending explanation. A nonstart is accepted only after the window and stores `censored_at_seconds`, with null latency. A known first start after the window retains its start timestamp and late-start flag, while the primary outcome remains a censored nonstart. Unknown timing never manufactures a timestamp or known 900-second latency.

Every subsequent observation requires `correction_reason` and references the immediately preceding observation through `supersedes`; earlier observations remain present. `priority_advanced` and `quality_note` are optional domain-specific observations. An optional `acceptance_id` must reference an existing Stage 3b acceptance for the same objective; it grants no new authority or credit.

### Study protocol and presentation

E00 has no allocation. E01 freezes 60 opportunities in 15 balanced blocks of four, with two `d` and two `d_plus_b` in each. A UUID-v4 seed feeds the documented `sha256-fy-v1` counter stream, rejection sampling, and Fisher–Yates shuffle; SHA-256 of serialized allocation verifies the exact sequence. The sequence is frozen at configuration. Originally eligible opportunities consume positions transactionally before the 60-second preview boundary; ineligible plans consume none. Overlapping E01 presentation windows are rejected. Exhaustion requires a new explicit protocol decision, never automatic extension or rerandomization.

Lifecycle decisions use `configured`, `ready`, `active`, `paused`, `completed`, and `abandoned`. E01 cannot move directly from configured to active. Activation/resumption requires a reason containing the owner's feasibility decision and the exact acknowledgements `e00_assessed`, `frozen_protocol`, `schedule_epoch`, `assignment_and_window`, `matched_support`, `as_assigned_analysis`, and `voluntary_stop`. The app records that acknowledgement; it cannot verify human feasibility from engineering tests. At most one study is active at any time, including historical intervals. Declaring a new epoch pauses active E01 assignment and preserves every existing assignment. Each planned study opportunity references its explicit active protocol decision. Epoch and manifest records remain immutable; a new epoch requires a new protocol configuration for new-epoch opportunities.

The declared preview runs from cue minus 60 seconds to cue. The checkpoint becomes available after the observation window; construction remains hidden during the window and before the assigned boundary. D receives matched numeric progress and no automatic construction. D_PLUS_B adds the scene only at those boundaries while construction is enabled. Leaving/pausing the study remains possible. Optional viewing outside a study is recorded separately; an active-study voluntary override requires an explicit reason. Exposure kinds distinguish `cue_delivered`, `preview_delivered`, `checkpoint_delivered`, `seen`, `voluntary_world`, and `deviation`. A delivered view is never evidence of being seen. No background notification or app-closed delivery is fabricated.

Assignment and exposure snapshots preserve their authoritative `ledger_sequence` and issued-BC prefix, plus the saturation flag. Assignment prefixes cannot decrease with allocation position. Exposures also require the exact `preference_sequence` that governed construction visibility (alongside their `protocol_event_id` and `ledger_sequence` references), so later toggles (including those in the same millisecond) cannot reinterpret whether delivery was permitted. The renderer's fixed `h1-ledger-64-v1` scene uses `min(64, workspace.progress.wallet_bc)`; retained correction credits remain in the ledger, and credits above 64 remain valid without extending the scene. `preferences` stores append-only construction visibility decisions; hiding the view persists through restart and restore.

Burden categories are `planning`, `logging`, `review`, `game`, `correction`, `other`, and `useful_work`. Each entry declares exclusive user-estimated minutes, bounded at 1,440; useful work is separate from management burden. No unrelated application activity or nested automatic interval monitoring is collected.

### Complete restore and compatibility

Modern v3 export/restore preserves all original study rows, frozen assignments, missing/unobserved opportunities, corrections, exposure distinctions, preferences, and Stage 3b authority history. Validation checks deterministic allocations/hashes, causal lifecycle references, assignment positions, timing/classification, ledger prefixes, global identities, references, and complete reward invariants before replacement. The existing shared 10,000-record and 10-MiB limits include every study row. No silent truncation occurs.

Legacy v2 can initialize a compatible workspace with empty study fields. Any v1/v2 replacement is rejected if local study history or presentation preferences exist. The prior v1 earned-credit protection also remains. Restore preview reports opportunity, assignment, outcome-event, manifest, epoch, exposure, burden, and reflection counts in addition to inherited authority counts. Explicit replacement saves a complete durable pre-restore v3 backup before its transaction. Migration saves a coherent SQLite backup of the old schema before additive changes; any failure preserves the original database.

## Stage 3b extension

The Stage 3b database version was 3 and complete interchange version was 2. The Stage 3a section below remains the legacy v1 input contract. Every current `Workspace` also has a required `progress` object. Its exact strict types and validation are in `src-tauri/src/progress.rs`; the renderer types are in `src/platform/client.ts`. The architecture and compatibility policy are recorded in ADR-0004.

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
