# 0002. Flutter UI with a Rust core, bridged by flutter_rust_bridge

- **Status:** Accepted
- **Date:** 2026-11-11

## Context

Tillstone must look identical on Android, Windows, Linux and macOS (and iOS later), and the code that handles money, records, sync and encryption must be as hard to get wrong as possible. That code also needs to run without a UI in `posd`.

## Decision

- **Flutter** draws every pixel of the UI from Tillstone's own design system, with platform-specific behavior pinned (PLAN §11). Material comes from the `material_ui` package, which replaced `package:flutter/material.dart` in Flutter 3.47.
- **Rust** holds all business logic, storage, sync, crypto and hardware drivers, in the `crates/` workspace.
- **flutter_rust_bridge v2** generates the bindings. The bridge crate (`crates/pos-core-ffi`, library `tillstone_ffi`) only converts types and forwards calls; it never contains business logic. It is the only crate allowed `unsafe` (generated glue).
- The app builds the Rust library through cargokit (`app/rust_builder`), pointing at the workspace crate.

## Alternatives considered

- **Pure Dart core:** one language fewer, but no shared validation code with `posd`, and weaker guarantees for money and crypto code.
- **Kotlin Multiplatform, React Native, .NET MAUI, Tauri:** see PLAN §1; none give identical rendering on every target as cheaply.

## Consequences

- M0 proof: the bridge works on macOS (integration test passes locally). CI runs the same proof on Linux and Windows and builds Android.
- macOS links Rust against the `SystemConfiguration`, `Security` and `CoreFoundation` frameworks (declared in the podspec), and needs CocoaPods.
- Contributors need both Rust and Flutter installed; `CONTRIBUTING.md` documents setup.
