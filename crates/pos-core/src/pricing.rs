//! The pricing engine (PLAN §7): a pure, deterministic function from an
//! order's lines (with the prices and taxes they were sold at) to totals.
//!
//! It never reads the live catalog. Every line carries a snapshot of what it
//! was sold for, so any peer can re-check any sale, years later, with the
//! same result (§6 "version-tolerant validation").

use serde::{Deserialize, Serialize};

use crate::{
    catalog::{DiscountAmount, RoundingScope, TaxRate},
    ids::{LineId, TaxId},
    money::{Currency, Money, MoneyError, Rounding},
};

/// Which version of the pricing rules an order uses. Recorded when the order
/// opens. A rule change adds a new version; old versions stay forever so old
/// sales always re-check correctly.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PricingRules {
    V1,
}

impl PricingRules {
    pub const CURRENT: Self = Self::V1;
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModifierSnapshot {
    pub name: String,
    /// Added to the unit price, per unit.
    pub price: Money,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DiscountSnapshot {
    pub name: String,
    pub amount: DiscountAmount,
}

/// One line as the pricing engine sees it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LineInput {
    pub line_id: LineId,
    pub quantity: i64,
    pub unit_price: Money,
    pub modifiers: Vec<ModifierSnapshot>,
    pub taxes: Vec<TaxRate>,
    pub discounts: Vec<DiscountSnapshot>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LineTax {
    pub tax: TaxId,
    pub amount: Money,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LineTotals {
    pub line_id: LineId,
    /// (unit price + modifiers) × quantity.
    pub gross: Money,
    /// Line and order discounts allocated to this line.
    pub discount: Money,
    /// gross − discount. For inclusive taxes this still contains the tax.
    pub net: Money,
    pub taxes: Vec<LineTax>,
    /// What the customer pays for this line: net + exclusive taxes.
    pub total: Money,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TaxTotal {
    pub tax: TaxId,
    pub name: String,
    pub rate_bp: i32,
    pub inclusive: bool,
    /// The amount the tax was charged on.
    pub base: Money,
    pub amount: Money,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Totals {
    pub lines: Vec<LineTotals>,
    pub gross: Money,
    pub discount: Money,
    pub net: Money,
    pub taxes: Vec<TaxTotal>,
    /// Exclusive taxes only; inclusive taxes are already inside `net`.
    pub exclusive_tax: Money,
    pub total: Money,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PricingError {
    Money(MoneyError),
    NegativeQuantity,
    /// The same tax appears on two lines with different settings.
    InconsistentTax(TaxId),
}

impl From<MoneyError> for PricingError {
    fn from(e: MoneyError) -> Self {
        Self::Money(e)
    }
}

impl std::fmt::Display for PricingError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Money(e) => write!(f, "{e}"),
            Self::NegativeQuantity => f.write_str("quantity must not be negative"),
            Self::InconsistentTax(id) => {
                write!(f, "tax {id} has different settings on different lines")
            }
        }
    }
}

impl std::error::Error for PricingError {}

/// Prices an order.
///
/// `order_discounts` are spread over the lines in proportion to their value
/// after line discounts, so refunds of single lines give back exactly the
/// right share.
pub fn price(
    rules: PricingRules,
    currency: Currency,
    lines: &[LineInput],
    order_discounts: &[DiscountSnapshot],
) -> Result<Totals, PricingError> {
    match rules {
        PricingRules::V1 => price_v1(currency, lines, order_discounts),
    }
}

fn price_v1(
    currency: Currency,
    lines: &[LineInput],
    order_discounts: &[DiscountSnapshot],
) -> Result<Totals, PricingError> {
    let zero = Money::zero(currency);

    // 1. Gross and line discounts.
    let mut gross = Vec::with_capacity(lines.len());
    let mut after_line_discounts = Vec::with_capacity(lines.len());
    let mut line_discount = Vec::with_capacity(lines.len());
    for line in lines {
        if line.quantity < 0 {
            return Err(PricingError::NegativeQuantity);
        }
        let mut unit = line.unit_price;
        for m in &line.modifiers {
            unit = unit.checked_add(m.price)?;
        }
        let g = unit.checked_mul(line.quantity)?;
        let mut d = zero;
        for disc in &line.discounts {
            let remaining = g.checked_sub(d)?;
            d = d.checked_add(discount_amount(disc.amount, remaining)?)?;
        }
        gross.push(g);
        line_discount.push(d);
        after_line_discounts.push(g.checked_sub(d)?);
    }

    // 2. Order discounts, allocated across lines by value.
    let subtotal = sum(zero, &after_line_discounts)?;
    let mut order_discount_total = zero;
    for disc in order_discounts {
        let remaining = subtotal.checked_sub(order_discount_total)?;
        order_discount_total =
            order_discount_total.checked_add(discount_amount(disc.amount, remaining)?)?;
    }
    let allocated = allocate_by(order_discount_total, &after_line_discounts)?;

    let mut net = Vec::with_capacity(lines.len());
    let mut discount = Vec::with_capacity(lines.len());
    for i in 0..lines.len() {
        let d = line_discount[i].checked_add(allocated[i])?;
        discount.push(d);
        net.push(gross[i].checked_sub(d)?);
    }

    // 3. Taxes, in order of first appearance so output is deterministic.
    let mut tax_defs: Vec<&TaxRate> = Vec::new();
    for line in lines {
        for t in &line.taxes {
            match tax_defs.iter().find(|d| d.id == t.id) {
                Some(existing) if *existing != t => {
                    return Err(PricingError::InconsistentTax(t.id));
                }
                Some(_) => {}
                None => tax_defs.push(t),
            }
        }
    }

    let mut line_taxes: Vec<Vec<LineTax>> = vec![Vec::new(); lines.len()];
    let mut tax_totals = Vec::new();
    // Non-compound first, so compound taxes can include them in their base.
    let ordered: Vec<&TaxRate> = tax_defs
        .iter()
        .filter(|t| !t.compound)
        .chain(tax_defs.iter().filter(|t| t.compound))
        .copied()
        .collect();

    for tax in ordered {
        let mut bases = Vec::with_capacity(lines.len());
        for (i, line) in lines.iter().enumerate() {
            if !line.taxes.iter().any(|t| t.id == tax.id) {
                bases.push(None);
                continue;
            }
            let mut base = net[i];
            if tax.compound {
                for lt in &line_taxes[i] {
                    let def = tax_defs.iter().find(|d| d.id == lt.tax).expect("known tax");
                    if !def.inclusive && !def.compound {
                        base = base.checked_add(lt.amount)?;
                    }
                }
            }
            bases.push(Some(base));
        }

        let amounts = tax_amounts(tax, &bases, zero)?;
        let mut base_total = zero;
        let mut amount_total = zero;
        for (i, amount) in amounts.into_iter().enumerate() {
            if let (Some(base), Some(amount)) = (bases[i], amount) {
                base_total = base_total.checked_add(base)?;
                amount_total = amount_total.checked_add(amount)?;
                line_taxes[i].push(LineTax { tax: tax.id, amount });
            }
        }
        tax_totals.push(TaxTotal {
            tax: tax.id,
            name: tax.name.clone(),
            rate_bp: tax.rate_bp,
            inclusive: tax.inclusive,
            base: base_total,
            amount: amount_total,
        });
    }

    // 4. Line and order totals.
    let mut out_lines = Vec::with_capacity(lines.len());
    let mut exclusive_tax = zero;
    for (i, line) in lines.iter().enumerate() {
        let mut total = net[i];
        for lt in &line_taxes[i] {
            if !tax_defs.iter().any(|d| d.id == lt.tax && d.inclusive) {
                total = total.checked_add(lt.amount)?;
                exclusive_tax = exclusive_tax.checked_add(lt.amount)?;
            }
        }
        out_lines.push(LineTotals {
            line_id: line.line_id,
            gross: gross[i],
            discount: discount[i],
            net: net[i],
            taxes: std::mem::take(&mut line_taxes[i]),
            total,
        });
    }

    let net_total = sum(zero, &net)?;
    Ok(Totals {
        gross: sum(zero, &gross)?,
        discount: sum(zero, &discount)?,
        net: net_total,
        exclusive_tax,
        total: net_total.checked_add(exclusive_tax)?,
        taxes: tax_totals,
        lines: out_lines,
    })
}

fn discount_amount(amount: DiscountAmount, of: Money) -> Result<Money, MoneyError> {
    let d = match amount {
        DiscountAmount::Percent(bp) => of.percent(bp, Rounding::HalfUp)?,
        DiscountAmount::Fixed(m) => m,
    };
    // A discount never makes something cost less than nothing.
    Ok(if d.minor() > of.minor() {
        of
    } else if d.minor() < 0 {
        Money::zero(of.currency())
    } else {
        d
    })
}

fn sum(zero: Money, values: &[Money]) -> Result<Money, MoneyError> {
    values.iter().try_fold(zero, |acc, v| acc.checked_add(*v))
}

/// Splits `amount` in proportion to `weights` (which may all be zero).
fn allocate_by(amount: Money, weights: &[Money]) -> Result<Vec<Money>, MoneyError> {
    let zero = Money::zero(amount.currency());
    if weights.is_empty() || amount.minor() == 0 || weights.iter().all(|w| w.minor() == 0) {
        return Ok(vec![zero; weights.len()]);
    }
    amount.allocate(&weights.iter().map(Money::minor).collect::<Vec<_>>())
}

/// Tax for each line that carries it (`None` where it doesn't apply).
fn tax_amounts(
    tax: &TaxRate,
    bases: &[Option<Money>],
    zero: Money,
) -> Result<Vec<Option<Money>>, MoneyError> {
    // Exact tax as a fraction: numerator / denominator minor units.
    let (num_factor, denominator) = if tax.inclusive {
        (i128::from(tax.rate_bp), 10_000 + i128::from(tax.rate_bp))
    } else {
        (i128::from(tax.rate_bp), 10_000)
    };
    let round = |numerator: i128| -> Result<Money, MoneyError> {
        let minor = round_div(numerator, denominator, tax.rounding);
        Ok(Money::from_minor(
            i64::try_from(minor).map_err(|_| MoneyError::Overflow)?,
            zero.currency(),
        ))
    };

    match tax.scope {
        RoundingScope::PerLine => bases
            .iter()
            .map(|b| b.map(|b| round(i128::from(b.minor()) * num_factor)).transpose())
            .collect(),
        RoundingScope::PerOrder => {
            let applicable: Vec<Money> = bases.iter().flatten().copied().collect();
            let exact: i128 = applicable.iter().map(|b| i128::from(b.minor()) * num_factor).sum();
            let total = round(exact)?;
            let shares = allocate_by(total, &applicable)?;
            let mut shares = shares.into_iter();
            Ok(bases
                .iter()
                .map(|b| b.map(|_| shares.next().expect("one share per base")))
                .collect())
        }
    }
}

fn round_div(numerator: i128, denominator: i128, rounding: Rounding) -> i128 {
    let q = numerator / denominator;
    let r = numerator % denominator;
    if r == 0 {
        return q;
    }
    let twice = r.abs() * 2;
    let bump = match rounding {
        Rounding::Down => false,
        Rounding::HalfUp => twice >= denominator,
        Rounding::HalfEven => twice > denominator || (twice == denominator && q % 2 != 0),
    };
    if bump { q + numerator.signum() } else { q }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    const USD: Currency = Currency::USD;

    fn usd(minor: i64) -> Money {
        Money::from_minor(minor, USD)
    }

    fn tax(id: u128, bp: i32, inclusive: bool, compound: bool, scope: RoundingScope) -> TaxRate {
        TaxRate {
            id: TaxId::from_u128(id),
            name: format!("Tax {id}"),
            rate_bp: bp,
            inclusive,
            compound,
            rounding: Rounding::HalfUp,
            scope,
        }
    }

    fn line(id: u128, qty: i64, price: i64, taxes: Vec<TaxRate>) -> LineInput {
        LineInput {
            line_id: LineId::from_u128(id),
            quantity: qty,
            unit_price: usd(price),
            modifiers: vec![],
            taxes,
            discounts: vec![],
        }
    }

    #[test]
    fn exclusive_sales_tax() {
        // Two coffees at $3.50 with 8.25 % sales tax: 7.00 + 0.5775 → 0.58.
        let t = tax(1, 825, false, false, RoundingScope::PerLine);
        let totals = price(PricingRules::V1, USD, &[line(1, 2, 350, vec![t])], &[]).unwrap();
        assert_eq!(totals.net, usd(700));
        assert_eq!(totals.exclusive_tax, usd(58));
        assert_eq!(totals.total, usd(758));
    }

    #[test]
    fn inclusive_vat() {
        // £12.00 including 20 % VAT: VAT = 12.00 × 20/120 = 2.00.
        let t = tax(1, 2000, true, false, RoundingScope::PerLine);
        let gbp = Currency::GBP;
        let l = LineInput { unit_price: Money::from_minor(1200, gbp), ..line(1, 1, 0, vec![t]) };
        let totals = price(PricingRules::V1, gbp, &[l], &[]).unwrap();
        assert_eq!(totals.total, Money::from_minor(1200, gbp));
        assert_eq!(totals.exclusive_tax, Money::zero(gbp));
        assert_eq!(totals.taxes[0].amount, Money::from_minor(200, gbp));
    }

    #[test]
    fn modifiers_and_line_discount() {
        let mut l = line(1, 2, 400, vec![]);
        l.modifiers.push(ModifierSnapshot { name: "Extra shot".into(), price: usd(75) });
        l.discounts
            .push(DiscountSnapshot { name: "10 %".into(), amount: DiscountAmount::Percent(1000) });
        let totals = price(PricingRules::V1, USD, &[l], &[]).unwrap();
        assert_eq!(totals.gross, usd(950));
        assert_eq!(totals.discount, usd(95));
        assert_eq!(totals.total, usd(855));
    }

    #[test]
    fn order_discount_is_allocated_by_line_value() {
        let lines = [line(1, 1, 300, vec![]), line(2, 1, 700, vec![])];
        let d = DiscountSnapshot { name: "$1 off".into(), amount: DiscountAmount::Fixed(usd(100)) };
        let totals = price(PricingRules::V1, USD, &lines, &[d]).unwrap();
        assert_eq!(totals.lines[0].discount, usd(30));
        assert_eq!(totals.lines[1].discount, usd(70));
        assert_eq!(totals.total, usd(900));
    }

    #[test]
    fn discounts_never_go_below_zero() {
        let d =
            DiscountSnapshot { name: "Huge".into(), amount: DiscountAmount::Fixed(usd(10_000)) };
        let totals = price(PricingRules::V1, USD, &[line(1, 1, 500, vec![])], &[d]).unwrap();
        assert_eq!(totals.total, usd(0));
    }

    #[test]
    fn per_order_rounding_differs_from_per_line() {
        // Three $0.10 items at 5 %: per line 0.005 → 0.01 each = 0.03; per order 0.015 → 0.02.
        let per_line = tax(1, 500, false, false, RoundingScope::PerLine);
        let per_order = tax(1, 500, false, false, RoundingScope::PerOrder);
        let lines =
            |t: &TaxRate| (1..=3).map(|i| line(i, 1, 10, vec![t.clone()])).collect::<Vec<_>>();
        let a = price(PricingRules::V1, USD, &lines(&per_line), &[]).unwrap();
        let b = price(PricingRules::V1, USD, &lines(&per_order), &[]).unwrap();
        assert_eq!(a.exclusive_tax, usd(3));
        assert_eq!(b.exclusive_tax, usd(2));
        // Per-order tax is still assigned to lines, and adds up.
        assert_eq!(b.lines.iter().map(|l| l.taxes[0].amount.minor()).sum::<i64>(), 2);
    }

    #[test]
    fn compound_tax_includes_other_exclusive_taxes() {
        // Quebec-style: 5 % GST, then 9.975 % QST on price + GST (historic compound rule).
        let gst = tax(1, 500, false, false, RoundingScope::PerLine);
        let qst = tax(2, 998, false, true, RoundingScope::PerLine);
        let totals =
            price(PricingRules::V1, USD, &[line(1, 1, 10_000, vec![qst, gst])], &[]).unwrap();
        assert_eq!(totals.taxes[0].amount, usd(500)); // GST first, regardless of line order
        assert_eq!(totals.taxes[1].base, usd(10_500));
        assert_eq!(totals.taxes[1].amount, usd(1048)); // 105.00 × 9.98 % = 10.479
        assert_eq!(totals.total, usd(11_548));
    }

    #[test]
    fn inconsistent_tax_definitions_are_rejected() {
        let a = tax(1, 500, false, false, RoundingScope::PerLine);
        let b = tax(1, 600, false, false, RoundingScope::PerLine);
        let err = price(
            PricingRules::V1,
            USD,
            &[line(1, 1, 100, vec![a]), line(2, 1, 100, vec![b])],
            &[],
        );
        assert_eq!(err, Err(PricingError::InconsistentTax(TaxId::from_u128(1))));
    }

    #[test]
    fn empty_order_is_zero() {
        let totals = price(PricingRules::V1, USD, &[], &[]).unwrap();
        assert_eq!(totals.total, usd(0));
    }

    fn arb_line() -> impl Strategy<Value = LineInput> {
        (any::<u64>(), 0i64..20, 0i64..100_000, prop::bool::ANY, prop::bool::ANY, 0i32..3000)
            .prop_map(|(id, qty, price, inclusive, per_order, bp)| {
                let scope =
                    if per_order { RoundingScope::PerOrder } else { RoundingScope::PerLine };
                line(
                    u128::from(id),
                    qty,
                    price,
                    vec![tax(
                        inclusive as u128 + 2 * per_order as u128 + 10,
                        bp,
                        inclusive,
                        false,
                        scope,
                    )],
                )
            })
    }

    proptest! {
        #[test]
        fn totals_are_consistent(
            lines in prop::collection::vec(arb_line(), 0..12),
            order_pct in 0i32..10_000,
        ) {
            // Same tax id must mean the same rate: normalize rates per id.
            let mut lines = lines;
            let rates: std::collections::HashMap<_, _> = lines.iter().flat_map(|l| l.taxes.iter().map(|t| (t.id, t.rate_bp))).collect();
            for l in &mut lines { for t in &mut l.taxes { t.rate_bp = rates[&t.id]; } }

            let d = DiscountSnapshot { name: "x".into(), amount: DiscountAmount::Percent(order_pct) };
            let t = price(PricingRules::V1, USD, &lines, &[d]).unwrap();

            prop_assert!(t.total.minor() >= 0);
            prop_assert!(t.net.minor() >= 0);
            prop_assert_eq!(t.lines.iter().map(|l| l.total.minor()).sum::<i64>(), t.total.minor());
            prop_assert_eq!(t.lines.iter().map(|l| l.net.minor()).sum::<i64>(), t.net.minor());
            prop_assert_eq!(t.gross.minor() - t.discount.minor(), t.net.minor());
            prop_assert_eq!(t.taxes.iter().filter(|x| !x.inclusive).map(|x| x.amount.minor()).sum::<i64>(), t.exclusive_tax.minor());
            for l in &t.lines { prop_assert!(l.net.minor() >= 0 && l.discount.minor() <= l.gross.minor()); }
            // Deterministic.
            prop_assert_eq!(price(PricingRules::V1, USD, &lines, &[DiscountSnapshot { name: "x".into(), amount: DiscountAmount::Percent(order_pct) }]).unwrap(), t);
        }
    }
}
