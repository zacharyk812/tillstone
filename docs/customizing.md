# Customizing Tillstone for your business

Tillstone is meant to be forked. Fork the repository, change what you
need, and keep pulling upstream updates. This guide lists the supported
places to customize, from easiest to hardest. Why it's built this way is
in [ADR 0005](adr/0005-built-to-be-forked.md).

> **Rule of thumb:** put your changes in *new files* where you can. Edit
> upstream files only where this guide says to. That keeps merges easy.

## 1. Name and look (no code)

Edit [`tokens/tokens.json`](../tokens/tokens.json):

- `brand.name`: the product name on screens and in the window title.
- `color.light` and `color.dark`: every color the app uses. Keep text
  contrast at 4.5:1 or more.
- `spacing`, `radius`, `type`: spacing, corners and type sizes.

Then run `just tokens` and commit the regenerated `tokens.g.dart`.

If you ship your build alongside upstream Tillstone, also change the app
identifiers so they don't replace each other:

| Platform | File | Setting |
|---|---|---|
| Android | `app/android/app/build.gradle.kts` | `applicationId` |
| macOS | `app/macos/Runner/Configs/AppInfo.xcconfig` | `PRODUCT_BUNDLE_IDENTIFIER`, `PRODUCT_NAME` |
| Windows | `app/windows/runner/Runner.rc` | product and company names |
| Linux | `app/linux/CMakeLists.txt` | `APPLICATION_ID` |

## 2. Business templates and the demo (no code)

The setup wizard offers one choice per file in [`templates/`](../templates).
Add `templates/my-bakery.json` and it appears on the next build. The format
is in [`templates/README.md`](../templates/README.md). Edit
`templates/demo.json` to change the "Try the demo" business.

`cargo test -p pos-app` checks every template, and sets each one up in a
test register, so mistakes show up in CI.

## 3. Countries, taxes and receipts (data first)

Country presets live in [`countries/`](../countries). Tax presets and
receipt rules are data; a fiscal adapter is needed only where the law
requires a certified register (PLAN §15).

## 4. Hardware and payments (a Rust module)

Printers, cash drawers, card readers and fiscal devices sit behind Rust
traits. Add a driver or provider as a new module that implements the
trait, and register it in one place. These crates arrive with milestones
M3–M5 (PLAN §18); this section will link the traits as they land.

## 5. Your own events (advanced)

Everything that happens is recorded as a signed event (ADR 0004). If your
fork needs to record something upstream doesn't have:

1. Pick an event number from **`0x8000` to `0xFFFF`**
   (`pos_core::events::FORK_KINDS`). Upstream never uses these, so your
   events never collide with future upstream ones.
2. Add the variant to the `events!` table in `crates/pos-core/src/events.rs`
   and a payload struct, following the rules at the top of that file.
3. Add a sample to the wire-format test.

Devices running upstream Tillstone keep and forward your events without
understanding them; they never drop or rewrite them.

**Never** change the number, name or payload of an existing upstream event.
Old records must stay readable forever.

## 6. Changing core rules

You can change anything: it's Apache-2.0. But pricing, tax, the ledger and
sync (`crates/pos-core`, `crates/pos-sync`) are where upstream changes most
carefully, and where your edits will cost the most to merge. If you need a
rule change there, consider opening an issue first; it may belong upstream.

## Keeping up with upstream

```sh
git remote add upstream https://github.com/zacharyk812/tillstone.git
git fetch upstream
git merge upstream/main      # or rebase your branch onto it
just check                   # run everything CI runs
```

Security fixes are announced in signed release notes (SECURITY.md).
