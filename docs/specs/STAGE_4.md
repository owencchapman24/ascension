# Stage 4 — The Beacon Commonwealth: River Reach

Owner-authorized implementation after the merged Stage 3c baseline `7b522be`. This public specification contains only the bounded implementation rules, not private research. Work on `feat/stage-4-river-reach`; verified local commits are authorized. Push, merge, release, deployment, real study activation, AI, and Stage 5 are excluded.

## Purpose and authority

Implement one optional, persistent strategy chapter while preserving the saved next action and all Stage 3a/3b/3c behavior. Work opens first. World rendering loads only after voluntary navigation. Stage 3b alone owns evidence, acceptance, reward entitlements, the four-BC lifetime parent cap, and `bc-v1` issuance. Gameplay cannot invoke acceptance, create parents, register validators, or issue credits.

The Rust world module owns construction legality and spending under the immutable `river-reach-v1` ruleset. The frontend submits choices and displays backend previews; it cannot supply prices or capabilities. No new framework, dependency, service, currency, production clock, upkeep, absence penalty, or procedural content is required.

## Province and content

Six stable sites form a two-by-three topology: `A1 A2 A3` above `B1 B2 B3`. Only shared-edge neighbors are adjacent. A river camp, dormant beacon, crossing, and terrace are free scenery, not evidence of work.

| Building | Sites | Capacity value | Engineering | Knowledge | Diplomacy |
| --- | ---: | ---: | ---: | ---: | ---: |
| Foundry | 2 adjacent | 8 BC | 5 | 0 | 0 |
| Grand Archive | 2 adjacent | 8 BC | 0 | 5 | 0 |
| Embassy | 2 adjacent | 8 BC | 0 | 0 | 5 |
| Survey Lodge | 1 | 4 BC | 2 | 1 | 0 |
| Observatory | 1 | 4 BC | 0 | 2 | 1 |
| Caravan House | 1 | 4 BC | 1 | 0 | 2 |

At most two copies of each type may exist. Queuing cannot bypass the limit. Capabilities are reusable properties of completed legal layouts, never currencies or measures of the person's abilities. Partial blueprints contribute nothing.

| Route | Requirements | Authored result |
| --- | --- | --- |
| Stone Causeway | Engineering >= 10; Knowledge >= 4 | Durable connections across the river |
| River Compact | Diplomacy >= 10; Knowledge >= 4 | A negotiated river assembly |
| Beacon Mesh | Engineering, Knowledge, Diplomacy each >= 6 | A distributed network of complementary beacons |

All routes are equally legitimate. Select or revise a preference before completion. Exhaustive enumeration of all `3^6 = 729` count combinations must find 3/3/1 qualifying combinations within six sites, with spatial legality checked separately.

## Economy and construction

`issued` remains the Stage 3b issuance sum and existing `wallet_bc` meaning. `spent` is irreversible construction investment; `reserved` is the active blueprint's unspent hold. `available = issued - spent - reserved`. All are nonnegative, spent plus reserved cannot exceed issuance, chapter spend cannot exceed 24 BC, and funded capacity equals spent / 4 with a maximum of six.

Capacity belongs to the province and is fungible across its six named sites. It is not a permanent purchase of one geographic location. Occupancy may be less than funded capacity. A new blueprint costs only `4 * max(0, current occupied sites + footprint - funded capacity)`. Show occupied, free funded, and unfunded capacity explicitly.

One durable blueprint may be active. Queue, reserve currently available credit, replace with an explicitly reviewed choice, cancel, and complete are explicit commands. Partial reservations do not build, fund sites, or provide capability. Cancel releases only unspent reservations. Completion revalidates the live ledger, world revision, footprint, adjacency, copies, occupancy, capacity, and funding inside one SQLite immediate transaction. It converts the deficit into committed spend, releases the reservation, creates the building, and persists the original command result atomically.

Commands carry schema/content version, UUID command identity, chapter reference, expected world revision, and typed operation parameters. Exact replay returns the original result before stale checks; same-ID changed payload fails. No silent retry of a changed decision. A deterministic FIFO policy attributes capacity investment to preexisting issuance entry IDs/sequences. Corrections retain fictional investment and expose corrected-credit provenance at province level; they never renew an entitlement.

## Reconfiguration, ending, and history

Preview a full proposed layout before committing. Move, rotate, remove, split specialists into mixed buildings, or replace types within owned capacity freely. No overlapping parcels, diagonal specialists, excessive copies, or unfunded occupancy may commit. Cancel an active blueprint before reconfiguring or undoing, so its reservation cannot become orphaned. Undo restores a durable previous legal layout without changing issuance, spend, or funded capacity.

Only explicit confirmation against the current qualifying layout commits a route. Preserve one immutable ending identity, timestamp, ruleset, route, exact layout, capabilities, spend, funded capacity, and authored narrative in the Chronicle. No reward follows completion. Later sandbox layouts are clearly labeled and cannot alter the original ending, add spend, or consume another parent budget.

## Persistence, portability, and study isolation

Add database migration 5 without editing migrations 1–4. Retain coherent pre-migration backup, transactional upgrade, full validation, and rejection of future versions. Explicit entry initializes a zero-spend chapter. All state resides in the existing private SQLite repository.

Complete interchange v4 includes the full Work/reward/study graph plus world state, spending attribution, queue/reservations, command digests/results, history, undo, ending, and sandbox. Validate complete historical transitions and their issuance prefixes before replacement. Reject v1/v2/v3 replacement when local world history exists; preserve earlier reward/study protections. Modern replacement requires a validated preview and complete durable backup. No history merging or reset shortcut.

H1 retains its exact 64-unit `min(64, cumulative issued BC)` rule. Spending does not reverse restoration. H2 is withheld while either E00 or E01 is active; existing explicit pause/leave controls remain accessible. It is never D_PLUS_B treatment content. Preserve frozen protocols and document the separately versioned H1 checkpoint-policy correction required before real E01 activation. No human feasibility or efficacy result is inferred from engineering tests.

## Verification gates

Execute the inherited suite, adversarial real-SQLite economy/replay/concurrency/failure tests, spatial/capacity/undo/route tests, 729-combination balance enumeration, complete export/restart/restore/migration cases, accessible UI/client tests, typecheck/build/fmt/strict Clippy, and disposable installed-Windows lifecycle/offline verification. Use only synthetic profiles and ignored artifacts. Verify isolation before destructive testing. Inspect normal/narrow native screenshots, keyboard flows, and timings. Record exact commands, counts, limitations, local commits, and merge verdict in `docs/implementation/STAGE_4_VERIFICATION.md`.
