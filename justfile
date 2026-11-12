# Common tasks. Install `just` (https://just.systems), then run `just <task>`.

# Fetch dependencies and enable the commit message hook
setup:
    git config core.hooksPath .githooks
    cd packages/design_system && flutter pub get
    cd app && flutter pub get

# Run every check CI runs, except the per-platform app proofs
check: rust design

# Format, lint and test the Rust crates
rust:
    cargo fmt -p pos-core -p pos-sync -p pos-store --check
    cargo clippy -p pos-core -p pos-sync -p pos-store --all-targets -- -D warnings
    cargo test -p pos-core -p pos-sync -p pos-store

# Check tokens, analyze and test the design system (goldens are macOS-rendered)
design:
    cd packages/design_system && dart run tool/generate_tokens.dart --check
    cd packages/design_system && flutter analyze
    cd packages/design_system && flutter test

# Regenerate design tokens after editing tokens/tokens.json
tokens:
    cd packages/design_system && dart run tool/generate_tokens.dart

# Regenerate the Rust <-> Dart bindings after changing crates/pos-core-ffi/src/api
bridge:
    cd app && flutter_rust_bridge_codegen generate

# Run the M0 proof on a platform: just proof macos | windows | linux | <android-device-id>
proof platform:
    cd app && flutter test integration_test -d {{platform}}
