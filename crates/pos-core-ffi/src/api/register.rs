//! The register, as the Flutter app sees it. A thin wrapper over
//! `pos_app::Register` that converts ids to strings and money to display
//! text plus minor units. No business logic here (PLAN §1).

use std::sync::{Mutex, MutexGuard};

use flutter_rust_bridge::frb;
use pos_app::{AppError, Register, demo};
use pos_core::{
    catalog::ModePreset,
    events::Fulfillment,
    ids::{ItemId, LineId, ModifierId, OrderId, RevenueCenterId, ShiftId, StaffId, VariantId},
    money::Money,
    order::Status,
};
use uuid::Uuid;

/// Formats money for display, e.g. "$12.34", "€5.00", "1,500 JPY".
/// (Full localization comes with translations, PLAN §11.)
pub(crate) fn format_money(m: Money) -> String {
    let c = m.currency();
    let symbol = match c.code() {
        "USD" | "CAD" | "AUD" | "NZD" | "MXN" => Some("$"),
        "EUR" => Some("€"),
        "GBP" => Some("£"),
        "JPY" => Some("¥"),
        _ => None,
    };
    let exp = u32::from(c.exponent());
    let abs = m.minor().unsigned_abs();
    let scale = 10u64.pow(exp);
    let whole = abs / scale;
    let mut grouped = String::new();
    for (i, ch) in whole.to_string().chars().rev().enumerate() {
        if i > 0 && i % 3 == 0 {
            grouped.push(',');
        }
        grouped.push(ch);
    }
    let whole: String = grouped.chars().rev().collect();
    let number = if exp == 0 { whole } else { format!("{whole}.{:0w$}", abs % scale, w = exp as usize) };
    let sign = if m.minor() < 0 { "−" } else { "" };
    match symbol {
        Some(s) => format!("{sign}{s}{number}"),
        None => format!("{sign}{number} {}", c.code()),
    }
}

fn err(e: AppError) -> String {
    e.to_string()
}

fn parse<T>(s: &str, f: fn(Uuid) -> T) -> Result<T, String> {
    Uuid::parse_str(s).map(f).map_err(|_| "invalid id".to_string())
}

pub struct StaffView {
    pub id: String,
    pub name: String,
    pub initials: String,
    pub role: String,
}

pub struct CenterView {
    pub id: String,
    pub name: String,
    /// "retail", "quick_service", "full_service" or "bar".
    pub mode: String,
}

pub struct OptionView {
    pub id: String,
    pub name: String,
    pub price: String,
}

pub struct OptionGroupView {
    pub name: String,
    pub min: u8,
    pub max: u8,
    pub options: Vec<OptionView>,
}

pub struct ItemView {
    pub id: String,
    pub name: String,
    pub price: String,
    pub variants: Vec<OptionView>,
    pub option_groups: Vec<OptionGroupView>,
    /// `None` when stock isn't tracked.
    pub stock: Option<i64>,
}

pub struct LineView {
    pub id: String,
    pub quantity: i64,
    pub name: String,
    pub details: Vec<String>,
    pub amount: String,
}

pub struct OrderView {
    pub id: String,
    pub number: String,
    pub lines: Vec<LineView>,
    pub subtotal: String,
    pub discount: Option<String>,
    pub taxes: Vec<(String, String)>,
    pub total: String,
    pub total_minor: i64,
    pub balance: String,
    pub balance_minor: i64,
    pub closed: bool,
}

pub struct CashView {
    pub change: String,
    pub closed: bool,
}

pub struct ShiftView {
    pub id: String,
    pub expected: String,
    pub counted: Option<String>,
    pub variance: Option<String>,
}

pub struct ReportView {
    pub orders: u32,
    pub total: String,
    pub tips: String,
    pub refunds: String,
    pub by_tender: Vec<(String, String)>,
}

fn initials(name: &str) -> String {
    name.split_whitespace().filter_map(|w| w.chars().next()).take(2).collect::<String>().to_uppercase()
}

/// One register, open on this device.
///
/// The register sits behind a mutex: calls from the app are serialized, so
/// two taps can never interleave halfway through an action.
#[frb(opaque)]
pub struct Till {
    reg: Mutex<Register>,
}

impl Till {
    fn r(&self) -> MutexGuard<'_, Register> {
        // A panic mid-action leaves no partial record (records are written
        // only after all checks pass), so a poisoned lock is safe to reuse.
        self.reg.lock().unwrap_or_else(|e| e.into_inner())
    }

    /// Creates the demo business on this device (PLAN §17 "try it").
    pub fn create_demo(
        db_path: String,
        db_key: Vec<u8>,
        device_secret: Vec<u8>,
        owner_secret: Vec<u8>,
    ) -> Result<Till, String> {
        let key = |v: Vec<u8>| <[u8; 32]>::try_from(v).map_err(|_| "keys must be 32 bytes".to_string());
        let mut reg = Register::create(&db_path, &key(db_key)?, &key(device_secret)?, &key(owner_secret)?, demo::setup())
            .map_err(err)?;
        demo::seed(&mut reg).map_err(err)?;
        reg.sign_out();
        Ok(Till { reg: Mutex::new(reg) })
    }

    /// Opens the register stored at `db_path`.
    pub fn open(db_path: String, db_key: Vec<u8>, device_secret: Vec<u8>) -> Result<Till, String> {
        let key = |v: Vec<u8>| <[u8; 32]>::try_from(v).map_err(|_| "keys must be 32 bytes".to_string());
        Register::open(&db_path, &key(db_key)?, &key(device_secret)?).map(|reg| Till { reg: Mutex::new(reg) }).map_err(err)
    }

    #[frb(sync)]
    pub fn business_name(&self) -> String {
        self.r().book().business().map(|b| b.name.clone()).unwrap_or_default()
    }

    #[frb(sync)]
    pub fn demo_pins(&self) -> Vec<(String, String)> {
        vec![("Alex (owner)".into(), demo::DEMO_OWNER_PIN.into()), ("Maya (cashier)".into(), demo::DEMO_CASHIER_PIN.into())]
    }

    #[frb(sync)]
    pub fn staff(&self) -> Vec<StaffView> {
        self.r()
            .staff_list()
            .into_iter()
            .map(|(id, name)| StaffView { id: id.to_string(), initials: initials(&name), name, role: String::new() })
            .collect()
    }

    /// Signs in by PIN; Argon2 is slow on purpose, so this runs off the UI thread.
    pub fn sign_in(&self, pin: String, staff_id: Option<String>) -> Result<StaffView, String> {
        let signed = match staff_id {
            Some(id) => self.r().sign_in_as(parse(&id, StaffId)?, &pin),
            None => self.r().sign_in(&pin),
        }
        .map_err(err)?;
        Ok(StaffView { id: signed.id.to_string(), initials: initials(&signed.name), name: signed.name, role: signed.role })
    }

    #[frb(sync)]
    pub fn sign_out(&self) {
        self.r().sign_out();
    }

    #[frb(sync)]
    pub fn signed_in(&self) -> Option<StaffView> {
        self.r().signed_in().map(|s| StaffView { id: s.id.to_string(), initials: initials(&s.name), name: s.name, role: s.role })
    }

    #[frb(sync)]
    pub fn training(&self) -> bool {
        self.r().training()
    }

    #[frb(sync)]
    pub fn set_training(&self, on: bool) {
        self.r().set_training(on);
    }

    #[frb(sync)]
    pub fn revenue_centers(&self) -> Vec<CenterView> {
        self.r()
            .revenue_centers()
            .into_iter()
            .map(|c| CenterView {
                id: c.id.to_string(),
                name: c.name,
                mode: match c.mode {
                    ModePreset::Retail => "retail",
                    ModePreset::QuickService => "quick_service",
                    ModePreset::FullService => "full_service",
                    ModePreset::Bar => "bar",
                }
                .into(),
            })
            .collect()
    }

    #[frb(sync)]
    pub fn items(&self, center_id: String) -> Result<Vec<ItemView>, String> {
        let reg = self.r();
        let center = parse(&center_id, RevenueCenterId)?;
        Ok(reg
            .items(center)
            .into_iter()
            .map(|i| ItemView {
                id: i.id.to_string(),
                name: i.name.clone(),
                price: format_money(i.price),
                variants: i
                    .variants
                    .iter()
                    .map(|v| OptionView { id: v.id.to_string(), name: v.name.clone(), price: format_money(v.price) })
                    .collect(),
                option_groups: i
                    .modifier_groups
                    .iter()
                    .filter_map(|g| reg.modifier_group(*g))
                    .map(|g| OptionGroupView {
                        name: g.name.clone(),
                        min: g.min,
                        max: g.max,
                        options: g
                            .modifiers
                            .iter()
                            .map(|m| OptionView { id: m.id.to_string(), name: m.name.clone(), price: format_money(m.price) })
                            .collect(),
                    })
                    .collect(),
                stock: i.track_stock.then(|| reg.stock(i.id)),
            })
            .collect())
    }

    pub fn open_order(&self, center_id: String, fulfillment: String) -> Result<String, String> {
        let f = match fulfillment.as_str() {
            "to_go" => Fulfillment::ToGo,
            "pickup" => Fulfillment::Pickup,
            _ => Fulfillment::Here,
        };
        self.r().open_order(parse(&center_id, RevenueCenterId)?, f).map(|id| id.to_string()).map_err(err)
    }

    pub fn add_item(
        &self,
        order_id: String,
        item_id: String,
        variant_id: Option<String>,
        option_ids: Vec<String>,
        quantity: i64,
    ) -> Result<String, String> {
        let variant = variant_id.map(|v| parse(&v, VariantId)).transpose()?;
        let options = option_ids.iter().map(|o| parse(o, ModifierId)).collect::<Result<Vec<_>, _>>()?;
        self.r()
            .add_item(parse(&order_id, OrderId)?, parse(&item_id, ItemId)?, variant, &options, quantity)
            .map(|id| id.to_string())
            .map_err(err)
    }

    pub fn set_quantity(&self, order_id: String, line_id: String, quantity: i64) -> Result<(), String> {
        self.r().set_quantity(parse(&order_id, OrderId)?, parse(&line_id, LineId)?, quantity).map_err(err)
    }

    pub fn discount(&self, order_id: String, name: String, basis_points: i32, approver_pin: Option<String>) -> Result<(), String> {
        self.r().discount_order(parse(&order_id, OrderId)?, &name, basis_points, approver_pin.as_deref()).map_err(err)
    }

    pub fn send(&self, order_id: String) -> Result<(), String> {
        self.r().send(parse(&order_id, OrderId)?).map_err(err)
    }

    pub fn pay_cash(&self, order_id: String, given_minor: i64) -> Result<CashView, String> {
        let given = Money::from_minor(given_minor, self.r().currency());
        let r = self.r().pay_cash(parse(&order_id, OrderId)?, given).map_err(err)?;
        Ok(CashView { change: format_money(r.change), closed: r.closed })
    }

    pub fn void_order(&self, order_id: String, reason: String) -> Result<(), String> {
        self.r().void_order(parse(&order_id, OrderId)?, &reason).map_err(err)
    }

    #[frb(sync)]
    pub fn order(&self, order_id: String) -> Result<Option<OrderView>, String> {
        let reg = self.r();
        let Some(o) = reg.order(parse(&order_id, OrderId)?) else { return Ok(None) };
        let t = o.totals().map_err(|e| e.to_string())?;
        let lines = o
            .active_lines()
            .zip(&t.lines)
            .map(|(l, lt)| LineView {
                id: l.snapshot.line.to_string(),
                quantity: l.snapshot.quantity,
                name: l.snapshot.name.clone(),
                details: l
                    .snapshot
                    .modifiers
                    .iter()
                    .map(|m| if m.price.minor() == 0 { m.name.clone() } else { format!("{} +{}", m.name, format_money(m.price)) })
                    .collect(),
                amount: format_money(lt.gross),
            })
            .collect();
        Ok(Some(OrderView {
            id: o.id.to_string(),
            number: o.number.clone(),
            lines,
            subtotal: format_money(t.gross),
            discount: (t.discount.minor() != 0).then(|| format_money(Money::from_minor(-t.discount.minor(), t.discount.currency()))),
            taxes: t
                .taxes
                .iter()
                .map(|x| {
                    let pct = format!("{}.{:02}", x.rate_bp / 100, x.rate_bp % 100);
                    let pct = pct.trim_end_matches('0').trim_end_matches('.').to_string();
                    (format!("{} {pct}%{}", x.name, if x.inclusive { " (incl.)" } else { "" }), format_money(x.amount))
                })
                .collect(),
            total: format_money(t.total),
            total_minor: t.total.minor(),
            balance: format_money(o.balance()),
            balance_minor: o.balance().minor(),
            closed: o.status == Status::Closed,
        }))
    }

    pub fn open_shift(&self, float_minor: i64) -> Result<String, String> {
        let float = Money::from_minor(float_minor, self.r().currency());
        self.r().open_shift(float).map(|id| id.to_string()).map_err(err)
    }

    pub fn close_shift(&self, shift_id: String, counted_minor: i64) -> Result<ShiftView, String> {
        let counted = Money::from_minor(counted_minor, self.r().currency());
        let s = self.r().close_shift(parse(&shift_id, ShiftId)?, counted).map_err(err)?;
        Ok(ShiftView {
            id: s.id.to_string(),
            expected: format_money(s.expected()),
            counted: s.counted.map(format_money),
            variance: s.variance().map(format_money),
        })
    }

    #[frb(sync)]
    pub fn report(&self) -> ReportView {
        let r = self.r().report();
        ReportView {
            orders: r.orders,
            total: format_money(r.total),
            tips: format_money(r.tips),
            refunds: format_money(r.refunds),
            by_tender: r.by_tender.into_iter().map(|(k, v)| (k, format_money(v))).collect(),
        }
    }

    /// Saves a checkpoint for fast startup (call when going idle).
    pub fn checkpoint(&self) -> Result<(), String> {
        self.r().checkpoint().map_err(err)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pos_core::money::Currency;

    #[test]
    fn money_formatting() {
        let usd = |m| Money::from_minor(m, Currency::USD);
        assert_eq!(format_money(usd(1234)), "$12.34");
        assert_eq!(format_money(usd(123_456_789)), "$1,234,567.89");
        assert_eq!(format_money(usd(-178)), "−$1.78");
        assert_eq!(format_money(Money::from_minor(1500, Currency::JPY)), "¥1,500");
        assert_eq!(format_money(Money::from_minor(1500, Currency::KWD)), "1.500 KWD");
    }
}
