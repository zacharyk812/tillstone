//! Tillstone's core: everything that must never be wrong.
//!
//! M0 contains only the money foundations (`docs/PLAN.md` §4 "Money rules").
//! Pricing, events and the ledger arrive in M2.

#![forbid(unsafe_code)]

pub mod money;

pub use money::{Currency, Money, MoneyError, Rounding};

/// Version of the core, recorded in every event (§6) so peers can re-check
/// sales with the rules that created them.
pub const CORE_VERSION: &str = env!("CARGO_PKG_VERSION");
