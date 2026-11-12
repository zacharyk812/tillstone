//! Configuration data: catalog, taxes, revenue centers and feature modules
//! (PLAN §4, §5). Owned by the business and replicated to every peer.

use serde::{Deserialize, Serialize};

use crate::{
    ids::{
        DiscountId, ItemId, LocationId, ModifierGroupId, ModifierId, RevenueCenterId, TaxId,
        VariantId,
    },
    money::{Money, Rounding},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ItemKind {
    PhysicalGood,
    MenuItem,
    Service,
    GiftCard,
    Fee,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Item {
    pub id: ItemId,
    pub name: String,
    pub kind: ItemKind,
    /// Price when no variant is chosen.
    pub price: Money,
    pub variants: Vec<Variant>,
    pub modifier_groups: Vec<ModifierGroupId>,
    pub taxes: Vec<TaxId>,
    /// The revenue center credited in reports when this item is sold, even on
    /// another center's check (e.g. a gift-shop mug on a dinner check).
    pub revenue_center: RevenueCenterId,
    pub track_stock: bool,
    pub age_restricted: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Variant {
    pub id: VariantId,
    pub name: String,
    pub price: Money,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModifierGroup {
    pub id: ModifierGroupId,
    pub name: String,
    pub min: u8,
    pub max: u8,
    pub modifiers: Vec<Modifier>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Modifier {
    pub id: ModifierId,
    pub name: String,
    pub price: Money,
}

/// When rounding happens for a tax.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RoundingScope {
    /// Round each line's tax, then add them up.
    PerLine,
    /// Add up the exact tax of all lines, then round once.
    PerOrder,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TaxRate {
    pub id: TaxId,
    pub name: String,
    /// 825 = 8.25 %.
    pub rate_bp: i32,
    /// Prices already include this tax (typical for VAT).
    pub inclusive: bool,
    /// Charged on the price plus the other (non-compound) exclusive taxes.
    pub compound: bool,
    pub rounding: Rounding,
    pub scope: RoundingScope,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DiscountAmount {
    /// Basis points: 1000 = 10 %.
    Percent(i32),
    Fixed(Money),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Discount {
    pub id: DiscountId,
    pub name: String,
    pub amount: DiscountAmount,
    /// Discounts above this percentage need manager approval.
    pub approval_over_bp: Option<i32>,
}

/// Starting sets of modules (PLAN §5). Every module can still be changed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ModePreset {
    Retail,
    QuickService,
    FullService,
    Bar,
}

/// Feature modules, enforced by the engine rather than just hidden in the UI.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Modules {
    pub barcode_scanning: bool,
    pub modifiers: bool,
    pub stock_tracking: bool,
    pub order_numbers: bool,
    pub dining_options: bool,
    pub kitchen_routing: bool,
    pub tables: bool,
    pub seats_and_courses: bool,
    pub open_checks: bool,
    pub tabs: bool,
    pub tips: bool,
    pub split_merge_transfer: bool,
    pub returns: bool,
}

impl Modules {
    pub const fn preset(mode: ModePreset) -> Self {
        let none = Self {
            barcode_scanning: false,
            modifiers: false,
            stock_tracking: false,
            order_numbers: false,
            dining_options: false,
            kitchen_routing: false,
            tables: false,
            seats_and_courses: false,
            open_checks: false,
            tabs: false,
            tips: false,
            split_merge_transfer: false,
            returns: false,
        };
        match mode {
            ModePreset::Retail => {
                Self { barcode_scanning: true, stock_tracking: true, returns: true, ..none }
            }
            ModePreset::QuickService => Self {
                modifiers: true,
                order_numbers: true,
                dining_options: true,
                kitchen_routing: true,
                tips: true,
                ..none
            },
            ModePreset::FullService => Self {
                modifiers: true,
                dining_options: true,
                kitchen_routing: true,
                tables: true,
                seats_and_courses: true,
                open_checks: true,
                tips: true,
                split_merge_transfer: true,
                ..none
            },
            ModePreset::Bar => Self {
                modifiers: true,
                kitchen_routing: true,
                open_checks: true,
                tabs: true,
                tips: true,
                split_merge_transfer: true,
                ..none
            },
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RevenueCenter {
    pub id: RevenueCenterId,
    pub location: LocationId,
    pub name: String,
    pub mode: ModePreset,
    pub modules: Modules,
    /// Orders must be fiscally signed before closing (PLAN §15).
    pub fiscal_mode: bool,
}

impl RevenueCenter {
    pub fn new(id: RevenueCenterId, location: LocationId, name: &str, mode: ModePreset) -> Self {
        Self {
            id,
            location,
            name: name.to_string(),
            mode,
            modules: Modules::preset(mode),
            fiscal_mode: false,
        }
    }
}
