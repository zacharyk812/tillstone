//! Each device's append-only, hash-chained, signed log (PLAN §6, ADR 0004).
//!
//! A record on the wire and on disk is `signature (64 bytes) | body`, where
//! `body` is the postcard encoding of `(Header, payload bytes)`. The hash is
//! SHA-256 over a domain prefix and `body`, and the signature is Ed25519 over
//! that hash. Bodies are hashed and signed exactly as stored, and never
//! re-encoded, so encoding details can't break verification later.

use std::collections::HashMap;

use ed25519_dalek::{Signature, Signer, SigningKey, Verifier, VerifyingKey};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::{
    events::{DecodeError, Event},
    hlc::{Hlc, HlcClock},
    ids::{DeviceId, EventId, LocationId, StaffId},
};

/// Version of the record layout itself (not of any payload).
pub const RECORD_FORMAT: u16 = 1;
const HASH_DOMAIN: &[u8] = b"tillstone/event/v1\0";

pub type Hash = [u8; 32];

/// Hash of "nothing": the `prev_hash` of a device's first event.
pub const GENESIS: Hash = [0; 32];

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Header {
    pub format: u16,
    pub event_id: EventId,
    pub device: DeviceId,
    /// 1, 2, 3... with no gaps, per device.
    pub seq: u64,
    pub prev_hash: Hash,
    pub hlc: Hlc,
    pub location: LocationId,
    /// Who did it, if a staff member was logged in.
    pub staff: Option<StaffId>,
    /// Manager who approved it, if approval was needed.
    pub approved_by: Option<StaffId>,
    /// Version of `pos-core` that created it (PLAN §6).
    pub core_version: String,
    /// Training-mode events never count as real sales (PLAN §20).
    pub training: bool,
    pub kind: u16,
    pub schema: u16,
}

/// A verified record: header, decoded event, and the exact stored bytes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Record {
    pub header: Header,
    pub event: Event,
    pub hash: Hash,
    /// `signature | body`, exactly as stored and sent.
    pub wire: Vec<u8>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LedgerError {
    Malformed,
    UnsupportedFormat(u16),
    BadSignature,
    /// The next event from a device isn't the one we expected.
    Gap {
        device: DeviceId,
        expected: u64,
        got: u64,
    },
    /// `prev_hash` doesn't match the previous event's hash.
    BrokenChain {
        device: DeviceId,
        seq: u64,
    },
    /// Two different events claim the same place in a device's log: proof
    /// the device rewrote its history (PLAN §6 "tamper detection").
    Equivocation {
        device: DeviceId,
        seq: u64,
    },
}

impl std::fmt::Display for LedgerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Malformed => f.write_str("malformed record"),
            Self::UnsupportedFormat(v) => write!(f, "unsupported record format {v}"),
            Self::BadSignature => f.write_str("signature doesn't verify"),
            Self::Gap { device, expected, got } => {
                write!(f, "device {device}: expected event {expected}, got {got}")
            }
            Self::BrokenChain { device, seq } => {
                write!(f, "device {device}: event {seq} breaks the hash chain")
            }
            Self::Equivocation { device, seq } => {
                write!(
                    f,
                    "device {device}: two different events numbered {seq} (history was rewritten)"
                )
            }
        }
    }
}

impl std::error::Error for LedgerError {}

impl From<DecodeError> for LedgerError {
    fn from(_: DecodeError) -> Self {
        Self::Malformed
    }
}

fn hash_body(body: &[u8]) -> Hash {
    let mut h = Sha256::new();
    h.update(HASH_DOMAIN);
    h.update(body);
    h.finalize().into()
}

/// Decodes a record and checks its own integrity: hash and signature by the
/// device named in its header. Chain position is checked by [`ChainTracker`].
pub fn verify_record(wire: &[u8]) -> Result<Record, LedgerError> {
    if wire.len() < 64 {
        return Err(LedgerError::Malformed);
    }
    let (sig, body) = wire.split_at(64);
    let (header, payload): (Header, Vec<u8>) =
        postcard::from_bytes(body).map_err(|_| LedgerError::Malformed)?;
    if header.format != RECORD_FORMAT {
        return Err(LedgerError::UnsupportedFormat(header.format));
    }
    let hash = hash_body(body);
    let key = VerifyingKey::from_bytes(&header.device.0).map_err(|_| LedgerError::BadSignature)?;
    let sig = Signature::from_bytes(sig.try_into().map_err(|_| LedgerError::Malformed)?);
    key.verify(&hash, &sig).map_err(|_| LedgerError::BadSignature)?;
    let event = Event::decode(header.kind, header.schema, &payload)?;
    Ok(Record { header, event, hash, wire: wire.to_vec() })
}

/// Context for a new event.
#[derive(Debug, Clone, Copy)]
pub struct Context {
    pub location: LocationId,
    pub staff: Option<StaffId>,
    pub approved_by: Option<StaffId>,
    pub training: bool,
}

/// The writing end of one device's log.
pub struct DeviceLog {
    key: SigningKey,
    device: DeviceId,
    seq: u64,
    prev: Hash,
    clock: HlcClock,
}

impl DeviceLog {
    /// A brand-new device. `secret` is the device's Ed25519 secret key (the
    /// same key iroh uses for transport, PLAN §9).
    pub fn new(secret: &[u8; 32]) -> Self {
        let key = SigningKey::from_bytes(secret);
        let device = DeviceId(key.verifying_key().to_bytes());
        Self { key, device, seq: 0, prev: GENESIS, clock: HlcClock::new(device) }
    }

    /// Continues a device's log after a restart, from its last record.
    pub fn resume(secret: &[u8; 32], last: &Header, last_hash: Hash) -> Self {
        let mut log = Self::new(secret);
        log.seq = last.seq;
        log.prev = last_hash;
        log.clock = HlcClock::resume(last.hlc);
        log
    }

    pub fn device(&self) -> DeviceId {
        self.device
    }

    pub fn clock(&mut self) -> &mut HlcClock {
        &mut self.clock
    }

    /// Creates, hashes and signs the next event.
    pub fn append(&mut self, now_ms: u64, ctx: Context, event: &Event) -> Record {
        let payload = event.encode_payload().expect("only known events are created locally");
        let header = Header {
            format: RECORD_FORMAT,
            event_id: EventId::new(),
            device: self.device,
            seq: self.seq + 1,
            prev_hash: self.prev,
            hlc: self.clock.tick(now_ms),
            location: ctx.location,
            staff: ctx.staff,
            approved_by: ctx.approved_by,
            core_version: crate::CORE_VERSION.to_string(),
            training: ctx.training,
            kind: event.kind(),
            schema: event.schema(),
        };
        let body = postcard::to_allocvec(&(&header, &payload)).expect("in-memory encoding");
        let hash = hash_body(&body);
        let sig = self.key.sign(&hash);
        let mut wire = sig.to_bytes().to_vec();
        wire.extend_from_slice(&body);
        self.seq = header.seq;
        self.prev = hash;
        Record { header, event: event.clone(), hash, wire }
    }
}

/// Signs with the owner authority key (PLAN §6).
pub fn owner_sign(owner_secret: &[u8; 32], bytes: &[u8]) -> [u8; 64] {
    SigningKey::from_bytes(owner_secret).sign(bytes).to_bytes()
}

pub fn owner_public(owner_secret: &[u8; 32]) -> [u8; 32] {
    SigningKey::from_bytes(owner_secret).verifying_key().to_bytes()
}

pub fn owner_verify(owner_key: &[u8; 32], bytes: &[u8], sig: &[u8; 64]) -> bool {
    VerifyingKey::from_bytes(owner_key)
        .map(|k| k.verify(bytes, &Signature::from_bytes(sig)).is_ok())
        .unwrap_or(false)
}

/// What happened when a record was offered to the tracker.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Accepted {
    /// The next event in the device's log.
    New,
    /// Already had it (retries are harmless).
    Duplicate,
}

/// The receiving end: tracks each device's log and checks every new record
/// extends it correctly. Also provides the version vector used to sync.
#[derive(Debug, Clone, Default)]
pub struct ChainTracker {
    heads: HashMap<DeviceId, (u64, Hash)>,
    /// Hashes of recent records, to tell duplicates from forks.
    seen: HashMap<(DeviceId, u64), Hash>,
}

impl ChainTracker {
    pub fn accept(&mut self, record: &Record) -> Result<Accepted, LedgerError> {
        let h = &record.header;
        if let Some(existing) = self.seen.get(&(h.device, h.seq)) {
            return if *existing == record.hash {
                Ok(Accepted::Duplicate)
            } else {
                Err(LedgerError::Equivocation { device: h.device, seq: h.seq })
            };
        }
        let (last_seq, last_hash) = self.heads.get(&h.device).copied().unwrap_or((0, GENESIS));
        if h.seq != last_seq + 1 {
            return Err(LedgerError::Gap { device: h.device, expected: last_seq + 1, got: h.seq });
        }
        if h.prev_hash != last_hash {
            return Err(LedgerError::BrokenChain { device: h.device, seq: h.seq });
        }
        self.heads.insert(h.device, (h.seq, record.hash));
        self.seen.insert((h.device, h.seq), record.hash);
        Ok(Accepted::New)
    }

    /// Highest sequence number held from each device (PLAN §6 "version vector").
    pub fn version_vector(&self) -> Vec<(DeviceId, u64)> {
        let mut v: Vec<_> = self.heads.iter().map(|(d, (s, _))| (*d, *s)).collect();
        v.sort();
        v
    }

    pub fn head(&self, device: &DeviceId) -> Option<(u64, Hash)> {
        self.heads.get(device).copied()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::events::{FiscalOutage, OrderVoided};
    use crate::ids::OrderId;

    fn ctx() -> Context {
        Context {
            location: LocationId::from_u128(1),
            staff: None,
            approved_by: None,
            training: false,
        }
    }

    fn event(n: u8) -> Event {
        Event::OrderVoided(OrderVoided {
            order: OrderId::from_u128(n.into()),
            reason: "test".into(),
        })
    }

    #[test]
    fn records_verify_and_chain() {
        let mut log = DeviceLog::new(&[7; 32]);
        let mut tracker = ChainTracker::default();
        for i in 1..=5 {
            let rec = log.append(1_000 + i, ctx(), &event(i as u8));
            let verified = verify_record(&rec.wire).unwrap();
            assert_eq!(verified, rec);
            assert_eq!(tracker.accept(&verified).unwrap(), Accepted::New);
        }
        assert_eq!(tracker.version_vector(), vec![(log.device(), 5)]);
    }

    #[test]
    fn duplicates_are_harmless() {
        let mut log = DeviceLog::new(&[7; 32]);
        let mut tracker = ChainTracker::default();
        let rec = log.append(1, ctx(), &event(1));
        tracker.accept(&rec).unwrap();
        assert_eq!(tracker.accept(&rec).unwrap(), Accepted::Duplicate);
    }

    #[test]
    fn any_flipped_bit_breaks_the_signature() {
        let mut log = DeviceLog::new(&[7; 32]);
        let rec = log.append(1, ctx(), &event(1));
        for i in 0..rec.wire.len() {
            let mut w = rec.wire.clone();
            w[i] ^= 1;
            assert!(verify_record(&w).is_err(), "flipping byte {i} went unnoticed");
        }
    }

    #[test]
    fn gaps_are_detected() {
        let mut log = DeviceLog::new(&[7; 32]);
        let mut tracker = ChainTracker::default();
        let _skipped = log.append(1, ctx(), &event(1));
        let second = log.append(2, ctx(), &event(2));
        assert!(matches!(
            tracker.accept(&second),
            Err(LedgerError::Gap { expected: 1, got: 2, .. })
        ));
    }

    #[test]
    fn rewritten_history_is_detected() {
        let secret = [7; 32];
        let mut honest = DeviceLog::new(&secret);
        let mut tracker = ChainTracker::default();
        let first = honest.append(1, ctx(), &event(1));
        tracker.accept(&first).unwrap();
        tracker.accept(&honest.append(2, ctx(), &event(2))).unwrap();

        // The same device key signs a different event 2 (e.g. after deleting a sale).
        let mut forked = DeviceLog::resume(&secret, &first.header, first.hash);
        let fake = forked.append(
            3,
            ctx(),
            &Event::FiscalOutage(FiscalOutage { started: true, reason: "x".into() }),
        );
        assert!(matches!(tracker.accept(&fake), Err(LedgerError::Equivocation { seq: 2, .. })));
    }

    #[test]
    fn records_from_another_key_dont_verify_as_this_device() {
        let mut a = DeviceLog::new(&[7; 32]);
        let b = DeviceLog::new(&[8; 32]);
        let rec = a.append(1, ctx(), &event(1));
        // Swap in B's device id: the signature no longer matches.
        let (header, payload): (Header, Vec<u8>) = postcard::from_bytes(&rec.wire[64..]).unwrap();
        let forged = Header { device: b.device(), ..header };
        let mut wire = rec.wire[..64].to_vec();
        wire.extend(postcard::to_allocvec(&(&forged, &payload)).unwrap());
        assert_eq!(verify_record(&wire), Err(LedgerError::BadSignature));
    }

    #[test]
    fn owner_signatures() {
        let owner = [9; 32];
        let sig = owner_sign(&owner, b"add device");
        assert!(owner_verify(&owner_public(&owner), b"add device", &sig));
        assert!(!owner_verify(&owner_public(&owner), b"add other device", &sig));
    }
}
