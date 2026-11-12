# 0005. Built to be forked

- **Status:** Accepted
- **Date:** 2026-11-12

## Context

Tillstone is free software with no company behind it. A business that
needs something different (its own brand, menu presets, a local payment
provider, a special report) should be able to **fork the repository and
tailor it**, then keep pulling in upstream fixes and security updates
without painful merges.

Forks pull against the security and robustness goals: a fork must not be
able to break the permanent record by accident, and two forks' devices must
never misread each other's data.

## Decision

Design every feature so the common customizations are **data or clearly
marked extension points**, kept apart from upstream code:

| Customization | Where | Code change? |
|---|---|---|
| Product name | `tokens/tokens.json` → `brand.name` | No |
| Colors, spacing, type | `tokens/tokens.json` | No |
| Business templates and the demo | `templates/*.json` (add a file) | No |
| Currencies offered at setup | `template::CURRENCIES` | One table |
| Country tax and receipt rules | `countries/<code>/` | Data first, adapter if needed |
| Payment providers, printers, fiscal adapters | Rust traits (`pos-payments`, `pos-devices`, `pos-fiscal`) | A new module |
| Fork-only events | Event kinds `0x8000–0xFFFF` (`events::FORK_KINDS`) | A new event |

Rules that protect the record:

- **Upstream never uses event kinds `0x8000` and above.** Forks use them for
  their own events, so upstream additions can't collide with a fork's.
  Devices that don't know a kind store and forward it unchanged.
- Upstream event numbers, payloads and the wire fixture stay frozen
  (ADR 0004), so a fork's devices and upstream devices read each other's
  records.
- Templates, tokens and country data are checked by tests at build time,
  so a broken file fails CI instead of a register.

## Alternatives considered

- **A runtime plugin system (dynamic libraries or scripts).** Most
  flexible, but it adds a sandbox, a stable binary interface and a second
  security boundary. Too much for 1.0; forks plus compile-time traits cover
  the need. We can revisit after 1.0.
- **Everything configurable at runtime in Manage.** Good for settings a
  business changes often (items, staff, taxes) and those live there. Brand
  and templates change rarely and are better as reviewed files in a fork.

## Consequences

- Upstream must keep extension points stable and documented
  (`docs/customizing.md`), and resist hard-coding names, prices or
  business types in code.
- Forks that rename the app should also change the app identifiers
  (bundle id, Android application id) so both can be installed, as
  `docs/customizing.md` explains.
- A fork that changes core rules (pricing, the ledger) takes on the merge
  cost for those files; the docs say so plainly.
