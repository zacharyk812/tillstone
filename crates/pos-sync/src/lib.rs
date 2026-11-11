//! Peer-to-peer messaging between Tillstone devices.
//!
//! This is the M0 proof from `docs/PLAN.md` §18: devices find each other on the
//! local network and exchange **signed** messages over end-to-end encrypted
//! QUIC connections (iroh), and devices that aren't members of the business
//! are refused.
//!
//! It is intentionally small. Log replication, membership events and leases
//! (§6) build on top of this in M4.

use std::{
    collections::HashSet,
    sync::{Arc, RwLock},
    time::Duration,
};

use anyhow::{Context, Result, bail, ensure};
use iroh::{
    Endpoint, EndpointAddr, EndpointId, PublicKey, RelayMode, SecretKey, Signature,
    endpoint::{Connection, presets},
};
use iroh_mdns_address_lookup::MdnsAddressLookup;

/// Protocol identifier negotiated during the connection handshake.
pub const ALPN: &[u8] = b"tillstone/proof/1";

/// Domain-separation prefixes, so a signature made for one purpose can never
/// be replayed as another.
const MESSAGE_DOMAIN: &[u8] = b"tillstone/message/v1\0";
const ACK_DOMAIN: &[u8] = b"tillstone/ack/v1\0";

/// Largest message accepted, to bound memory use from a misbehaving peer.
pub const MAX_PAYLOAD: usize = 64 * 1024;

/// QUIC application close code sent to devices that aren't members.
pub const CLOSE_NOT_A_MEMBER: u32 = 403;

/// A device's long-lived identity. The same Ed25519 key authenticates the
/// encrypted connection and signs messages.
#[derive(Clone)]
pub struct DeviceIdentity {
    secret: SecretKey,
}

impl DeviceIdentity {
    pub fn generate() -> Self {
        Self { secret: SecretKey::generate() }
    }

    pub fn from_secret_bytes(bytes: &[u8; 32]) -> Self {
        Self { secret: SecretKey::from_bytes(bytes) }
    }

    pub fn device_id(&self) -> PublicKey {
        self.secret.public()
    }

    /// The raw secret key, for storing in the platform keystore.
    pub fn secret_bytes(&self) -> [u8; 32] {
        self.secret.to_bytes()
    }

    pub fn sign(&self, payload: &[u8]) -> SignedMessage {
        let signature = self.secret.sign(&domain(MESSAGE_DOMAIN, payload));
        SignedMessage { signer: self.device_id(), payload: payload.to_vec(), signature }
    }
}

/// A payload plus the signer's public key and Ed25519 signature.
///
/// Wire format: `signer (32) | signature (64) | payload length (u32 BE) | payload`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SignedMessage {
    pub signer: PublicKey,
    pub payload: Vec<u8>,
    pub signature: Signature,
}

impl SignedMessage {
    pub fn verify(&self) -> Result<()> {
        self.signer
            .verify(&domain(MESSAGE_DOMAIN, &self.payload), &self.signature)
            .context("invalid message signature")
    }

    pub fn encode(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(32 + 64 + 4 + self.payload.len());
        out.extend_from_slice(self.signer.as_bytes());
        out.extend_from_slice(&self.signature.to_bytes());
        out.extend_from_slice(&(self.payload.len() as u32).to_be_bytes());
        out.extend_from_slice(&self.payload);
        out
    }

    pub fn decode(bytes: &[u8]) -> Result<Self> {
        ensure!(bytes.len() >= 100, "message too short");
        let signer = PublicKey::from_bytes(bytes[0..32].try_into()?).context("bad signer key")?;
        let signature = Signature::from_bytes(bytes[32..96].try_into()?);
        let len = u32::from_be_bytes(bytes[96..100].try_into()?) as usize;
        ensure!(len <= MAX_PAYLOAD, "payload too large");
        ensure!(bytes.len() == 100 + len, "length mismatch");
        Ok(Self { signer, payload: bytes[100..].to_vec(), signature })
    }
}

fn domain(prefix: &[u8], payload: &[u8]) -> Vec<u8> {
    [prefix, payload].concat()
}

/// The set of devices allowed to talk to this one. In M4 this is derived from
/// signed `DeviceAdded` / `DeviceRevoked` events; here it's a plain set.
#[derive(Clone, Default)]
pub struct Members(Arc<RwLock<HashSet<PublicKey>>>);

impl Members {
    pub fn add(&self, id: PublicKey) {
        self.0.write().expect("members lock").insert(id);
    }

    pub fn remove(&self, id: &PublicKey) {
        self.0.write().expect("members lock").remove(id);
    }

    pub fn contains(&self, id: &PublicKey) -> bool {
        self.0.read().expect("members lock").contains(id)
    }
}

/// How a peer finds and reaches other peers.
#[derive(Debug, Clone, Copy)]
pub struct PeerOptions {
    /// Advertise and discover peers on the local network (mDNS).
    pub local_discovery: bool,
    /// Use encrypted relays when a direct connection isn't possible.
    pub relays: bool,
}

impl PeerOptions {
    /// Local network only: no relays, no internet lookups. Used in tests.
    pub const LOCAL_ONLY: Self = Self { local_discovery: false, relays: false };
    /// Same local network, discovered automatically.
    pub const LAN: Self = Self { local_discovery: true, relays: false };
}

/// A received, verified message.
#[derive(Debug, Clone)]
pub struct Received {
    pub from: PublicKey,
    pub payload: Vec<u8>,
}

/// A running Tillstone peer.
pub struct Peer {
    identity: DeviceIdentity,
    endpoint: Endpoint,
    members: Members,
}

impl Peer {
    /// Binds a peer and starts accepting messages from members. Every verified
    /// message is delivered to `inbox`.
    pub async fn bind(
        identity: DeviceIdentity,
        members: Members,
        options: PeerOptions,
        inbox: tokio::sync::mpsc::Sender<Received>,
    ) -> Result<Self> {
        let preset = if options.relays { RelayMode::Default } else { RelayMode::Disabled };
        let endpoint = Endpoint::builder(presets::Minimal)
            .secret_key(identity.secret.clone())
            .relay_mode(preset)
            .alpns(vec![ALPN.to_vec()])
            .bind()
            .await
            .context("binding endpoint")?;

        if options.local_discovery {
            let mdns = MdnsAddressLookup::builder()
                .service_name("tillstone")
                .build(endpoint.id())
                .context("starting local discovery")?;
            endpoint.address_lookup()?.add(mdns);
        }

        tokio::spawn(accept_loop(endpoint.clone(), identity.clone(), members.clone(), inbox));
        Ok(Self { identity, endpoint, members })
    }

    pub fn device_id(&self) -> EndpointId {
        self.endpoint.id()
    }

    pub fn members(&self) -> &Members {
        &self.members
    }

    /// Address including the local sockets, for connecting without discovery.
    pub fn direct_addr(&self) -> EndpointAddr {
        let mut addr = EndpointAddr::new(self.endpoint.id());
        for socket in self.endpoint.bound_sockets() {
            let ip = if socket.ip().is_unspecified() {
                match socket {
                    std::net::SocketAddr::V4(_) => std::net::Ipv4Addr::LOCALHOST.into(),
                    std::net::SocketAddr::V6(_) => std::net::Ipv6Addr::LOCALHOST.into(),
                }
            } else {
                socket.ip()
            };
            addr = addr.with_ip_addr(std::net::SocketAddr::new(ip, socket.port()));
        }
        addr
    }

    /// Signs `payload`, sends it to `to`, and waits for the recipient's signed
    /// acknowledgement.
    pub async fn send(&self, to: impl Into<EndpointAddr>, payload: &[u8]) -> Result<()> {
        ensure!(payload.len() <= MAX_PAYLOAD, "payload too large");
        let to = to.into();
        let recipient = to.id;
        ensure!(self.members.contains(&recipient), "recipient is not a member");

        let conn = tokio::time::timeout(Duration::from_secs(10), self.endpoint.connect(to, ALPN))
            .await
            .context("connection timed out")?
            .context("connecting")?;
        // The encrypted handshake already proved the remote holds this key.
        ensure!(conn.remote_id() == recipient, "connected to the wrong peer");

        let (mut send, mut recv) = conn.open_bi().await?;
        send.write_all(&self.identity.sign(payload).encode()).await?;
        send.finish()?;

        let ack = recv.read_to_end(Signature::LENGTH).await.context("no acknowledgement")?;
        let ack = Signature::from_bytes(ack.as_slice().try_into().context("bad acknowledgement")?);
        recipient
            .verify(&domain(ACK_DOMAIN, payload), &ack)
            .context("acknowledgement signature invalid")?;

        conn.close(0u32.into(), b"done");
        Ok(())
    }

    pub async fn shutdown(self) {
        self.endpoint.close().await;
    }
}

async fn accept_loop(
    endpoint: Endpoint,
    identity: DeviceIdentity,
    members: Members,
    inbox: tokio::sync::mpsc::Sender<Received>,
) {
    while let Some(incoming) = endpoint.accept().await {
        let (identity, members, inbox) = (identity.clone(), members.clone(), inbox.clone());
        tokio::spawn(async move {
            let Ok(conn) = incoming.await else { return };
            if !members.contains(&conn.remote_id()) {
                conn.close(CLOSE_NOT_A_MEMBER.into(), b"not a member");
                return;
            }
            if let Ok(received) = handle(&conn, &identity).await {
                let _ = inbox.send(received).await;
            }
            conn.closed().await;
        });
    }
}

async fn handle(conn: &Connection, identity: &DeviceIdentity) -> Result<Received> {
    let (mut send, mut recv) = conn.accept_bi().await?;
    let bytes = recv.read_to_end(100 + MAX_PAYLOAD).await?;
    let message = SignedMessage::decode(&bytes)?;
    message.verify()?;
    if message.signer != conn.remote_id() {
        bail!("message signer doesn't match the connected device");
    }
    let ack = identity.secret.sign(&domain(ACK_DOMAIN, &message.payload));
    send.write_all(&ack.to_bytes()).await?;
    send.finish()?;
    Ok(Received { from: message.signer, payload: message.payload })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn signed_message_round_trips() {
        let id = DeviceIdentity::generate();
        let msg = id.sign(b"order A-0001 closed");
        let decoded = SignedMessage::decode(&msg.encode()).unwrap();
        assert_eq!(decoded, msg);
        decoded.verify().unwrap();
    }

    #[test]
    fn tampered_payload_is_rejected() {
        let id = DeviceIdentity::generate();
        let mut bytes = id.sign(b"total 1000").encode();
        *bytes.last_mut().unwrap() ^= 1;
        let decoded = SignedMessage::decode(&bytes).unwrap();
        assert!(decoded.verify().is_err());
    }

    #[test]
    fn signature_from_another_device_is_rejected() {
        let alice = DeviceIdentity::generate();
        let mallory = DeviceIdentity::generate();
        let mut msg = mallory.sign(b"refund 5000");
        msg.signer = alice.device_id();
        assert!(msg.verify().is_err());
    }

    #[test]
    fn truncated_and_oversized_messages_are_rejected() {
        assert!(SignedMessage::decode(&[0u8; 50]).is_err());
        let id = DeviceIdentity::generate();
        let mut bytes = id.sign(b"x").encode();
        bytes[96..100].copy_from_slice(&((MAX_PAYLOAD as u32) + 1).to_be_bytes());
        assert!(SignedMessage::decode(&bytes).is_err());
    }
}
