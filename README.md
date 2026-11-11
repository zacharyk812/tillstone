# Tillstone

**A free, open-source point of sale for shops, cafés, restaurants, bars, and everything in between.**

Tillstone is designed to be secure, fast, robust and simple. It runs the same on every platform, works fully offline, and needs no server: start with one register, add another, and they find each other and sync.

> **Status: planning.** There's no usable software yet. The design is in [`docs/PLAN.md`](docs/PLAN.md), and the work is tracked as milestones there. Until 1.0, treat everything as beta and don't run a real business on it.

## Goals

- **One product for every kind of business.** Retail, quick service, full-service restaurants, bars, and hybrids like a café inside a bookstore. Businesses switch features on per selling area.
- **Peer-to-peer.** Every device holds a copy of the data. There's no master and no cloud service run by us. An optional always-on device can bridge locations and run backups.
- **Offline-first.** Every device keeps selling with no network at all.
- **Tamper-evident by design.** Every sale is a permanent, signed, hash-chained record. Corrections are new records; history is never edited.
- **Every country.** Per-country fiscal adapters integrate the certified devices and services that some countries legally require (see [§15 of the plan](docs/PLAN.md#15-countries-fiscal-laws-and-tamper-proofing)).
- **Never touches card data.** Card payments go through certified smart card readers from a payment provider the business chooses.
- **Identical UI everywhere.** Android, Windows, Linux and macOS through 1.0; iPad and iPhone after.
- **Private.** No telemetry. Businesses own all of their data.

## Technology

| Part | Choice |
|---|---|
| UI | Flutter |
| Core (money, tax, records, sync, encryption, hardware) | Rust |
| Storage | SQLite with SQLCipher encryption |
| Peer-to-peer networking | iroh |
| First payment provider | Stripe Terminal smart readers |

The reasoning for each choice is in the [plan](docs/PLAN.md) and in [decision records](docs/adr/).

## Contributing

Contributions are welcome once the foundations are in place. Read [CONTRIBUTING.md](CONTRIBUTING.md) first. This project uses [Conventional Commits](https://www.conventionalcommits.org/).

## Security

Please report vulnerabilities privately as described in [SECURITY.md](SECURITY.md). Don't open public issues for them.

## License

[Apache License 2.0](LICENSE).
