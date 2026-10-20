# ADR-0005 — separate opportunity history from accomplishment authority

**Status:** Accepted for owner-authorized Stage 3c.

## Decision

Extend the existing private SQLite repository and narrow Rust command boundary with additive opportunity/study records. Freeze prospective action, eligibility, epoch, cue, window, and assignment context; append outcome corrections and exposure/deviation observations. Existing work identities are referenced, not duplicated into another task manager.

Keep experiment assignment distinct from presentation delivery and reported viewing. Freeze a reproducible balanced sequence before activation; persist consumption atomically and hide future allocations from the normal renderer. Explicit lifecycle decisions govern activation, pause, epoch changes, amendments, and completion. Synthetic isolated tests can exercise activation; the agent cannot enroll the owner's normal profile.

Use the existing ledger as the sole H1 progression authority. The frontend renders one fixed original SVG with 64 ordered restoration units. Rendering, opportunities, outcome correction, and study restore never invoke acceptance or issue/spend grants. Numeric feedback remains matched between conditions. The scene's capacity is independent of the lifetime ledger total and of excluded Stage 4 economies.

Upgrade complete interchange to v3, preserving frozen schedules and all missing/corrected observations alongside the unchanged Stage 3b authority graph. Reject older exports that would erase consequential study history. Backups and restore remain complete replacement under explicit user confirmation, not a merge system.

## Rationale and limits

Independent history avoids conflating app engagement with work and prevents favorable-only denominators. An immutable prospective definition prevents retrospective reinterpretation; append-only correction preserves both the original account and current interpretation. Local reproducibility is sufficient; cryptographic protection against the database owner is not claimed.

No background cue delivery, OS notification, AI, external validator, or telemetry is added. Closed-app exposure may be absent; assignment still remains. A local small personal comparison cannot establish population-level efficacy, and engineering tests cannot establish E00 feasibility. Report timing uncertainty, missingness bounds, epochs, and exposure limitations explicitly.
