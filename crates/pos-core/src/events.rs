//! Events: the permanent record (PLAN §4, ADR 0004).
//!
//! **The wire format is a promise that lasts forever.** Rules:
//! - Never reorder, rename or remove a variant's `KIND`, or change an existing
//!   payload struct. To change a payload, add a new schema version and an
//!   upcaster in [`Event::decode`] that converts the old version.
//! - New kinds get new numbers. Peers that don't know a kind still store and
//!   forward it ([`Event::Unknown`]), because the payload stays opaque bytes.
//! - `tests::wire_format_is_stable` locks the encoding of every kind.
//! - Kinds in [`FORK_KINDS`] belong to forks and are never used upstream, so a
//!   business that adds its own events can keep merging upstream changes
//!   without a clash (see docs/customizing.md).

use serde::{Deserialize, Serialize, de::DeserializeOwned};

use crate::{
    catalog::{Discount, Item, ModifierGroup, RevenueCenter, TaxRate},
    ids::{
        CustomerId, DeviceId, ItemId, LineId, LocationId, OrderId, PaymentId, RefundId,
        RevenueCenterId, RoleId, ShiftId, StaffId, TableId, VariantId,
    },
    money::{Currency, Money},
    permissions::Role,
    pricing::{DiscountSnapshot, ModifierSnapshot, PricingRules},
};

/// What a device is used for (PLAN §5).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DeviceRole {
    Register,
    Handheld,
    KitchenDisplay,
    CustomerDisplay,
    OrderBoard,
    Manager,
    AlwaysOnPeer,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Fulfillment {
    Here,
    ToGo,
    Pickup,
    Delivery,
    Ship,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Tender {
    Cash,
    Card { provider: String, reader: Option<String> },
    Other { name: String },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum VoidReason {
    CustomerChangedMind,
    Mistake,
    Waste,
    Comp,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CashMovementKind {
    PaidIn,
    PaidOut,
    NoSale,
}

/// A configuration record, replaced as a whole and ordered by hybrid logical
/// clock: the latest change to a record wins (PLAN §6).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ConfigRecord {
    RevenueCenter(RevenueCenter),
    Item(Item),
    ModifierGroup(ModifierGroup),
    Tax(TaxRate),
    Discount(Discount),
    Role(Role),
    Staff(Staff),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Staff {
    pub id: StaffId,
    /// Display name. Staff names are configuration, never inside sale events.
    pub name: String,
    pub role: RoleId,
    /// Argon2id hash of the PIN, in PHC string format.
    pub pin_hash: String,
    pub locations: Vec<LocationId>,
    pub active: bool,
}

/// Points at one event in one device's log.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct EventRef {
    pub device: DeviceId,
    pub seq: u64,
}

// ---- Payloads (schema version 1) -------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BusinessCreated {
    pub name: String,
    pub currency: Currency,
    /// The owner authority (PLAN §6): only this key can add or revoke devices.
    pub owner_key: [u8; 32],
    pub first_location: LocationId,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeviceAdded {
    pub device: DeviceId,
    pub name: String,
    pub role: DeviceRole,
    pub revenue_centers: Vec<RevenueCenterId>,
    /// Single-letter-ish prefix for this device's order numbers ("A").
    pub order_prefix: String,
    /// Owner-authority signature over [`DeviceAdded::signing_bytes`].
    #[serde(with = "sig")]
    pub owner_signature: [u8; 64],
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeviceRevoked {
    pub device: DeviceId,
    /// The last event from this device that is still trusted (PLAN §6).
    pub trusted_through_seq: u64,
    #[serde(with = "sig")]
    pub owner_signature: [u8; 64],
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConfigChanged {
    pub record: ConfigRecord,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConfigDeleted {
    pub kind: String,
    pub id: uuid::Uuid,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ShiftOpened {
    pub shift: ShiftId,
    pub float: Money,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ShiftClosed {
    pub shift: ShiftId,
    /// Counted blind: the cashier didn't see the expected amount (PLAN §9).
    pub counted: Money,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CashMovement {
    pub shift: ShiftId,
    pub kind: CashMovementKind,
    pub amount: Money,
    pub reason: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OrderOpened {
    pub order: OrderId,
    pub revenue_center: RevenueCenterId,
    pub currency: Currency,
    pub pricing_rules: PricingRules,
    /// Human-facing number, unique per device ("A-0142").
    pub number: String,
    pub fulfillment: Fulfillment,
    pub table: Option<TableId>,
    pub guests: Option<u16>,
    pub customer: Option<CustomerId>,
    pub tab_name: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LineAdded {
    pub order: OrderId,
    pub line: LineId,
    pub item: ItemId,
    pub variant: Option<VariantId>,
    /// Snapshot of the name shown on receipts.
    pub name: String,
    /// Credited in reports (may differ from the order's revenue center).
    pub revenue_center: RevenueCenterId,
    pub quantity: i64,
    pub unit_price: Money,
    pub modifiers: Vec<ModifierSnapshot>,
    pub taxes: Vec<TaxRate>,
    pub seat: Option<u8>,
    pub course: Option<u8>,
    pub track_stock: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LineQtyChanged {
    pub order: OrderId,
    pub line: LineId,
    pub quantity: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LineRemoved {
    pub order: OrderId,
    pub line: LineId,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OrderSent {
    pub order: OrderId,
    pub lines: Vec<LineId>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LineVoidedAfterSend {
    pub order: OrderId,
    pub line: LineId,
    pub reason: VoidReason,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DiscountApplied {
    pub order: OrderId,
    /// `None` applies to the whole order.
    pub line: Option<LineId>,
    pub discount: DiscountSnapshot,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TableAssigned {
    pub order: OrderId,
    pub table: TableId,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CheckLeaseTaken {
    pub order: OrderId,
    /// Wall-clock milliseconds when the lease lapses if not released.
    pub until_ms: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CheckLeaseReleased {
    pub order: OrderId,
}

/// A full copy of a line, carried by splits and merges so that every order
/// can be rebuilt from its own events alone.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LineSnapshot {
    pub line: LineId,
    pub item: ItemId,
    pub variant: Option<VariantId>,
    pub name: String,
    pub revenue_center: RevenueCenterId,
    pub quantity: i64,
    pub unit_price: Money,
    pub modifiers: Vec<ModifierSnapshot>,
    pub taxes: Vec<TaxRate>,
    pub discounts: Vec<DiscountSnapshot>,
    pub seat: Option<u8>,
    pub course: Option<u8>,
    pub sent: bool,
    pub track_stock: bool,
}

/// Moves lines to a new order (split by seat, item or evenly by item).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OrderSplit {
    pub from: OrderId,
    pub to: OrderId,
    pub number: String,
    pub revenue_center: RevenueCenterId,
    pub currency: Currency,
    pub pricing_rules: PricingRules,
    pub fulfillment: Fulfillment,
    pub table: Option<TableId>,
    pub lines: Vec<LineSnapshot>,
}

/// Moves every line of `from` into `into`; `from` ends as merged.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OrderMerged {
    pub from: OrderId,
    pub into: OrderId,
    pub lines: Vec<LineSnapshot>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PaymentStarted {
    pub order: OrderId,
    pub payment: PaymentId,
    pub tender: Tender,
    pub amount: Money,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PaymentCaptured {
    pub order: OrderId,
    pub payment: PaymentId,
    /// The provider's id for the payment, for reconciliation.
    pub provider_ref: Option<String>,
    pub tip: Money,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PaymentFailed {
    pub order: OrderId,
    pub payment: PaymentId,
    pub reason: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CashTendered {
    pub order: OrderId,
    pub payment: PaymentId,
    /// What the customer handed over.
    pub given: Money,
    /// What was applied to the order; the rest is change.
    pub applied: Money,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OrderClosed {
    pub order: OrderId,
    /// The total the creating device computed. Peers recompute and compare.
    pub total: Money,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OrderVoided {
    pub order: OrderId,
    pub reason: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RefundIssued {
    pub order: OrderId,
    pub refund: RefundId,
    pub lines: Vec<(LineId, i64)>,
    pub amount: Money,
    pub tender: Tender,
    pub reason: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InventoryAdjusted {
    pub item: ItemId,
    pub delta: i64,
    pub reason: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FiscalSigned {
    pub order: OrderId,
    pub adapter: String,
    /// Fields the receipt must print (signature, counters, QR data...).
    pub fields: Vec<(String, String)>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FiscalOutage {
    pub started: bool,
    pub reason: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CustomerErased {
    pub customer: CustomerId,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Correction {
    pub target: EventRef,
    pub reason: String,
}

mod sig {
    use serde::{Deserialize, Deserializer, Serializer};

    pub fn serialize<S: Serializer>(v: &[u8; 64], s: S) -> Result<S::Ok, S::Error> {
        s.serialize_bytes(v)
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<[u8; 64], D::Error> {
        let v: Vec<u8> = Deserialize::deserialize(d)?;
        v.try_into().map_err(|_| serde::de::Error::custom("signature must be 64 bytes"))
    }
}

impl DeviceAdded {
    /// What the owner authority signs.
    pub fn signing_bytes(&self) -> Vec<u8> {
        let mut out = b"tillstone/device-added/v1\0".to_vec();
        out.extend_from_slice(&self.device.0);
        out.extend_from_slice(&(self.role as u8).to_be_bytes());
        out.extend_from_slice(self.order_prefix.as_bytes());
        out
    }
}

impl DeviceRevoked {
    pub fn signing_bytes(&self) -> Vec<u8> {
        let mut out = b"tillstone/device-revoked/v1\0".to_vec();
        out.extend_from_slice(&self.device.0);
        out.extend_from_slice(&self.trusted_through_seq.to_be_bytes());
        out
    }
}

// ---- The event enum ---------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DecodeError {
    Malformed,
}

macro_rules! events {
    ($( $kind:literal => $variant:ident ),* $(,)?) => {
        /// Every kind of event, in its current schema.
        #[derive(Debug, Clone, PartialEq, Eq)]
        pub enum Event {
            $( $variant($variant), )*
            /// A kind this version doesn't know. Stored and forwarded as is.
            Unknown { kind: u16, schema: u16 },
        }

        impl Event {
            /// Stable numeric kind. Never reuse or change a number.
            pub fn kind(&self) -> u16 {
                match self {
                    $( Self::$variant(_) => $kind, )*
                    Self::Unknown { kind, .. } => *kind,
                }
            }

            pub fn name(&self) -> &'static str {
                match self {
                    $( Self::$variant(_) => stringify!($variant), )*
                    Self::Unknown { .. } => "Unknown",
                }
            }

            /// Current schema version of each kind.
            pub fn schema(&self) -> u16 {
                match self {
                    Self::Unknown { schema, .. } => *schema,
                    _ => 1,
                }
            }

            /// Encodes the payload. `Unknown` events can't be re-encoded; their
            /// original bytes are kept by the ledger instead.
            pub fn encode_payload(&self) -> Option<Vec<u8>> {
                match self {
                    $( Self::$variant(p) => Some(encode(p)), )*
                    Self::Unknown { .. } => None,
                }
            }

            /// Decodes a payload, upcasting old schema versions to the current one.
            pub fn decode(kind: u16, schema: u16, bytes: &[u8]) -> Result<Self, DecodeError> {
                match (kind, schema) {
                    $( ($kind, 1) => Ok(Self::$variant(decode(bytes)?)), )*
                    // Upcasters go here, e.g.:
                    // (12, 1) => Ok(Self::LineAdded(decode::<LineAddedV1>(bytes)?.into())),
                    _ => Ok(Self::Unknown { kind, schema }),
                }
            }
        }
    };
}

events! {
    1 => BusinessCreated,
    2 => DeviceAdded,
    3 => DeviceRevoked,
    4 => ConfigChanged,
    5 => ConfigDeleted,
    6 => ShiftOpened,
    7 => ShiftClosed,
    8 => CashMovement,
    10 => OrderOpened,
    11 => LineAdded,
    12 => LineQtyChanged,
    13 => LineRemoved,
    14 => OrderSent,
    15 => LineVoidedAfterSend,
    16 => DiscountApplied,
    17 => TableAssigned,
    18 => CheckLeaseTaken,
    19 => CheckLeaseReleased,
    20 => OrderSplit,
    21 => OrderMerged,
    22 => PaymentStarted,
    23 => PaymentCaptured,
    24 => PaymentFailed,
    25 => CashTendered,
    26 => OrderClosed,
    27 => OrderVoided,
    28 => RefundIssued,
    30 => InventoryAdjusted,
    40 => FiscalSigned,
    41 => FiscalOutage,
    50 => CustomerErased,
    60 => Correction,
}

/// Event kinds reserved for forks. Upstream Tillstone never uses these
/// numbers; devices that don't know a fork's kinds keep and forward them.
pub const FORK_KINDS: std::ops::RangeInclusive<u16> = 0x8000..=0xFFFF;

fn encode<T: Serialize>(v: &T) -> Vec<u8> {
    postcard::to_allocvec(v).expect("in-memory encoding can't fail")
}

fn decode<T: DeserializeOwned>(bytes: &[u8]) -> Result<T, DecodeError> {
    postcard::from_bytes(bytes).map_err(|_| DecodeError::Malformed)
}

impl Event {
    /// Every order this event affects (splits and merges affect two).
    pub fn orders(&self) -> Vec<OrderId> {
        match self {
            Self::OrderSplit(e) => vec![e.from, e.to],
            Self::OrderMerged(e) => vec![e.from, e.into],
            other => other.order().into_iter().collect(),
        }
    }

    /// The order this event belongs to, if any.
    pub fn order(&self) -> Option<OrderId> {
        Some(match self {
            Self::OrderOpened(e) => e.order,
            Self::LineAdded(e) => e.order,
            Self::LineQtyChanged(e) => e.order,
            Self::LineRemoved(e) => e.order,
            Self::OrderSent(e) => e.order,
            Self::LineVoidedAfterSend(e) => e.order,
            Self::DiscountApplied(e) => e.order,
            Self::TableAssigned(e) => e.order,
            Self::CheckLeaseTaken(e) => e.order,
            Self::CheckLeaseReleased(e) => e.order,
            Self::OrderSplit(e) => e.from,
            Self::OrderMerged(e) => e.into,
            Self::PaymentStarted(e) => e.order,
            Self::PaymentCaptured(e) => e.order,
            Self::PaymentFailed(e) => e.order,
            Self::CashTendered(e) => e.order,
            Self::OrderClosed(e) => e.order,
            Self::OrderVoided(e) => e.order,
            Self::RefundIssued(e) => e.order,
            Self::FiscalSigned(e) => e.order,
            _ => return None,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::money::Currency;

    fn usd(m: i64) -> Money {
        Money::from_minor(m, Currency::USD)
    }

    fn samples() -> Vec<Event> {
        let order = OrderId::from_u128(1);
        vec![
            Event::OrderOpened(OrderOpened {
                order,
                revenue_center: RevenueCenterId::from_u128(2),
                currency: Currency::USD,
                pricing_rules: PricingRules::V1,
                number: "A-0001".into(),
                fulfillment: Fulfillment::Here,
                table: None,
                guests: Some(2),
                customer: None,
                tab_name: None,
            }),
            Event::LineAdded(LineAdded {
                order,
                line: LineId::from_u128(3),
                item: ItemId::from_u128(4),
                variant: None,
                name: "Latte".into(),
                revenue_center: RevenueCenterId::from_u128(2),
                quantity: 1,
                unit_price: usd(475),
                modifiers: vec![ModifierSnapshot { name: "Oat milk".into(), price: usd(75) }],
                taxes: vec![],
                seat: None,
                course: None,
                track_stock: false,
            }),
            Event::CashTendered(CashTendered {
                order,
                payment: PaymentId::from_u128(5),
                given: usd(1000),
                applied: usd(550),
            }),
            Event::OrderClosed(OrderClosed { order, total: usd(550) }),
        ]
    }

    #[test]
    fn every_sample_round_trips() {
        for e in samples() {
            let bytes = e.encode_payload().unwrap();
            assert_eq!(Event::decode(e.kind(), e.schema(), &bytes).unwrap(), e, "{}", e.name());
        }
    }

    #[test]
    fn upstream_never_uses_fork_kinds() {
        for e in samples() {
            assert!(!FORK_KINDS.contains(&e.kind()), "{} uses a fork kind", e.name());
        }
        assert_eq!(
            Event::decode(0x8001, 1, b"fork data").unwrap(),
            Event::Unknown { kind: 0x8001, schema: 1 }
        );
    }

    #[test]
    fn unknown_kinds_are_kept_not_rejected() {
        assert_eq!(
            Event::decode(999, 1, b"whatever").unwrap(),
            Event::Unknown { kind: 999, schema: 1 }
        );
        assert_eq!(Event::decode(11, 7, b"x").unwrap(), Event::Unknown { kind: 11, schema: 7 });
    }

    #[test]
    fn garbage_payload_for_a_known_kind_is_an_error() {
        assert_eq!(Event::decode(26, 1, &[0xff]), Err(DecodeError::Malformed));
    }

    /// Locks the wire format. If this fails, you changed the encoding of an
    /// existing event, which would make old records unreadable. Add a new
    /// schema version instead (see the module docs).
    #[test]
    fn wire_format_is_stable() {
        let hex: Vec<String> = samples()
            .iter()
            .map(|e| e.encode_payload().unwrap().iter().map(|b| format!("{b:02x}")).collect())
            .collect();
        let expected = include_str!("../tests/fixtures/event_wire_v1.txt");
        let expected: Vec<&str> = expected.lines().filter(|l| !l.starts_with('#')).collect();
        assert_eq!(hex, expected);
    }
}
