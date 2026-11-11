# 0003. iroh for peer-to-peer networking

- **Status:** Accepted
- **Date:** 2026-11-11

## Context

Devices sync directly with each other (PLAN §3, §6). We need: devices identified by public key, end-to-end encryption with mutual authentication, discovery on the local network, and a way through NATs between sites, without running our own service.

## Decision

Use **iroh 1.4** (QUIC) with **iroh-mdns-address-lookup** for local discovery, wrapped by `crates/pos-sync` so the rest of Tillstone never depends on iroh directly.

- One Ed25519 key per device both authenticates the encrypted connection and signs messages.
- Messages are signed with a domain-separation prefix, and the receiver checks that the signer is the device it is connected to.
- Peers refuse connections from devices that aren't members.
- Relays are off by default; they're only used for multi-site setups (M7), ideally a private relay on `posd`.

## M0 results

| Proof | Result |
|---|---|
| Two in-process peers exchange signed, acknowledged messages | ✅ `crates/pos-sync/tests/two_peers.rs` |
| Non-members and revoked devices are refused | ✅ same tests |
| Tampered, forged, truncated and oversized messages rejected | ✅ unit tests |
| Two separate processes find each other by public key via mDNS and exchange a message | ✅ `examples/lan.rs`, same machine |
| Runs inside the app on a real platform | ✅ macOS integration test |
| Two physical devices on a real network | ⏳ manual check, see `docs/m0-proofs.md` |

iroh reached 1.x and is stable enough to commit to.

## Alternatives considered

- **Plain QUIC (`quinn`) plus our own discovery and NAT traversal:** more code to write and secure.
- **libp2p:** more general and much larger.
- **Tailscale/WireGuard as a requirement:** an extra product for every business to install.

## Consequences

- Android needs a Wi-Fi multicast lock for mDNS, held by the background service.
- Networks with client isolation block discovery (PLAN §13), so multi-device setups need a dedicated POS network.
