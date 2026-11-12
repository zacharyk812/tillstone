# M0 proofs

Milestone M0 (PLAN §18) has three proofs. This page explains how to run each one and records the results.

## 1. The app and Rust core run on every platform

The integration test in `app/integration_test/proof_test.dart` starts the real app and checks that:
- the Rust core loads,
- money math runs in Rust (10.00 USD split 3 ways = 3.34 + 3.33 + 3.33),
- two peers inside the app exchange a signed message over an encrypted iroh connection, and an outsider is refused.

```sh
cd app
flutter test integration_test -d macos     # or: -d windows, -d linux, or an Android device id
```

| Platform | Result |
|---|---|
| macOS | ✅ passes locally |
| Linux | ✅ passes in CI (after the backtrace fix in ADR 0002) |
| Windows | ✅ passes in CI |
| Android | ✅ APK builds in CI; run the test on a device as below |

## 2. Two devices find each other and exchange signed messages

Automated tests cover this in-process (`cargo test -p pos-sync`). To try it across two real machines on the same network:

```sh
# On machine A
cargo run -p pos-sync --example lan -- id a.key
# On machine B
cargo run -p pos-sync --example lan -- id b.key
# On A: wait for messages from B
cargo run -p pos-sync --example lan -- listen a.key <B's id>
# On B: send to A
cargo run -p pos-sync --example lan -- send b.key <A's id> "hello"
```

Expected: B prints `delivered and acknowledged`; A prints `verified message from …: hello`. Neither side needs the other's IP address. If it times out, the network probably blocks devices from seeing each other (client isolation); see PLAN §13.

## 3. Android keeps the app alive in the background

The app starts a foreground service (`KeepAliveService`) that records a heartbeat every minute, and holds a Wi-Fi multicast lock for local discovery.

To run the proof on a cheap Android tablet:
1. Install the debug build: `cd app && flutter run -d <device>`.
2. Allow notifications when asked, then press Home so the app is in the background.
3. Leave the tablet plugged in, screen off, for 24 hours.
4. Open the app. The proof screen shows the number of heartbeats and the **largest gap** between them.

**Pass:** around 1,440 heartbeats and a largest gap under 3 minutes. A bigger gap means Android paused or killed the app. Note the device model, Android version and any battery-saver settings in the results.

| Device | Android | Result |
|---|---|---|
| _not run yet_ | | |
