# Tillstone — Master Plan

> Status: v5 · 2026-11-11
> Goals: **secure, fast, robust, simple, cross-platform, identical UI everywhere.**
> Project shape: **free and open source, self-hosted, peer-to-peer.** No company, no hosted service, no accounts with us.

---

## 0. Project Shape

| Decision | Choice | Why |
|---|---|---|
| Distribution | **Self-hosted.** Each business runs its own copy. | No company to run servers; businesses own their data. |
| Tenancy | **One install = one business** (any number of locations). | Removes cross-business data leaks as a category. |
| Topology | **Peer-to-peer.** Start with one register; add another and they find each other and sync. No server required. | Nothing extra to run; every device holds a copy. |
| Optional always-on peer | `posd`: the same core with no screen, on a mini PC, Raspberry Pi, NAS or VPS. Just another peer, never a master. | Bridges locations, keeps long-term history, runs backups. |
| Shared hardware | Any device can use the business's **network** printers and **smart** card readers, with named routes and failover (§10). Sharing a USB/Bluetooth device plugged into another register comes after 1.0. | Most of the benefit, a fraction of the complexity. |
| Business types | Retail, quick service, full service, bars, and hybrids (§5) | One order model + feature modules |
| Payments | Cash built in; cards through **provider plugins**, Stripe Terminal smart readers first (§8) | No lock-in for an open project |
| Platforms | **Android, Windows, Linux, macOS** through v1.0. **iPad/iPhone after 1.0.** | iOS needs a paid Apple account and suspends background apps, which breaks peer-to-peer hosting (§13). Flutter keeps the UI identical, so adding iOS later is cheap. |
| Telemetry | **None.** Crash reporting is opt-in, to a destination the business chooses. | Privacy and trust |
| Countries | **Every country.** A tamper-evident journal everywhere, plus per-country **fiscal adapters** for places that legally require certified, tamper-proof registers (§15). | One product worldwide, without pretending software alone can be tamper-proof |
| License | **Apache-2.0** | Permissive, includes a patent grant, easy for businesses, contributors and certified distributions (§15) |

---

## 1. Simplicity Rules

1. **Two languages: Rust and Dart.** Dart draws the UI. Rust does everything else.
2. **One program**, plus `posd`, which is the same Rust core without a UI.
3. **One database: SQLite** everywhere.
4. **One networking stack: iroh.** No fallback stacks unless the M0 proof fails.
5. **Business logic only in `pos-core`.** The UI never computes money.
6. **Hardware code only in Rust** (`pos-devices`). Dart only renders receipts and tickets to images or text.
7. **Not used:** Kubernetes, Docker as a requirement, Postgres, Redis, Kafka, GraphQL, microservices, Prometheus, any cloud service run by us.
8. **Dependency budget.** Every new crate or package needs a stated reason in the PR.
9. **One way to do each thing:** one state library, one HTTP client, one logging approach.
10. **Every feature answers "what happens offline?"** before it's built.
11. **Decision records (ADRs)** for every significant choice, in `docs/adr/`.
12. **Secure by default.** The safe setup is the only setup; nothing to get wrong.
13. **Ship small.** v0.1 is one device with no networking (§18). Each later milestone adds one big thing.

---

## 2. Technology Stack

### App (all device roles)
| Concern | Choice |
|---|---|
| UI | **Flutter** (latest stable) + **Dart 3** |
| State management | **Riverpod** |
| Navigation | **go_router** |
| Immutable models | **freezed** |
| Rust ↔ Dart bridge | **flutter_rust_bridge v2** (typed calls + streams) |
| Local database | **SQLite + SQLCipher** (encrypted), owned by Rust (`rusqlite`, `bundled-sqlcipher`) |
| Peer-to-peer | **iroh** (QUIC; devices addressed by public key; local discovery; NAT hole-punching; encrypted relay). Check maturity and API stability in M0 before committing. |
| Provider API calls | `reqwest` + `rustls` |
| Key storage | Android Keystore, Windows DPAPI/TPM, macOS Keychain, Linux Secret Service |
| Crash reporting | Opt-in only; Sentry-compatible, pointed at a destination the business chooses |

The **management back office is part of the same app** (a permission-gated Manage section). An owner checks sales from home by joining their own laptop or Android phone as a device.

### `posd` (optional always-on peer)
| Concern | Choice |
|---|---|
| Language | Rust (edition 2024), Tokio |
| Database | SQLite (WAL), same code as the app |
| Identity | Joins by QR code like any device |
| Backups | Built in: scheduled, encrypted, to a local disk/USB drive and/or any S3-compatible storage |
| Email receipts | Optional, any SMTP server |
| Health | Shown in the app's Doctor screen; structured logs (`tracing`) |
| Packaging | Single static binary (Linux/macOS/Windows, x86_64 and ARM64), optional container image |
| Relay | Can also run a **private iroh relay** for the business, so it doesn't depend on public relays |

### Project tooling
| Concern | Choice |
|---|---|
| Repo | Monorepo (§16) |
| CI | GitHub Actions on Linux, macOS and Windows runners |
| Tasks | `just`, Cargo workspace, Dart pub workspace |
| Docs | mdBook |
| Signing | Sigstore/cosign for binaries and images; platform code signing for apps (§17) |
| Supply chain | `cargo deny`, `cargo audit`, `osv-scanner`, Dependabot, pinned Action SHAs, SBOM per release, `gitleaks` |

---

## 3. Architecture

```
  v0.1: ONE REGISTER          v0.2+: SEVERAL DEVICES ON A DEDICATED POS NETWORK
  ┌──────────┐                ┌──────────┐◄──────►┌──────────┐
  │ Register │                │Register A│        │Handheld B│
  │ (all     │                └────▲─────┘        └────▲─────┘
  │  data)   │                     │   ┌──────────┐    │
  └──────────┘                     └──►│ Kitchen  │◄───┘
                                       │ display  │
                                       └──────────┘
                     network printers · smart card readers (usable by any device)

  MULTIPLE LOCATIONS / REMOTE (optional)
  [Shop 1] ◄── end-to-end encrypted (direct, or via relay) ──► [Shop 2]
                              ▲
             optional posd (always-on: archive, backups, private relay)
                              ▲
                     owner's laptop at home

  every device: Flutter UI → pos-core · pos-sync · pos-devices (Rust) → SQLite (encrypted)
```

**How it works**
- Every device is a **peer**. There is no master.
- Peers hold the data **their role needs** (§9): registers hold everything; kitchen displays hold only tickets and menus.
- To add a device, tap "Add device", scan the QR code, confirm the matching code on both screens, and have the owner approve (§6).
- Peers find each other on the local network and sync directly. Across sites they connect over end-to-end encrypted links, directly or via a relay that only sees encrypted bytes.

**Why this works:** each device only ever appends to **its own** log. Syncing is copying logs. Devices never edit the same record, so sales, payments, cash and stock need no merging. A few things need explicit rules (§6).

**Core principles**
1. **Offline-first.** Any device sells alone, at any time.
2. **Append-only, single-writer logs.** Corrections are new events; history is never edited.
3. **Same core everywhere**, with version-tolerant validation (§6).
4. **Never touch card data.**
5. **One design system.**
6. **No device accepts connections from the open internet.**
7. **No single device is critical** once there are two or more devices plus an off-site backup.

---

## 4. Domain Model

### Configuration (replicated; each edit is an event)
- **Business** → **Location** → **Revenue Center** (§5) → **Device** (role + revenue centers)
- **Staff**: name, role, PIN hash, allowed locations
- **Role**: permissions (`order.create`, `order.refund`, `discount.over_10pct`, `drawer.open`, `manage.catalog`, `reports.view`, ...)
- **Catalog**: one item library. An **Item** has a kind (`physical_good`, `menu_item`, `service`, `gift_card`, `fee`) and optional Variants, ModifierGroups → Modifiers, Barcodes/SKUs, stock tracking and kitchen routing. **Menus** are per-revenue-center views (items, layout, price, hours).
- **Price** per menu, optionally per time window
- **Tax**: TaxRate and TaxRule (items, locations, inclusive/exclusive, rounding)
- **Discount**: percent or fixed, line or order, conditions
- **Floor plan**: Areas → Tables
- **Routes**: logical hardware destinations ("Front receipts", "Kitchen", "Front drawer") with failover lists (§10)

### Personal data (mutable, deletable, never in events)
- **Customers** and any contact details live in separate records, **each encrypted with its own key**. Events refer to them by ID only.
- Deleting a customer deletes their key on every peer, which makes their data unreadable everywhere, including in old backups ("crypto-shredding"). This is how a permanent log stays compatible with GDPR/CCPA deletion rights.
- Staff names are configuration, not events; events carry `staff_id`.

### Events (append-only, per device)
Every event carries: `event_id` (UUIDv7), `event_type` + `schema_version`, `core_version`, `location_id`, `device_id`, `staff_id`, `device_seq` (gap-free per device), `occurred_at`, `hlc` (hybrid logical clock), `prev_hash`, `hash`, `signature`, `payload`.

| Event | Notes |
|---|---|
| `ShiftOpened` / `ShiftClosed` | Float, counted cash, expected vs actual |
| `OrderOpened` | Revenue center, fulfillment type, table, guest count, customer ID, tab name |
| `LineAdded` / `LineRemoved` / `LineQtyChanged` | **Snapshot** of price, tax rules and modifiers used; optional seat and course |
| `OrderSent` / `CourseFired` | Sends lines to kitchen routes |
| `TicketBumped` / `TicketRecalled` | Kitchen display progress |
| `LineVoidedAfterSend` | Reason required; feeds waste reports |
| `CheckLeaseTaken` / `CheckLeaseReleased` | Who may restructure a shared check (§6) |
| `TableAssigned` / `OrderTransferred` | |
| `OrderSplit` / `OrderMerged` | Require the check lease |
| `DiscountApplied` | Approving manager if required |
| `TipAdded` / `TipAdjusted` | |
| `PaymentStarted` / `PaymentCaptured` / `PaymentFailed` | Provider payment ID, reader ID |
| `CashTendered` | |
| `OrderClosed` | Full computed breakdown |
| `OrderVoided` | Before payment, with reason |
| `RefundRequested` / `RefundIssued` | Original order and lines, reason, approver; card refunds complete on a device holding the refunds key (§8) |
| `CashMovement` / `DrawerOpened` | Paid in/out, no-sale, every drawer opening |
| `InventoryAdjusted` | Received, counted, damaged, transferred |
| `Correction` | Compensates a bad event (e.g. one created by a bug), with reason |
| `ConfigChanged` | One field of one config record, ordered by `hlc` |
| `DeviceAdded` / `DeviceRevoked` / `KeyRotated` | Membership (§6), signed by the owner authority |
| `PrintJobFailed` | Undeliverable ticket or receipt |
| `FiscalSigned` | Signature, counters and receipt fields from the fiscal adapter (§15) |
| `FiscalOutageStarted` / `FiscalOutageEnded` | Fiscal device unavailable; the legally defined fallback procedure applies |

**Event formats are a permanent promise.** Every type is versioned; old versions are converted on read by small "upcaster" functions; the meaning of an existing event never changes. Old events are kept forever (subject to retention, §6).

**Read models** (open orders, today's totals, stock) are built from events and **checkpointed**, so a device never replays years of history on startup (§6).

### Money rules
- **`i64` minor units** + ISO-4217 currency code. Never floating point.
- Percentages in **basis points**.
- Explicit rounding per tax rule.
- **Splits add up exactly** (largest-remainder allocation).
- Every line snapshots the price and tax it was sold at, so receipts never change and any device can re-check the math (§6).

---

## 5. Business Modes and Hybrids

**One product.** A boutique, a burger counter and a steakhouse differ only in **feature modules** and **register layout**.

### Revenue centers
Each location has one or more revenue centers, each with its own mode preset (Retail, Quick Service, Full Service, Bar), menu, prices, hours, taxes, routes, tip settings, order numbering and reporting bucket.

| Business | Setup |
|---|---|
| Restaurant with a gift shop | "Dining room" (Full Service) + "Gift shop" (Retail). A mug on a dinner check is taxed as merchandise and credited to the gift shop. |
| Café in a bookstore | "Café" (Quick Service) + "Books" (Retail) |
| Brewery | "Taproom" (Bar) + "Merch" (Retail) |
| Food hall | Many centers, one per vendor |

### Feature modules
| Module | Retail | Quick Service | Full Service | Bar |
|---|:-:|:-:|:-:|:-:|
| Barcode scanning, SKU search | ✅ | optional | – | – |
| Variants | ✅ | ✅ | ✅ | ✅ |
| Modifiers | optional | ✅ | ✅ | ✅ |
| Stock tracking | ✅ | optional | optional | optional |
| Order numbers / pickup name | – | ✅ | – | – |
| Dining option | – | ✅ | ✅ | – |
| Kitchen routing | – | ✅ | ✅ | ✅ |
| Tables and floor plan | – | – | ✅ | optional |
| Seats and courses | – | – | ✅ | – |
| Open checks over time | optional | optional | ✅ | ✅ |
| Tabs with card pre-auth | – | – | optional | ✅ |
| Tips | optional | ✅ | ✅ | ✅ |
| Split / merge / transfer | – | optional | ✅ | ✅ |
| Returns and exchanges | ✅ | – | – | – |

Modules are **enforced in `pos-core`**, not just hidden in the UI.

### Device roles
**Register** · **Handheld** · **Kitchen Display** · **Customer Display** · **Order Board** · **Manager** (owner's laptop or phone; management only) · Kiosk (later)

### Register layouts (same design-system panels)
- **Retail**: scan/search first, grid, cart, returns
- **Quick Service**: big item grid, modifier sheet, dining option, order number, Send + Pay
- **Full Service**: floor plan → table → seat/course-aware order → Send → Pay/Split
- **Bar**: open tabs, quick drink grid, pre-auth on open

---

## 6. Sync Protocol

### Membership and authority
- **Owner authority.** At setup the first device creates an owner key. It is stored encrypted with the **owner passphrase** (not only the keystore), so a stolen device without the passphrase can't use it. It can live on several owner devices.
- **Only the owner authority can add devices, revoke devices, rotate keys, and grant or remove manager roles.** Managers can do day-to-day management (catalog, staff PINs, reports), but they can't change who belongs to the business.
- **Joining a device:**
  1. On an existing device, tap "Add device". It shows a QR code containing the business ID, its address, and a single-use invite that expires in 5 minutes.
  2. The new device scans it and generates its own key.
  3. **Both screens show the same 6-digit code.** The person confirms they match, which stops someone who photographed the QR code from joining.
  4. The owner enters the passphrase, and a `DeviceAdded` event (role, revenue centers) is signed by the owner authority.
- **Revoking a device** records the **last event number trusted from it**. Its later events are held in quarantine for owner review, never applied automatically. Revocation always wins over anything the revoked device says; a revoked device can't revoke others because it isn't the owner authority.
- **Restored or wiped devices rejoin as new devices** with a new key. A device identity is never restored from backup, so its event numbers can't be reused.
- **Tamper detection:** two different events with the same `device_id` and `device_seq` are proof that a device rewrote its history. Both are kept as evidence, the device is flagged, and the owner is alerted.
- **Recovery kit:** at setup the owner prints or saves a recovery code that can recreate the owner authority if every owner device is lost. Setup can't finish without it.

### Replicating logs
1. Each device writes to its own hash-chained, signed log.
2. When peers connect they swap a **version vector**: the highest `device_seq` held from each device.
3. Each sends what the other is missing, in order; new events are pushed live while connected.
4. Peers forward each other's events, so a sale from A reaches C through B.
5. **Priority:** membership and permission changes sync **first**, then everything else.
6. **Flood protection:** if one device suddenly produces events far outside its normal pattern (e.g. thousands of refunds), peers keep receiving and storing them but pause applying them to reports until a manager reviews. One buggy or hostile device can't silently corrupt every peer's totals.

### Validation: tolerant of different versions
Devices may run different versions during upgrades, and configuration arrives with some delay. So a receiving peer:
- **Rejects** (quarantines) an event only for **hard failures**: bad signature, unknown or revoked device (past the cutoff), broken hash chain, malformed data.
- **Re-checks the math using the snapshot inside the event** (the prices and tax rules it was sold with), never its own current catalog. Each event records the `core_version` that created it.
- **Warns** instead of rejecting for anything else: a permission the receiver thinks the staff member didn't have, a total that doesn't match, an unknown newer event type. Warnings go to a manager review list. The event is still stored and forwarded.
- Stores and forwards **unknown newer event types** without interpreting them, so older peers never lose data from newer ones.

### Conflict rules
| Data | Rule |
|---|---|
| **Configuration** | Each field change is ordered by hybrid logical clock; latest per field wins, ties broken by device ID. Deletes are tombstones. A same-field clash shows "changed on another device". |
| **Shared checks: adding items** | Anyone with access can add lines; each line has its own ID, so concurrent additions simply combine. Removals and voids target a specific line. |
| **Shared checks: restructuring** | Splitting, merging, transferring, taking payment and closing **require the check lease**. A device takes the lease with a live request to reachable peers and records `CheckLeaseTaken`. The lease is released on finish or after 2 minutes idle. **If peers can't be reached, only the device that opened the check may restructure it.** Lines added during a split go to the original check, never into the split-off ones. |
| **Two payments at once** | Prevented by the lease. If it still happens (e.g. a network split), both payments are kept, the check shows as overpaid, and a refund of the extra is prompted. Money is never discarded. |
| **Stock** | A sum of changes, which merges exactly. Can go below zero; flagged in reports. |
| **Order and receipt numbers** | Device-prefixed (`A-0142`), gap-free per device. Short pickup numbers (`A42`). |
| **Permission removed while a device is offline** | That device keeps the old permission until it syncs. Documented; permission changes sync first; actions taken in that window appear on the review list. |
| **Gift cards / store credit** (later) | Need another peer online to redeem, or a small configured offline limit. |

### Live messages (not in the log)
Presence, lease requests, reader claims, hardware status and print jobs travel as direct encrypted messages with IDs, acknowledgements and retries. Lasting outcomes become events.

### Snapshots, storage and retention
- Each peer **checkpoints** its read models regularly. A new device joins by downloading a **signed snapshot** plus recent events, not years of history.
- Events are compact: roughly 1 MB a day for a busy café.
- **Per-role data:** registers and `posd` hold everything; handhelds hold recent orders and configuration; kitchen displays and order boards hold only tickets and menus, with **no staff PIN hashes for other roles, no payment keys and no sales history**.
- A device prunes old events only after at least two other peers (or `posd`) confirm they hold them.

### Status
A small indicator: "3 devices · all synced" / "offline · 12 waiting" / "can't see other devices: check network". Never blocks the cashier.

---

## 7. Pricing and Tax Engine (`pos-core::pricing`)

- **Pure function**: `price(cart, catalog_snapshot, tax_rules) -> Totals`. No I/O, deterministic.
- Variants, modifiers, discounts, inclusive/exclusive/compound tax, rounding, splits, multiple tenders.
- **Versioned rules**: if a rounding or tax behavior changes, the old behavior stays available and is selected by the `core_version` recorded in the event, so old sales always re-check correctly.
- **Tests**: unit tests per rule; property tests (`proptest`: never negative, lines sum to subtotal, full refund nets to zero, splits sum exactly); golden receipt fixtures; fuzzing of all deserialization.
- **Tax as data**: community-maintained regional presets.

---

## 8. Payments

### Provider plugin interface
A Rust trait in `pos-payments`: `list_readers`, `claim_reader`, `start_payment`, `poll_status`, `cancel`, `capture`, `adjust_tip`, `refund`. Each business uses **its own** provider account; the project never touches the money.

### Stripe Terminal (first provider)
- **Smart readers** (e.g. Stripe Reader S700 / BBPOS WisePOS E) via server-driven integration: the register's Rust core asks Stripe to run a payment on a chosen reader; the reader handles the card.
- No native SDK in Flutter, so payments work identically on every platform.
- **Polling, not webhooks**, so nothing accepts incoming internet connections.
- **Shared reader pool**: each register has a default reader and can send to another. A register **claims** a reader (live message to peers) before using it, and handles Stripe's "reader busy" response gracefully.

### Keys: split by risk
| Key | Can do | Lives on | Protection |
|---|---|---|---|
| **Payments key** (Stripe restricted key) | Create, read, cancel and capture payments; manage readers. **No refunds.** | Register and handheld devices | Encrypted at rest; rotatable from any owner device |
| **Refunds key** (separate restricted key) | Refunds only | **Owner devices only** | Encrypted with the owner passphrase; needed for each refund session |

A cashier asks for a refund; it's queued as `RefundRequested` and completed when an owner or authorized manager approves on a device holding the refunds key. Cash refunds don't need it. This means a stolen register can't be used to drain past payments.

**PCI:** the business is the merchant and is responsible for its own compliance. Card data never touches this software, which keeps their burden minimal. Docs explain this plainly.

### Hard cases
| Case | Handling |
|---|---|
| App crashes mid-payment | `PaymentStarted` is logged first; on restart the status is checked and the payment resumed or cancelled |
| Network drops after the customer taps | Never "try again" blindly; check status, reconcile on reconnect |
| Double-tap on Pay | Idempotency key per attempt |
| Two registers send to one reader | Reader claim + "busy" handling |
| Split payments | Multiple payments per order; closes when paid = total |
| Bar tabs | Pre-authorize at open, capture with tip at close; stale-tab warnings because holds expire. Check provider rules for your country. |
| Tips | On-reader prompt, or tip adjust on a signed receipt |
| End-of-day | Reconciliation compares provider records with closed orders and flags mismatches |

**Cash** works fully offline.

---

## 9. Security Design

### Threat model
1. A stolen or lost device, possibly rooted
2. A dishonest employee
3. A hostile network
4. A stolen or modified device sending fake events or commands
5. A compromised dependency or release pipeline
6. Vulnerabilities in our public code
7. Unpatched installations, since nobody controls updates centrally

### Controls
| Area | Control |
|---|---|
| **Identity and transport** | Each device has its own key pair, addressed by public key. All traffic is end-to-end encrypted (QUIC, TLS 1.3) with both sides authenticated. Relays see only encrypted bytes. A business can run its own private relay on `posd`. |
| **Membership** | Owner authority only, passphrase-protected; matching-code confirmation on join; revocation cutoffs; tamper detection; recovery kit (§6). |
| **Least data per role** | Devices hold only what their role needs (§6). A stolen kitchen display reveals almost nothing. |
| **Data at rest** | SQLCipher everywhere; database key wrapped by the platform keystore. **Honest limit:** on cheap or rooted devices the keystore may be software-only, so a determined attacker with the device can read its data. That's why revocation, per-role data, the refund key split and PIN changes after theft matter. |
| **Staff login** | 4–6 digit PIN, Argon2id hashes, lockout with backoff, auto-lock, optional NFC badge. After a theft: revoke the device and the app prompts everyone to change their PIN. |
| **Owner actions** | Owner passphrase (plus optional TOTP) for membership, key changes, refunds-key use, full data export. |
| **Authorization** | Checked by `pos-core` on the creating device and re-checked on receivers as **warnings** (§6). **Honest limit:** other devices can verify which *device* signed an approval, not that a manager actually typed their PIN, so a modified device could fake approvals. The defense is detection: per-device and per-staff reports, outlier alerts, the review list and flood protection. |
| **Hardware** | Drawer openings and print jobs are permission-checked on the sending device and always logged. **Honest limit:** network printers accept commands from anything on the LAN, which is one reason for the dedicated POS network (§13). |
| **Tamper evidence** | Hash chains, signatures, gap-free numbering, duplicate-number detection. |
| **Network exposure** | No device listens on the public internet; no port forwarding. Dedicated POS network recommended for multi-device setups. |
| **Personal data** | Never in events; per-customer encryption keys; deletion by crypto-shredding (§4). |
| **Logging** | Redaction layer: no PINs, keys, tokens, card data or personal data in logs, enforced by tests. |
| **Security updates** | Devices check a **signed security notice file** published with each GitHub release (optional, on by default, no tracking). A security fix shows a persistent banner to owners until installed. |
| **Setup wizard** | No default passwords, encryption always on, recovery kit and **off-site backup** set up before the first sale. |
| **Loss prevention** | Blind cash close; per-employee void/refund/discount/no-sale reports with outliers flagged; refunds never exceed the original; removing staff revokes their PIN everywhere at next sync; age-check prompts. |
| **Releases** | Signed tags, signed binaries and images, SBOM, reproducible builds as a goal. |
| **Vulnerability handling** | `SECURITY.md`, private reporting via GitHub Security Advisories, disclosure policy, fixes for the latest release; seek a grant-funded audit before 1.0. |

---

## 10. Hardware

All drivers live in Rust (`pos-devices`). Dart renders content only.

| Device | Approach |
|---|---|
| **Receipt printer** | Network ESC/POS (TCP 9100), e.g. Epson TM-m30, Star mC-Print. Receipts rendered by Flutter to a bitmap (**image mode**) so they look identical everywhere. Rendering runs off the UI thread; headers and logos are cached. |
| **Kitchen printer** | Network ESC/POS impact printers (e.g. Epson TM-U220) use **text mode**, because impact printers handle graphics poorly. Layout comes from the same ticket template. |
| **Kitchen display** | Any Android tablet, or a TV + mini PC, in KDS role |
| **Cash drawer** | Kicked by its printer; a "Front drawer" route maps to that printer |
| **Barcode scanner** | USB/Bluetooth HID keyboard mode on the device itself; camera fallback |
| **Card reader** | Provider smart readers, reachable by any register |
| **Local USB printer** | Supported for the device it's plugged into (Windows/Linux/macOS/Android). Sharing it with other devices comes after 1.0. |

### Routes and failover
- Configuration maps **logical names** ("Front receipts", "Kitchen", "Bar tickets", "Front drawer") to physical devices.
- Each route has an ordered **failover list**: kitchen printer → backup printer → kitchen display. The sending screen shows where the job went.
- **Every print job has an ID**, and devices remember recent IDs, so a retry after a lost acknowledgement never double-prints.
- Jobs wait on the sender until delivered. An undeliverable job becomes `PrintJobFailed` and appears in the reprint queue.

### After 1.0: hosted peripherals
USB/Bluetooth printers, scales and customer displays plugged into one device and shared with others, hosted only by Android, desktop or `posd` (never iOS). Same routes and job IDs.

A **community-tested hardware list** lives in the docs.

---

## 11. UI and Design System

**Goal: pixel-identical on every platform.**

- `design_system` package: tokens (`tokens.json` → Dart) for color, spacing (4px grid), radii, elevation, motion and type; a bundled open-licensed font (e.g. Inter); ~15 components (`PosButton`, `PosIconButton`, `PosTile`, `PosNumpad`, `PosPinPad`, `PosCartLine`, `PosTotalsPanel`, `PosDialog`, `PosSheet`, `PosToast`, `PosTextField`, `PosTabs`, `PosBadge`, `PosEmptyState`, `PosSyncIndicator`).
- **Pinned behavior**: one `ScrollBehavior` and `PageTransitionsTheme`, fixed `ThemeData.platform`, no `.adaptive()` widgets, custom context menus.
- **In-app numpad and PIN pad**; the OS keyboard never appears on register screens.
- **Layout by width** (compact < 600px, medium 600–1024px, expanded > 1024px), never by OS.
- **Touch-first**: 48px minimum targets, 64px for primary actions; keyboard shortcuts for everything.
- Light and dark themes; contrast ≥ 4.5:1; text scaling to 1.3×.
- **Localization from day one** (ARB files, Weblate for community translations).
- **Golden tests** (`alchemist`): changed screens on every PR; the **full matrix** (every screen × breakpoint × theme) nightly.

### Key screens
**Shared:** setup wizard (including recovery kit and backup) · add device · PIN login · register (per revenue center, with switcher) · modifiers · payment · receipt options · order history · refund request · shifts · device settings · Doctor

**Manage:** catalog and menus · staff and roles · taxes · revenue centers and floor plans · routes · reports (sales, X/Z, loss prevention) · review list · devices · backups · payment setup

**Restaurant and bar:** kitchen display · order board · floor plan · split check · tabs

**Retail:** product lookup with stock · returns and exchanges · stock count / receiving

---

## 12. Speed: Performance Budgets

Measured on a **reference low-end Android tablet** (~$150) and a Raspberry Pi 4/5 for `posd`. Regressions fail CI.

| Action | Budget |
|---|---|
| Tap → visual feedback | < 16 ms |
| Add item → cart and totals updated on screen | < 16 ms |
| Line change durably on disk | < 100 ms (writes are batched) |
| **Payment or order completion durably on disk before the screen confirms** | < 50 ms |
| Search or barcode lookup across 100,000 items | < 30 ms (SQLite FTS5 + indexes) |
| Open payment screen | < 100 ms |
| Receipt starts printing after payment | < 1 s |
| Cold start to PIN screen | < 2 s (from checkpoint, never full replay) |
| PIN login | < 300 ms |
| Order → kitchen display (LAN) | < 300 ms |
| Event reaches every peer (LAN) | < 500 ms |
| New device fully joined (snapshot) | < 60 s on a typical LAN |
| App memory | < 250 MB |
| App install size | < 60 MB |
| `posd` memory (Raspberry Pi) | < 100 MB |

**How:** the UI updates from in-memory state immediately; Rust persists in the background in small batches, except payments and order completion, which commit before confirming. All database, sync, crypto, networking and receipt rendering run off the UI thread. Read models are checkpointed. `criterion` benchmarks for Rust hot paths; frame-timing checks in Flutter integration tests.

---

## 13. Robustness

### Network requirements
- **Multi-device setups need a dedicated POS network**: a cheap router or a separate network with **client isolation off**. Guest and café Wi-Fi often block devices from seeing each other. The setup guide covers this and recommends specific inexpensive routers.
- The Doctor screen detects "devices can't see each other" and explains the likely cause.

### Keeping devices alive
- **Android**: the app runs a **foreground service** so the OS doesn't kill sync or printing; kiosk (lock-task) mode and keep-screen-awake for registers.
- **Desktop**: runs as a full-screen kiosk app, with optional start-on-boot.
- **iOS (after 1.0)**: Guided Access, keep-awake, and never used as a hardware host or the only always-on peer.

### Degradation matrix
| Failure | Cash sales | Card sales | Kitchen tickets | Management / reports | Recovery |
|---|---|---|---|---|---|
| Internet down | ✅ | ⚠️ only if the reader supports offline payments; otherwise cash | ✅ (LAN) | ✅ on site, ❌ remote | Automatic |
| POS network down | ✅ on every device | ❌ | ⚠️ queued, unless the device has its own printer | ✅ own data | Sync when it returns |
| Any one device dies | ✅ | ✅ | ✅ | ✅ | Data is on other peers; join a replacement |
| The only device dies | ❌ until replaced | | | | **Restore from the off-site backup** onto a new device |
| Printer down | ✅ | ✅ | ⚠️ failover route | ✅ | Reprint queue |
| Card reader down | ✅ | ✅ another reader | ✅ | ✅ | Reader pool |
| Device stolen | ✅ | ✅ | ✅ | ✅ | Owner revokes it; PIN changes prompted |
| Fire, theft of everything, ransomware | | | | | **Off-site encrypted backup** (required by setup) |
| A bug writes bad events | ✅ | ✅ | ✅ | ⚠️ review list | `Correction` events from a fixed version; never edit history |

### Crash and power safety
- Payments and completed orders are on disk before the screen confirms; line edits within 100 ms.
- Automated **"pull the plug" tests** kill processes at random points.
- Disk-full and corruption detection; any device can be wiped and rejoined.
- Watchdog restarts a frozen app.

### Time and ordering
`device_seq` orders a device's own events; hybrid logical clocks order configuration edits and tolerate wrong clocks. Large skew raises a warning.

### Updates
- Stable and beta channels; a device shows "update available" when it sees a newer peer or a release notice.
- **Never auto-update during business hours.**
- **Compatibility promise**: every release syncs with the previous minor version, so devices can be updated one at a time. Version-tolerant validation (§6) makes mixed versions safe.
- Database migrations are automatic, additive and preceded by a backup.

### Backups
- **Every multi-device business has live copies** on its peers, **plus** a scheduled encrypted off-site or USB backup, set up in the wizard.
- A verify step restores to a temporary location and checks integrity; the app nags if backups fail or are old.

### Operations
- **Doctor** screen (and `posd doctor`): disk space, backups, peers not seen recently, network isolation, clock skew, hardware status, review-list size, pending security updates.
- Device health view: last sync, pending events, version, printer and reader status.
- Redacted "export diagnostics" bundle for GitHub issues.

---

## 14. Testing and Quality

| Layer | Tooling | Covers |
|---|---|---|
| `pos-core` | `cargo test`, `proptest` | Money, tax, versioned rules, events, upcasters, permissions, modules |
| Fuzzing | `cargo-fuzz` | All deserialization and sync messages |
| Multi-peer simulation | Deterministic simulator with many in-process peers | Joining, revocation cutoffs, tamper detection, partitions and healing, **mixed versions**, leases, convergence (identical state everywhere), flood protection |
| Sync chaos | Custom harness | Network cuts, process kills, duplicated/reordered messages, clock skew: zero lost or duplicated events |
| Hardware | Simulated printers and readers + manual checklist | Routes, failover, job-ID dedup, reader claims, text vs image mode |
| Flutter widget + golden | `flutter_test`, `alchemist` | Changed screens per PR; full matrix nightly |
| End-to-end | `integration_test` / Patrol | Full flows on Android, Windows, Linux, macOS |
| Device lifecycle | Real Android devices | Foreground service survives, kiosk mode, low memory |
| Security | `cargo deny`/`audit`, `osv-scanner`, `gitleaks`, log-redaction tests | |

**Definition of done:** tests pass, goldens updated, works offline, works with the previous version on another peer, permission-checked, documented, translation keys added.

---

## 15. Countries, Fiscal Laws and Tamper-Proofing

**Goal: usable in every country.** Two layers make that possible.

### Layer 1: tamper-evident journal (every country, always on)
Already part of the core design:
- Append-only, hash-chained, signed per device, gap-free numbering, duplicate-number detection (§6).
- **Fiscal data is never deleted.** Only personal data is crypto-shredded, and personal data is never part of fiscal records (§4).
- Corrections, voids and refunds are new events; nothing is edited.
- Training mode is clearly marked and kept apart from real sales.
- **Retention** is set by the country preset (often 7–10 years). In fiscal mode, pruning of fiscal data is disabled; history lives on registers, `posd` or exported archives.
- **Standard exports** through plugins (e.g. SAF-T, DSFinV-K) for tax audits.

**Honest limit:** software on its own is tamper-*evident*, not tamper-*proof*. Anyone who controls a device can still delete the whole database. That's exactly why many countries require an external certified device, which is Layer 2.

### Layer 2: fiscal adapters
A Rust trait in `pos-fiscal`, called by `pos-core` when an order closes:
`begin`, `update`, `finish → FiscalSignature` (signature, counters, QR code and other fields the receipt must show), `daily_close`, `export(period)`, `health`, `outage_procedure`.
- The signature is stored in a `FiscalSigned` event, and the receipt template prints the required fields.
- **Fiscal devices are routed hardware** like printers (§10): a USB security module on one register, a certified cloud service, or a fiscal printer. A route can point several registers at one device where the law allows, or one per register where it doesn't.
- **Fiscal mode:** when a location's country requires fiscalization, an order can't close without a fiscal signature, except through the legally defined outage procedure, which is recorded (`FiscalOutageStarted/Ended`).

### Kinds of regimes
Rules change often; every country page in the docs must be verified against current law before it's marked supported.

| Kind | How it works | Can an open-source project support it? | Examples (verify current rules) |
|---|---|---|---|
| **A. No POS-specific fiscal law** | Normal bookkeeping and tax rules | ✅ Layer 1 is enough | US, UK, Australia, New Zealand, most of Canada |
| **B. Certified external device or service** | Each transaction is signed by a certified device or service | ✅ Yes. The certification belongs to the device; the software integrates it. The business buys and registers the device. | Germany (TSE), Austria (RKSV signature unit), Italy (RT), Sweden (control unit), Poland and Hungary (online registers / fiscal printers), Belgium (hospitality FDM), Croatia (real-time fiscalization with certificate) |
| **C. Software must be certified or declared** | A responsible party certifies or self-declares a specific software version | ⚠️ Through a **certified distribution**: a company or person in that country takes legal responsibility for a specific Tillstone build. The project makes this as easy as possible (conformance tests, documentation, reproducible builds) but can't do it itself. Apache-2.0 allows this. | France (NF525), Portugal (AT certification), Spain (VeriFactu), Norway (product declaration), Quebec (SRM) |
| **D. Real-time e-receipts or e-invoices** | Each receipt or invoice is reported to the tax authority or an authorized provider | ✅ Through adapters, usually with the business's own certificate or provider account | Brazil (NFC-e), Mexico (CFDI), Greece (myDATA) |

### Country presets
Each country (or region) is one folder in `countries/` bundling: tax presets, receipt rules, fiscal adapter and settings, retention period, export formats, and a status page in the docs:
**Supported** · **Adapter in progress** · **Available through a certified distribution** · **Not yet supported**.

### How coverage grows
- The project ships the adapter trait, a **conformance test suite**, an adapter development guide, and one **reference adapter** for a Kind B country before 1.0.
- Further adapters come from contributors and local partners, prioritized by demand.
- Peer-to-peer fits: fiscal counters and signatures come from the fiscal device, so they never conflict between peers.

### Other compliance (the business's responsibility, documented by us)
| Area | Notes |
|---|---|
| **PCI DSS** | Card data never touches Tillstone; the business completes its own provider-guided self-assessment. |
| **Sales tax / VAT** | Country presets; businesses verify their own settings. |
| **Accessibility** | Contrast, labels and text scaling (EU Accessibility Act, ADA). |
| **Privacy** | No telemetry; personal data kept out of events; crypto-shredding for deletion; data export. |
| **Warranty** | License disclaimer; "beta, use at your own risk" until 1.0. |

---

## 16. Repository Layout

```
pos/
├─ app/                    # Flutter app (all platforms, all roles)
├─ crates/
│  ├─ pos-core/            # Pricing, events, upcasters, ledger, storage, checkpoints, crypto
│  ├─ pos-core-ffi/        # flutter_rust_bridge bindings
│  ├─ pos-sync/            # Membership, discovery, replication, leases, live messages
│  ├─ pos-devices/         # Printer/drawer drivers, routes, failover, job queue
│  ├─ pos-payments/        # Provider trait + Stripe Terminal
│  ├─ pos-fiscal/          # Fiscal adapter trait, country adapters, conformance tests, exports
│  └─ posd/                # Headless always-on peer (+ optional private relay)
├─ packages/
│  └─ design_system/       # Flutter components + generated tokens
├─ tokens/                 # tokens.json + generator
├─ countries/             # Country presets: tax, receipt rules, fiscal adapter, retention, exports
├─ docs/                   # mdBook: user guide, network setup, dev guide, ADRs, threat model, country status pages
├─ .github/                # CI, issue/PR templates
├─ LICENSE · README.md · CONTRIBUTING.md · SECURITY.md · CODE_OF_CONDUCT.md · GOVERNANCE.md
└─ justfile
```

---

## 17. Running the Open-Source Project

### Releases and distribution
| Platform | Channel | Cost |
|---|---|---|
| Android | GitHub Releases (APK), F-Droid, optionally Google Play | Play: one-time developer fee |
| Windows | Signed installer on GitHub Releases, winget | Free signing for open source is available (e.g. SignPath Foundation) |
| Linux | AppImage, Flatpak (Flathub) | Free |
| macOS | Notarized DMG | Needs an Apple Developer account (~$99/year). Until then, unsigned developer builds. |
| `posd` | Static binaries, optional container image | Free |
| iOS / iPadOS | **After 1.0**, App Store / TestFlight | Same Apple account |

Funding for fees and an audit: GitHub Sponsors, Open Collective, or open-source grants (e.g. NLnet), if wanted.

### Community
- README with screenshots, a 5-minute "try it" with a **demo business and sample data**, and an honest status.
- CONTRIBUTING.md with one-command setup (`just setup`).
- Good first issues: translations, tax presets, printer profiles, design-system components.
- Stable, documented extension points: payment providers, printer drivers, fiscal modules, tax presets, report exports.
- Governance: you decide at first (GOVERNANCE.md); revisit if a core team forms.
- Semantic versioning; changelog from conventional commits.

---

## 18. Roadmap (milestones, not deadlines)

### M0: Foundations and proofs
Monorepo, CI on three OSes, license, CONTRIBUTING and SECURITY files, design tokens, first golden tests.
**Proofs:** (1) `pos-core` bridged into Flutter running on Android, Windows, Linux and macOS; (2) two devices on a LAN find each other and exchange signed messages over iroh; (3) an Android foreground service survives a day in the background on a cheap tablet.
**Exit:** all three proofs pass. If iroh fails, write an ADR choosing an alternative before M4.

### M1: Design
Mockups of shared screens, the four register layouts, kitchen display, floor plan and Manage section; states for empty, error, offline and "can't see other devices"; threat model; ADRs.
**Exit:** design system proven against every mode.

### M2: Core engine
Money and versioned pricing (with splits), order lifecycle, fiscal-ready journal (retention rules, no deletion of fiscal data, `FiscalSigned` hooks), revenue centers and modules, event formats with versioning and upcasters, hash-chained signed log, checkpoints, encrypted SQLite, permissions, personal-data separation.
**Exit:** retail, quick-service, full-service and hybrid orders simulated and verified in Rust tests.

### M3: Single register → **v0.1 (first public release)**
**One device, no networking.** Setup wizard (owner passphrase, recovery kit, off-site/USB backup), Quick Service and Retail layouts, Manage section, cash payments, network and local USB receipt printing, drawer, scanner, shifts, X/Z reports, training mode, demo data, Doctor screen.
**Exit:** a small café or shop runs a full day on one device. People can download and try it.

### M4: Multi-device peer-to-peer → v0.2
Owner authority and joining with matching codes, revocation cutoffs, tamper detection, per-role data, LAN discovery, replication with priority and flood protection, version-tolerant validation, snapshots for joining, replicated configuration, routes and failover to network printers, kitchen displays, order board, network setup guide.
**Exit:** simulator and chaos tests pass, including mixed versions; a café-in-a-bookstore runs on several devices with the internet unplugged; any one device can be destroyed without losing data.

### M5: Card payments → v0.3
Provider trait, Stripe Terminal smart readers, split keys and refund requests, reader pool with claims, every hard case in §8, split tender, on-reader tips, reconciliation.

### M6: Full Service + Bar → v0.4
Check leases, floor plan, seats and courses, send/fire, split/merge/transfer, tip adjust, tabs with pre-auth, handheld role, kitchen text-mode tickets.
**Exit:** a dinner service with a busy kitchen runs smoothly on the POS network.

### M7: Remote and always-on → v0.5
`posd`, private relay, sync between locations, archive and pruning, owner laptop/phone at home.

### M8: Hardening → v1.0
Security audit, pilots (a shop, a quick-service spot, a restaurant, a hybrid), signed security notices, fiscal adapter framework with conformance tests and one reference Kind B adapter, first export formats, complete docs, performance budgets met on reference devices, several translations.
**Exit:** 30 days of live use at pilot sites with no lost transactions.

### After 1.0
More country adapters and certified distributions, iPad/iPhone, hosted USB/Bluetooth peripherals, customer display, customers and loyalty, gift cards, purchase orders and stock transfers, recipe-based inventory, online ordering, kiosk, reservations, time clock, more payment providers, scales, read-only web dashboard.

---

## 19. Risks

| Risk | Goal at risk | Mitigation |
|---|---|---|
| Huge scope for a volunteer project | Simple | v0.1 is one device without networking; one big feature per milestone; iOS and hosted peripherals after 1.0 |
| iroh immaturity or API churn | Robust | M0 proof; pin versions; keep it behind the `pos-sync` interface so it can be replaced |
| Rust + Flutter build complexity (also for contributors) | Simple | M0 proof on every platform; small generated bridge; one-command setup |
| Networks that block devices from seeing each other | Robust | Dedicated POS network requirement, router recommendations, Doctor detection |
| OS killing or suspending the app | Robust | Android foreground service, kiosk mode, M0 proof; no iOS hosts |
| Mixed versions disagreeing | Robust | Version-tolerant validation, versioned pricing rules, mixed-version simulation tests |
| Payment edge cases (double charges, lost sales) | Robust | Events before payments, idempotency, leases, reader claims, reconciliation |
| Stolen or rooted device | Secure | Owner-only membership, revocation cutoffs, per-role data, no refunds key on registers, PIN changes |
| Faked manager approvals from a modified device | Secure | Detection: review list, outlier reports, flood protection (documented limit) |
| Personal data in permanent logs | Secure | Kept out of events; crypto-shredding |
| Unpatched self-hosted installs | Secure | Signed security notices, owner banners |
| A bug spreading bad data to every peer | Robust | Hard-reject only on integrity failures, warnings and review list, flood protection, `Correction` events |
| Slow durable writes on cheap storage | Fast | Separate screen and durability budgets; batched writes; payments committed first |
| Long history slowing startup or joining | Fast | Checkpoints and snapshots |
| Event formats becoming a burden over years | Simple | Versioning, upcasters, never changing meanings, ADR per new event type |
| Fiscal laws vary and change | Robust | Adapter trait + conformance tests; country pages verified before "Supported"; certified distributions for Kind C countries |
| Maintainer burnout | All | ADRs, tests, docs, welcoming co-maintainers early |

---

## 20. Decisions and Next Steps

**Decided:** name **Tillstone**, license **Apache-2.0**, every country supported through §15, conventional commits.

**Still to decide:** the **first launch region**, which picks the first country preset and reference fiscal adapter.

**Next:**
1. Write ADR-001 to ADR-011: Flutter, Rust core, SQLite everywhere, event sourcing with versioned events, self-hosted single business, peer-to-peer with owner authority, version-tolerant validation, routes and failover, revenue centers and modules, payment provider plugins with split keys, fiscal adapters.
2. Create the design tokens and mock up the Quick Service and Retail register layouts side by side.
3. Set up the Cargo and Flutter workspaces and run the three M0 proofs.
