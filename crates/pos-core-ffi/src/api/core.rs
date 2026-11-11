//! Functions the Flutter app calls. Keep this layer thin: it converts types
//! and forwards to `pos-core` / `pos-sync`, nothing more (PLAN §1).

use std::time::{Duration, Instant};

use pos_core::{Currency, Money};
use pos_sync::{DeviceIdentity, Members, Peer, PeerOptions};

#[flutter_rust_bridge::frb(init)]
pub fn init_app() {
    flutter_rust_bridge::setup_default_user_utils();
}

/// Version of the Rust core.
#[flutter_rust_bridge::frb(sync)]
pub fn core_version() -> String {
    pos_core::CORE_VERSION.to_string()
}

/// Splits an amount (in cents) evenly, formatted for display. Proves money
/// math runs in Rust on every platform.
#[flutter_rust_bridge::frb(sync)]
pub fn split_evenly(amount_minor: i64, parts: u32) -> Result<Vec<String>, String> {
    Money::from_minor(amount_minor, Currency::USD)
        .split_evenly(parts as usize)
        .map(|v| v.iter().map(ToString::to_string).collect())
        .map_err(|e| e.to_string())
}

/// Result of the on-device peer-to-peer self-test.
pub struct PeerSelfTest {
    pub sender: String,
    pub receiver: String,
    pub message: String,
    pub round_trip_ms: u64,
    pub outsider_refused: bool,
}

/// Starts two peers inside the app, sends a signed message between them over
/// an encrypted iroh connection, and checks a non-member is refused.
pub async fn peer_self_test() -> Result<PeerSelfTest, String> {
    run_self_test().await.map_err(|e| format!("{e:#}"))
}

async fn run_self_test() -> anyhow::Result<PeerSelfTest> {
    let (a_id, b_id, outsider_id) =
        (DeviceIdentity::generate(), DeviceIdentity::generate(), DeviceIdentity::generate());
    let members = Members::default();
    members.add(a_id.device_id());
    members.add(b_id.device_id());

    let (a_tx, _a_rx) = tokio::sync::mpsc::channel(4);
    let (b_tx, mut b_rx) = tokio::sync::mpsc::channel(4);
    let a = Peer::bind(a_id.clone(), members.clone(), PeerOptions::LOCAL_ONLY, a_tx).await?;
    let b = Peer::bind(b_id.clone(), members.clone(), PeerOptions::LOCAL_ONLY, b_tx).await?;

    let started = Instant::now();
    a.send(b.direct_addr(), b"order A-0001 closed").await?;
    let received = tokio::time::timeout(Duration::from_secs(5), b_rx.recv())
        .await?
        .ok_or_else(|| anyhow::anyhow!("no message received"))?;
    let round_trip_ms = started.elapsed().as_millis() as u64;

    let outsider_members = Members::default();
    outsider_members.add(b_id.device_id());
    let (o_tx, _o_rx) = tokio::sync::mpsc::channel(1);
    let outsider = Peer::bind(outsider_id, outsider_members, PeerOptions::LOCAL_ONLY, o_tx).await?;
    let outsider_refused = outsider.send(b.direct_addr(), b"open drawer").await.is_err();

    a.shutdown().await;
    b.shutdown().await;
    outsider.shutdown().await;

    Ok(PeerSelfTest {
        sender: a_id.device_id().fmt_short().to_string(),
        receiver: b_id.device_id().fmt_short().to_string(),
        message: String::from_utf8_lossy(&received.payload).into_owned(),
        round_trip_ms,
        outsider_refused,
    })
}
