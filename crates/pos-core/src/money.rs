//! Money as whole minor units (e.g. cents). Never floating point.

use std::fmt;

use serde::{Deserialize, Serialize};

/// An ISO-4217 currency code and how many minor units it has.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Currency {
    code: [u8; 3],
    exponent: u8,
}

impl Currency {
    pub const USD: Self = Self::new(*b"USD", 2);
    pub const EUR: Self = Self::new(*b"EUR", 2);
    pub const GBP: Self = Self::new(*b"GBP", 2);
    pub const CAD: Self = Self::new(*b"CAD", 2);
    pub const JPY: Self = Self::new(*b"JPY", 0);
    pub const KWD: Self = Self::new(*b"KWD", 3);

    /// `exponent` is the number of minor-unit digits: 2 for cents, 0 for yen.
    pub const fn new(code: [u8; 3], exponent: u8) -> Self {
        Self { code, exponent }
    }

    pub fn code(&self) -> &str {
        std::str::from_utf8(&self.code).unwrap_or("???")
    }

    pub const fn exponent(&self) -> u8 {
        self.exponent
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MoneyError {
    CurrencyMismatch,
    Overflow,
    NoParts,
    NegativeWeight,
    ZeroTotalWeight,
}

impl fmt::Display for MoneyError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::CurrencyMismatch => "currencies don't match",
            Self::Overflow => "amount out of range",
            Self::NoParts => "can't split into zero parts",
            Self::NegativeWeight => "weights must not be negative",
            Self::ZeroTotalWeight => "weights must not all be zero",
        })
    }
}

impl std::error::Error for MoneyError {}

/// How to round a fractional minor unit. Each tax rule names its own (§7).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Rounding {
    /// 0.5 rounds away from zero.
    HalfUp,
    /// 0.5 rounds to the nearest even number ("banker's rounding").
    HalfEven,
    /// Always toward zero.
    Down,
}

/// An amount in minor units of one currency.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Money {
    minor: i64,
    currency: Currency,
}

impl Money {
    pub const fn from_minor(minor: i64, currency: Currency) -> Self {
        Self { minor, currency }
    }

    pub const fn zero(currency: Currency) -> Self {
        Self { minor: 0, currency }
    }

    pub const fn minor(&self) -> i64 {
        self.minor
    }

    pub const fn currency(&self) -> Currency {
        self.currency
    }

    pub fn checked_add(self, other: Self) -> Result<Self, MoneyError> {
        self.same_currency(other)?;
        let minor = self.minor.checked_add(other.minor).ok_or(MoneyError::Overflow)?;
        Ok(Self { minor, ..self })
    }

    pub fn checked_sub(self, other: Self) -> Result<Self, MoneyError> {
        self.same_currency(other)?;
        let minor = self.minor.checked_sub(other.minor).ok_or(MoneyError::Overflow)?;
        Ok(Self { minor, ..self })
    }

    pub fn checked_mul(self, quantity: i64) -> Result<Self, MoneyError> {
        let minor = self.minor.checked_mul(quantity).ok_or(MoneyError::Overflow)?;
        Ok(Self { minor, ..self })
    }

    /// `self × basis_points / 10 000`, rounded. 825 bp = 8.25 %.
    pub fn percent(self, basis_points: i32, rounding: Rounding) -> Result<Self, MoneyError> {
        let numerator = i128::from(self.minor) * i128::from(basis_points);
        let minor = round_div(numerator, 10_000, rounding);
        let minor = i64::try_from(minor).map_err(|_| MoneyError::Overflow)?;
        Ok(Self { minor, ..self })
    }

    /// Splits into `parts` amounts that differ by at most one minor unit and
    /// always add up exactly: $10.00 / 3 = $3.34 + $3.33 + $3.33.
    pub fn split_evenly(self, parts: usize) -> Result<Vec<Self>, MoneyError> {
        if parts == 0 {
            return Err(MoneyError::NoParts);
        }
        self.allocate(&vec![1; parts])
    }

    /// Splits in proportion to `weights` using the largest-remainder method.
    /// The parts always add up exactly to `self`; ties go to earlier parts,
    /// so the result is deterministic on every device.
    pub fn allocate(self, weights: &[i64]) -> Result<Vec<Self>, MoneyError> {
        if weights.is_empty() {
            return Err(MoneyError::NoParts);
        }
        if weights.iter().any(|w| *w < 0) {
            return Err(MoneyError::NegativeWeight);
        }
        let total_weight: i128 = weights.iter().map(|w| i128::from(*w)).sum();
        if total_weight == 0 {
            return Err(MoneyError::ZeroTotalWeight);
        }

        let amount = i128::from(self.minor);
        let sign = amount.signum();
        let magnitude = amount.abs();

        let mut shares = Vec::with_capacity(weights.len());
        let mut remainders = Vec::with_capacity(weights.len());
        for (i, w) in weights.iter().enumerate() {
            let product = magnitude * i128::from(*w);
            shares.push(product / total_weight);
            remainders.push((product % total_weight, i));
        }

        let mut leftover = magnitude - shares.iter().sum::<i128>();
        // Largest remainder first; ties broken by position.
        remainders.sort_by(|a, b| b.0.cmp(&a.0).then(a.1.cmp(&b.1)));
        for (_, i) in remainders {
            if leftover == 0 {
                break;
            }
            shares[i] += 1;
            leftover -= 1;
        }

        shares
            .into_iter()
            .map(|s| {
                let minor = i64::try_from(s * sign).map_err(|_| MoneyError::Overflow)?;
                Ok(Self { minor, ..self })
            })
            .collect()
    }

    fn same_currency(&self, other: Self) -> Result<(), MoneyError> {
        if self.currency == other.currency { Ok(()) } else { Err(MoneyError::CurrencyMismatch) }
    }
}

fn round_div(numerator: i128, denominator: i128, rounding: Rounding) -> i128 {
    let quotient = numerator / denominator;
    let remainder = numerator % denominator;
    if remainder == 0 {
        return quotient;
    }
    let away = numerator.signum();
    let twice = remainder.abs() * 2;
    let bump = match rounding {
        Rounding::Down => false,
        Rounding::HalfUp => twice >= denominator,
        Rounding::HalfEven => twice > denominator || (twice == denominator && quotient % 2 != 0),
    };
    if bump { quotient + away } else { quotient }
}

impl fmt::Display for Money {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let exp = u32::from(self.currency.exponent);
        let sign = if self.minor < 0 { "-" } else { "" };
        let abs = self.minor.unsigned_abs();
        if exp == 0 {
            return write!(f, "{sign}{abs} {}", self.currency.code());
        }
        let scale = 10u64.pow(exp);
        write!(
            f,
            "{sign}{}.{:0width$} {}",
            abs / scale,
            abs % scale,
            self.currency.code(),
            width = exp as usize
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    fn usd(minor: i64) -> Money {
        Money::from_minor(minor, Currency::USD)
    }

    #[test]
    fn ten_dollars_three_ways() {
        let parts = usd(1000).split_evenly(3).unwrap();
        assert_eq!(parts, vec![usd(334), usd(333), usd(333)]);
    }

    #[test]
    fn negative_amounts_split_symmetrically() {
        let parts = usd(-1000).split_evenly(3).unwrap();
        assert_eq!(parts, vec![usd(-334), usd(-333), usd(-333)]);
    }

    #[test]
    fn allocation_by_weight() {
        // A $1.00 discount spread over lines priced 3, 3 and 4.
        let parts = usd(100).allocate(&[3, 3, 4]).unwrap();
        assert_eq!(parts, vec![usd(30), usd(30), usd(40)]);
        let parts = usd(100).allocate(&[1, 1, 1]).unwrap();
        assert_eq!(parts, vec![usd(34), usd(33), usd(33)]);
    }

    #[test]
    fn invalid_allocations() {
        assert_eq!(usd(1).split_evenly(0), Err(MoneyError::NoParts));
        assert_eq!(usd(1).allocate(&[1, -1]), Err(MoneyError::NegativeWeight));
        assert_eq!(usd(1).allocate(&[0, 0]), Err(MoneyError::ZeroTotalWeight));
    }

    #[test]
    fn percent_rounding_modes() {
        // 8.25 % of $10.10 = 83.325 cents
        assert_eq!(usd(1010).percent(825, Rounding::HalfUp).unwrap(), usd(83));
        // 5 % of $0.50 = 2.5 cents
        assert_eq!(usd(50).percent(500, Rounding::HalfUp).unwrap(), usd(3));
        assert_eq!(usd(50).percent(500, Rounding::HalfEven).unwrap(), usd(2));
        assert_eq!(usd(70).percent(500, Rounding::HalfEven).unwrap(), usd(4));
        assert_eq!(usd(50).percent(500, Rounding::Down).unwrap(), usd(2));
        assert_eq!(usd(-50).percent(500, Rounding::HalfUp).unwrap(), usd(-3));
    }

    #[test]
    fn currencies_never_mix() {
        let eur = Money::from_minor(100, Currency::EUR);
        assert_eq!(usd(100).checked_add(eur), Err(MoneyError::CurrencyMismatch));
    }

    #[test]
    fn overflow_is_an_error_not_a_wrap() {
        assert_eq!(usd(i64::MAX).checked_add(usd(1)), Err(MoneyError::Overflow));
        assert_eq!(usd(i64::MAX).checked_mul(2), Err(MoneyError::Overflow));
    }

    #[test]
    fn display() {
        assert_eq!(usd(1234).to_string(), "12.34 USD");
        assert_eq!(usd(-5).to_string(), "-0.05 USD");
        assert_eq!(Money::from_minor(500, Currency::JPY).to_string(), "500 JPY");
        assert_eq!(Money::from_minor(1500, Currency::KWD).to_string(), "1.500 KWD");
    }

    proptest! {
        #[test]
        fn allocation_always_sums_exactly(
            amount in -1_000_000_000_000i64..1_000_000_000_000,
            weights in prop::collection::vec(0i64..10_000, 1..20),
        ) {
            prop_assume!(weights.iter().any(|w| *w > 0));
            let parts = usd(amount).allocate(&weights).unwrap();
            prop_assert_eq!(parts.len(), weights.len());
            prop_assert_eq!(parts.iter().map(Money::minor).sum::<i64>(), amount);
            // Every part has the same sign as the whole (or is zero).
            prop_assert!(parts.iter().all(|p| p.minor() == 0 || p.minor().signum() == amount.signum()));
        }

        #[test]
        fn even_split_parts_differ_by_at_most_one(amount in -1_000_000_000i64..1_000_000_000, parts in 1usize..50) {
            let split = usd(amount).split_evenly(parts).unwrap();
            let max = split.iter().map(Money::minor).max().unwrap();
            let min = split.iter().map(Money::minor).min().unwrap();
            prop_assert!(max - min <= 1);
            prop_assert_eq!(split.iter().map(Money::minor).sum::<i64>(), amount);
        }

        #[test]
        fn allocation_is_deterministic(amount in any::<i32>(), weights in prop::collection::vec(1i64..100, 1..10)) {
            let a = usd(i64::from(amount)).allocate(&weights).unwrap();
            let b = usd(i64::from(amount)).allocate(&weights).unwrap();
            prop_assert_eq!(a, b);
        }

        #[test]
        fn percent_is_within_one_unit_of_exact(amount in -1_000_000_000i64..1_000_000_000, bp in 0i32..100_000) {
            let exact_times_10k = i128::from(amount) * i128::from(bp);
            for mode in [Rounding::HalfUp, Rounding::HalfEven, Rounding::Down] {
                let got = i128::from(usd(amount).percent(bp, mode).unwrap().minor());
                prop_assert!((got * 10_000 - exact_times_10k).abs() < 10_000);
            }
        }
    }
}
