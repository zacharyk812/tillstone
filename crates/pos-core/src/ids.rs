//! Identifiers. Records get UUIDv7s (time-ordered, generated on any device
//! without coordination); devices are identified by their Ed25519 public key.

use std::fmt;

use serde::{Deserialize, Serialize};
use uuid::Uuid;

macro_rules! uuid_id {
    ($(#[$doc:meta])* $name:ident) => {
        $(#[$doc])*
        #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
        pub struct $name(pub Uuid);

        impl $name {
            /// A new, time-ordered id.
            pub fn new() -> Self {
                Self(Uuid::now_v7())
            }

            /// A fixed id, for tests and fixtures.
            pub const fn from_u128(v: u128) -> Self {
                Self(Uuid::from_u128(v))
            }
        }

        impl Default for $name {
            fn default() -> Self {
                Self::new()
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                self.0.fmt(f)
            }
        }
    };
}

uuid_id!(/// One event in a device's log.
    EventId);
uuid_id!(/// A physical site.
    LocationId);
uuid_id!(/// A selling area inside a location (PLAN §5).
    RevenueCenterId);
uuid_id!(/// A staff member.
    StaffId);
uuid_id!(/// A role (set of permissions).
    RoleId);
uuid_id!(/// A catalog item.
    ItemId);
uuid_id!(/// A variant of an item (size, color).
    VariantId);
uuid_id!(/// A group of modifiers.
    ModifierGroupId);
uuid_id!(/// A modifier (extra shot, no onions).
    ModifierId);
uuid_id!(/// A tax rate.
    TaxId);
uuid_id!(/// A discount definition.
    DiscountId);
uuid_id!(/// An order (a sale or a restaurant check).
    OrderId);
uuid_id!(/// One line of an order.
    LineId);
uuid_id!(/// One payment on an order.
    PaymentId);
uuid_id!(/// One refund.
    RefundId);
uuid_id!(/// A cash shift.
    ShiftId);
uuid_id!(/// A table in a floor plan.
    TableId);
uuid_id!(/// A customer (personal data lives outside the event log, PLAN §4).
    CustomerId);

/// A device's Ed25519 public key.
#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct DeviceId(pub [u8; 32]);

impl fmt::Debug for DeviceId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "DeviceId({self})")
    }
}

impl fmt::Display for DeviceId {
    /// Short form: the first 5 bytes in hex, like iroh's `fmt_short`.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for b in &self.0[..5] {
            write!(f, "{b:02x}")?;
        }
        Ok(())
    }
}
