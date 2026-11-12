# Contributing to Tillstone

Thanks for your interest. Tillstone is in the planning stage, so the most useful contributions right now are feedback on the [plan](docs/PLAN.md), and knowledge of local tax and fiscal rules for the country pages.

## Ground rules

- Be kind. Everyone here follows the [Code of Conduct](CODE_OF_CONDUCT.md).
- **Security issues go through [SECURITY.md](SECURITY.md)**, never public issues.
- For anything bigger than a small fix, open an issue first so we can agree on the approach.
- Significant design decisions get a decision record in [`docs/adr/`](docs/adr/).

## Development setup

You need:
- [Rust](https://rustup.rs) (stable)
- [Flutter](https://docs.flutter.dev/get-started/install) 3.50 or newer
- For macOS builds: Xcode and CocoaPods. For Android: the Android SDK and NDK (Android Studio installs them). For Linux desktop: `clang cmake ninja-build pkg-config libgtk-3-dev`.
- Optional: [`just`](https://just.systems) for the shortcuts in the `justfile`, and `flutter_rust_bridge_codegen` (`cargo install flutter_rust_bridge_codegen --version 2.13.0`) if you change the Rust API used by the app.

Then:

```sh
just setup          # or run the commands in the justfile by hand
just check          # Rust and design-system checks
just proof macos    # run the app proof on your platform
```

## Commit messages

Tillstone uses [Conventional Commits](https://www.conventionalcommits.org/en/v1.0.0/). Every commit message must start with a type, an optional scope, and a short summary:

```
<type>(<optional scope>): <summary>
```

| Type | Use for |
|---|---|
| `feat` | A new feature |
| `fix` | A bug fix |
| `docs` | Documentation only |
| `style` | Formatting, no behavior change |
| `refactor` | Code change that neither fixes a bug nor adds a feature |
| `perf` | Performance improvement |
| `test` | Adding or fixing tests |
| `build` | Build system or dependencies |
| `ci` | CI configuration |
| `chore` | Maintenance that doesn't fit the above |
| `revert` | Reverting a previous commit |

Mark breaking changes with `!` after the type or scope (`feat(sync)!: ...`) and explain them in a `BREAKING CHANGE:` footer.

Examples:

```
feat(pricing): support compound tax rules
fix(sync): resend events after a dropped connection
docs(countries): add status page for Austria
```

To check your messages locally, enable the repository's hooks once:

```sh
git config core.hooksPath .githooks
```

Pull requests are checked automatically, too.

## Licensing of contributions

By contributing, you agree that your contributions are licensed under the [Apache License 2.0](LICENSE), as described in section 5 of the license. There is no separate contributor agreement.
