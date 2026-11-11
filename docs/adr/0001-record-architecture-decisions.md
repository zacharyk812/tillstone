# 0001. Record architecture decisions

- **Status:** Accepted
- **Date:** 2026-11-11

## Context

Tillstone is a volunteer, open-source project that handles money and legally important records. Contributors come and go, and the reasoning behind a design is easily lost. Choices about security, data formats and sync are especially hard to undo later.

## Decision

Every significant design decision is recorded as a numbered decision record in `docs/adr/`, using the template in `0000-template.md`. Records are never deleted; a changed decision gets a new record that supersedes the old one.

A decision is significant if it changes a data format, the sync protocol, a security property, a public extension point, a supported platform, or adds a major dependency.

## Alternatives considered

- **Decisions only in issues and pull requests:** hard to find later, and scattered.
- **Only the master plan:** the plan says *what*, but gets rewritten; records keep *why*, permanently.

## Consequences

- Newcomers can understand why things are the way they are.
- A little extra writing for each significant change.
- The first records to write are listed in [the plan, §20](../PLAN.md#20-decisions-and-next-steps).
