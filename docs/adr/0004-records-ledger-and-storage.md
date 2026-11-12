# 0004. Records, the ledger and local storage

- **Status:** Accepted
- **Date:** 2026-11-12

## Context

Every sale is a permanent record that must stay readable and verifiable for years (fiscal retention is often 7–10 years), across many app versions, on every peer. Peers receive records in any order and may run different versions (PLAN §6).

## Decision

**Encoding.** Payloads and headers use **postcard** (compact, serde-based, stable 1.x wire format). A record is `Ed25519 signature (64 bytes) | body`, where `body = postcard((Header, payload bytes))`. The payload stays opaque bytes inside the body, so peers store and forward event kinds they don't know.

**Hashing and signing.** `hash = SHA-256("tillstone/event/v1\0" || body)`; the device signs the hash with Ed25519 (the same key as its iroh identity). Bodies are hashed exactly as stored and never re-encoded, so verification can't break when encoding details change. SHA-256 was chosen over BLAKE3 because auditors and fiscal regimes widely accept it.

**Evolution.** Event kinds have permanent numbers; each payload has a schema version. Changing a payload means adding a new version and an upcaster in `Event::decode`. Inside payloads, enum variants and struct fields are append-only too (postcard encodes enums by position). `tests/fixtures/event_wire_v1.txt` locks the encoding.

**Chains.** Each device's records form a gap-free, hash-linked chain. `ChainTracker` treats a repeated record as a harmless duplicate, a missing predecessor as "wait" (pending), and two different records at the same position as **equivocation**, proof that a device rewrote its history.

**Rebuilding state.** The `Book` rebuilds each order by folding its records sorted by hybrid logical clock (then device, then sequence). The fold uses only data inside the records (lines carry price and tax snapshots; splits and merges carry full line copies), so every peer with the same records reaches the same state in any arrival order. Checks that depend on configuration (permissions, modules, fiscal mode) are **audits**: errors on the creating device (`Book::check`), warnings on receivers.

**Coordination rules.**
- Any device may take a check's lease if no other device holds it unexpired. Restructuring (split, merge, payment, close) needs the lease, or must come from the opener when nobody holds one.
- Events that record real money (cash tendered, card payments started or captured) are **never skipped** by the fold. A lease violation there is a warning, and the order shows as overpaid, so nothing is lost.
- Devices must merge the latest timestamp they've seen (`Book::latest_hlc`) into their clock before creating an event, so their events sort after everything they know about.

**Configuration conflicts** are resolved per **record** (latest change wins, by hybrid logical clock), not per field as first sketched in the plan. Per-record is simpler and enough for catalog edits; per-field merging can be added later as a new event kind if real conflicts show up.

**Storage.** SQLite with **SQLCipher** (`rusqlite` with vendored OpenSSL), a raw 256-bit key from the platform keystore, WAL mode with `synchronous = FULL`. There's no API to delete a record. Refused records are quarantined, not dropped. Records waiting for predecessors are stored too, so they survive restarts. Checkpoints store the Book's state plus order records, which are decoded without re-verifying signatures (they were verified on arrival, and the database is encrypted).

**Personal data** lives in its own tables, encrypted per customer with XChaCha20-Poly1305, never in events. Erasing deletes the key and ciphertext on every peer (via a `CustomerErased` event). Backups taken before an erasure still hold the key until they expire, so backup retention must be finite.

## Alternatives considered

- **CBOR / JSON:** self-describing but larger; we don't need self-description because the body is never re-encoded and schemas are versioned.
- **A central sequencer for orders:** simpler conflict handling, but it would bring back a server (PLAN §3).
- **Per-field configuration merging:** more precise, but more code and more ways to get it wrong, with no evidence yet that it's needed.

## Consequences

- Old records stay verifiable forever, at the cost of discipline: never edit a shipped payload or enum.
- The Book keeps every order's records in memory for now. Compacting old closed orders is future work.
- Large-discount approval uses a fixed threshold (`LARGE_DISCOUNT_BP`, 20 %) until discounts carry their own threshold in their snapshot.
