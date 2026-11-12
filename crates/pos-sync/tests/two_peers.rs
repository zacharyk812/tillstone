//! M0 proof: two devices exchange signed messages, and outsiders are refused.

use std::time::Duration;

use pos_sync::{DeviceIdentity, Members, Peer, PeerOptions, Received};
use tokio::sync::mpsc;

async fn peer(identity: DeviceIdentity, members: &Members) -> (Peer, mpsc::Receiver<Received>) {
    let (tx, rx) = mpsc::channel(16);
    let peer = Peer::bind(identity, members.clone(), PeerOptions::LOCAL_ONLY, tx).await.unwrap();
    (peer, rx)
}

#[tokio::test]
async fn members_exchange_signed_messages() {
    let (a_id, b_id) = (DeviceIdentity::generate(), DeviceIdentity::generate());
    let members = Members::default();
    members.add(a_id.device_id());
    members.add(b_id.device_id());

    let (a, mut a_inbox) = peer(a_id.clone(), &members).await;
    let (b, mut b_inbox) = peer(b_id.clone(), &members).await;

    a.send(b.direct_addr(), b"order A-0001 closed").await.unwrap();
    let got = tokio::time::timeout(Duration::from_secs(5), b_inbox.recv()).await.unwrap().unwrap();
    assert_eq!(got.from, a_id.device_id());
    assert_eq!(got.payload, b"order A-0001 closed");

    b.send(a.direct_addr(), b"ticket bumped").await.unwrap();
    let got = tokio::time::timeout(Duration::from_secs(5), a_inbox.recv()).await.unwrap().unwrap();
    assert_eq!(got.from, b_id.device_id());

    a.shutdown().await;
    b.shutdown().await;
}

#[tokio::test]
async fn non_members_are_refused() {
    let (a_id, outsider_id) = (DeviceIdentity::generate(), DeviceIdentity::generate());

    // A only trusts itself.
    let a_members = Members::default();
    a_members.add(a_id.device_id());
    let (a, mut a_inbox) = peer(a_id.clone(), &a_members).await;

    // The outsider believes A is a member, and tries anyway.
    let outsider_members = Members::default();
    outsider_members.add(a_id.device_id());
    let (outsider, _) = peer(outsider_id, &outsider_members).await;

    assert!(outsider.send(a.direct_addr(), b"open drawer").await.is_err());
    assert!(
        tokio::time::timeout(Duration::from_millis(500), a_inbox.recv()).await.is_err(),
        "a non-member's message must never be delivered"
    );

    a.shutdown().await;
    outsider.shutdown().await;
}

#[tokio::test]
async fn revoked_devices_are_refused() {
    let (a_id, b_id) = (DeviceIdentity::generate(), DeviceIdentity::generate());
    let members = Members::default();
    members.add(a_id.device_id());
    members.add(b_id.device_id());

    let a_members = members.clone();
    let (a, mut a_inbox) = peer(a_id.clone(), &a_members).await;
    // B keeps its own view of membership, which still includes A.
    let b_members = Members::default();
    b_members.add(a_id.device_id());
    b_members.add(b_id.device_id());
    let (b, _) = peer(b_id.clone(), &b_members).await;

    b.send(a.direct_addr(), b"before revocation").await.unwrap();
    assert!(a_inbox.recv().await.is_some());

    a_members.remove(&b_id.device_id());
    assert!(b.send(a.direct_addr(), b"after revocation").await.is_err());

    a.shutdown().await;
    b.shutdown().await;
}
