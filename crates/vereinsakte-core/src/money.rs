//! Geldbeträge in Euro.

use std::fmt;
use std::iter::Sum;
use std::ops::{Add, AddAssign, Mul, Neg, Sub, SubAssign};
use std::str::FromStr;

use rust_decimal::{Decimal, RoundingStrategy};
use serde::de::{self, Visitor};
use serde::{Deserialize, Deserializer, Serialize, Serializer};

/// Geldbetrag in Euro, immer auf ganze Cent (2 Nachkommastellen) gerundet.
///
/// Im JSON eine Zeichenkette mit genau zwei Nachkommastellen, z. B. `"240.00"`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct Money(Decimal);

impl Money {
    pub const ZERO: Money = Money(Decimal::ZERO);

    /// Betrag in Euro; wird kaufmännisch auf Cent gerundet.
    pub fn from_euros(euros: Decimal) -> Self {
        let mut rounded = euros.round_dp_with_strategy(2, RoundingStrategy::MidpointAwayFromZero);
        rounded.rescale(2);
        Self(rounded)
    }

    pub fn from_cents(cents: i64) -> Self {
        Self::from_euros(Decimal::new(cents, 2))
    }

    pub fn euros(self) -> Decimal {
        self.0
    }

    pub fn is_positive(self) -> bool {
        self.0 > Decimal::ZERO
    }

    /// Anteiliger Betrag `self × numerator / denominator`, auf Cent gerundet.
    ///
    /// # Panics
    /// Wenn `denominator` 0 ist.
    pub fn prorate(self, numerator: u32, denominator: u32) -> Self {
        assert!(denominator > 0, "Nenner darf nicht 0 sein");
        Self::from_euros(self.0 * Decimal::from(numerator) / Decimal::from(denominator))
    }

    /// `self × factor` für einen Gleitkomma-Faktor (z. B. Stunden).
    pub fn times_f64(self, factor: f64) -> Self {
        let factor = Decimal::try_from(factor).unwrap_or(Decimal::ZERO);
        self * factor
    }
}

impl Add for Money {
    type Output = Money;
    fn add(self, rhs: Money) -> Money {
        Money::from_euros(self.0 + rhs.0)
    }
}

impl AddAssign for Money {
    fn add_assign(&mut self, rhs: Money) {
        *self = *self + rhs;
    }
}

impl Sub for Money {
    type Output = Money;
    fn sub(self, rhs: Money) -> Money {
        Money::from_euros(self.0 - rhs.0)
    }
}

impl SubAssign for Money {
    fn sub_assign(&mut self, rhs: Money) {
        *self = *self - rhs;
    }
}

impl Neg for Money {
    type Output = Money;
    fn neg(self) -> Money {
        Money::from_euros(-self.0)
    }
}

impl Mul<Decimal> for Money {
    type Output = Money;
    fn mul(self, rhs: Decimal) -> Money {
        Money::from_euros(self.0 * rhs)
    }
}

impl Mul<u32> for Money {
    type Output = Money;
    fn mul(self, rhs: u32) -> Money {
        self * Decimal::from(rhs)
    }
}

impl Sum for Money {
    fn sum<I: Iterator<Item = Money>>(iter: I) -> Money {
        iter.fold(Money::ZERO, Add::add)
    }
}

impl<'a> Sum<&'a Money> for Money {
    fn sum<I: Iterator<Item = &'a Money>>(iter: I) -> Money {
        iter.copied().sum()
    }
}

/// Fehler beim Einlesen eines Betrags.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MoneyParseError {
    Invalid(String),
    TooManyDecimals(String),
}

impl fmt::Display for MoneyParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Invalid(s) => write!(f, "„{s}“ ist kein gültiger Betrag"),
            Self::TooManyDecimals(s) => {
                write!(f, "„{s}“ hat mehr als zwei Nachkommastellen")
            }
        }
    }
}

impl std::error::Error for MoneyParseError {}

impl FromStr for Money {
    type Err = MoneyParseError;

    /// Liest Beträge wie `"240"`, `"240.5"` oder `"240.50"` (Punkt als Dezimaltrenner).
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let value =
            Decimal::from_str_exact(s.trim()).map_err(|_| MoneyParseError::Invalid(s.into()))?;
        if value.scale() > 2 {
            return Err(MoneyParseError::TooManyDecimals(s.into()));
        }
        Ok(Self::from_euros(value))
    }
}

impl fmt::Display for Money {
    /// Deutsche Darstellung, z. B. „240,00 €“.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} €", self.0.to_string().replace('.', ","))
    }
}

impl Serialize for Money {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.0.to_string())
    }
}

impl<'de> Deserialize<'de> for Money {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct MoneyVisitor;

        impl Visitor<'_> for MoneyVisitor {
            type Value = Money;

            fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str("einen Betrag als Zeichenkette, z. B. \"240.00\"")
            }

            fn visit_str<E: de::Error>(self, v: &str) -> Result<Money, E> {
                v.parse().map_err(E::custom)
            }
        }

        deserializer.deserialize_str(MoneyVisitor)
    }
}

#[cfg(test)]
mod tests {
    use rust_decimal_macros::dec;

    use super::*;

    fn eur(s: &str) -> Money {
        s.parse().unwrap()
    }

    #[test]
    fn serializes_as_string_with_two_decimals() {
        let m = Money::from_euros(dec!(75));
        assert_eq!(serde_json::to_string(&m).unwrap(), "\"75.00\"");
        assert_eq!(m.to_string(), "75,00 €");
    }

    #[test]
    fn reads_whole_euros() {
        let m: Money = serde_json::from_str("\"240\"").unwrap();
        assert_eq!(m, Money::from_cents(24000));
        assert_eq!(serde_json::to_string(&m).unwrap(), "\"240.00\"");
    }

    #[test]
    fn rejects_numbers_and_sub_cent_amounts() {
        assert!(serde_json::from_str::<Money>("75.0").is_err());
        assert!(serde_json::from_str::<Money>("\"75.005\"").is_err());
        assert!(serde_json::from_str::<Money>("\"75,00\"").is_err());
    }

    #[test]
    fn arithmetic_rounds_to_cents() {
        assert_eq!(eur("15") * dec!(2.5), eur("37.50"));
        assert_eq!(eur("15").times_f64(2.5), eur("37.50"));
        assert_eq!(eur("240").prorate(6, 12), eur("120.00"));
        assert_eq!(eur("170").prorate(5, 12), eur("70.83"));
        assert_eq!(Money::from_euros(dec!(0.125)), eur("0.13"));
        assert_eq!(Money::from_euros(dec!(-0.125)), eur("-0.13"));
    }

    #[test]
    fn sums_and_negative_amounts() {
        let total: Money = [eur("240"), eur("36"), eur("0.5")].iter().sum();
        assert_eq!(total, eur("276.50"));
        let negative = eur("10") - eur("13.50");
        assert_eq!(negative.to_string(), "-3,50 €");
        assert_eq!(-eur("3.5"), negative);
    }
}
