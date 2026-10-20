# ADR-0006 — River Reach capacity and construction authority

Status: accepted for owner-authorized Stage 4; executed verification is recorded separately.

## Decision

Retain React/TypeScript/Vite, Tauri 2, and the existing private SQLite repository. Add a bounded Rust world module and immutable `river-reach-v1` content, independently of Stage 3b accomplishment authority. Reuse the existing immediate transaction coordinator and complete exportability validation. World commands may read issuance and correction records, but cannot mutate them.

Purchase permanent province capacity at four BC per site, up to six sites and 24 BC. Geographic site IDs define placement, not separate resale assets. Derive spend from immutable commitments, reservations from one active blueprint, and available credit from issuance minus both. Existing `wallet_bc` and H1 continue to mean cumulative issuance. Credit attribution uses stable FIFO ledger identities and is described at province level because layouts are freely replaceable.

A canonical typed command identity, revision, ruleset, historical issuance prefix, digest, and original result bind each committed transition. New state, spend, history, and replay result commit together. An exact retry returns its original result; altered replay conflicts. Complete portable validation reconstructs this small chapter's legal transitions and rejects impossible final projections, undo history, spending, or endings. This is a chapter-specific integrity check, not a general replay platform.

Reconfiguration changes only the active layout within funded capacity. Undo changes layout only. Active blueprints must be canceled first. Chapter completion snapshots one immutable ending; subsequent layouts are sandbox alternatives with no added spend or reward. No timers or automatic production exist.

Database schema 5 and interchange 4 extend the existing coherent migration/backup/restore path. Old exports cannot erase local world history. New complete restores preserve exact commands and accounting after explicit preview and backup. Keep the shared 10,000-record and 10-MiB bounds.

## Experimental boundary

Lazy-load the strategic interface only on voluntary navigation, and withhold it during active E00/E01 in both UI and backend. Existing explicit study pause/leave controls remain available. Do not alter H1, allocation sequences, or frozen treatment policies. The previously identified unbounded H1 post-window checkpoint requires a separate versioned policy correction before real E01 activation; Stage 4 neither resolves that research-launch issue nor establishes behavioral benefit.

## Consequences

The six-site economy is inspectable, reversible at the layout level, and nonpunitive after factual corrections. Investment is irreversible; cancellation and undo do not refund it. Prospective previews and explicit spend/ending confirmations communicate that distinction. Original SVG and semantic keyboard controls fit the existing stack without a game engine or new dependency. Local owner edits to SQLite or a compromised operating system remain outside the command-boundary guarantee.
