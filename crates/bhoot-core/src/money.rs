use crate::error::CoreError;
use serde::{Deserialize, Serialize};
use std::fmt;

/// Exact integer monetary representation denominated in Indian Paisa (1 Rupee = 100 Paisa).
///
/// Floating-point operations are strictly forbidden across the accounting engine.
/// All internal calculations utilize signed 64-bit integer checked arithmetic.
#[derive(
    Debug, Default, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize,
)]
pub struct Money(i64);

impl Money {
    /// Zero money constant.
    pub const ZERO: Self = Self(0);

    /// One Rupee constant (100 paisa).
    pub const ONE_RUPEE: Self = Self(100);

    /// Construct Money directly from an integer quantity of paisa.
    #[must_use]
    pub const fn from_paisa(paisa: i64) -> Self {
        Self(paisa)
    }

    /// Construct Money from discrete Rupees and Paisa components.
    ///
    /// Note: To construct negative fractional amounts with zero whole Rupees (e.g. -₹0.50),
    /// use [`Self::from_rupees_and_paisa_signed`] or [`Self::from_paisa`], since `-0 == 0`.
    ///
    /// # Errors
    /// Returns `CoreError::InvalidAmount` if `paisa >= 100` or if multiplication overflows.
    pub fn from_rupees_and_paisa(rupees: i64, paisa: u32) -> Result<Self, CoreError> {
        if paisa >= 100 {
            return Err(CoreError::InvalidAmount(format!(
                "Paisa component must be between 0 and 99, got {paisa}"
            )));
        }

        let rupee_paisa = rupees
            .checked_mul(100)
            .ok_or_else(|| CoreError::ArithmeticOverflow("Rupees to paisa overflow".into()))?;

        let paisa_signed = i64::from(paisa);
        let total_paisa = if rupees < 0 {
            rupee_paisa.checked_sub(paisa_signed).ok_or_else(|| {
                CoreError::ArithmeticOverflow("Negative paisa subtraction overflow".into())
            })?
        } else {
            rupee_paisa
                .checked_add(paisa_signed)
                .ok_or_else(|| CoreError::ArithmeticOverflow("Paisa addition overflow".into()))?
        };

        Ok(Self(total_paisa))
    }

    /// Construct Money from explicit signed components, supporting negative fractional amounts (e.g. -₹0.50).
    ///
    /// # Errors
    /// Returns `CoreError::InvalidAmount` if `paisa >= 100` or `CoreError::ArithmeticOverflow`.
    pub fn from_rupees_and_paisa_signed(
        is_negative: bool,
        rupees: u64,
        paisa: u32,
    ) -> Result<Self, CoreError> {
        if paisa >= 100 {
            return Err(CoreError::InvalidAmount(format!(
                "Paisa component must be between 0 and 99, got {paisa}"
            )));
        }

        let rupee_paisa = rupees
            .checked_mul(100)
            .ok_or_else(|| CoreError::ArithmeticOverflow("Rupees to paisa overflow".into()))?;

        let total_unsigned = rupee_paisa
            .checked_add(u64::from(paisa))
            .ok_or_else(|| CoreError::ArithmeticOverflow("Paisa addition overflow".into()))?;

        if total_unsigned > (i64::MAX as u64) {
            return Err(CoreError::ArithmeticOverflow(
                "Amount exceeds i64::MAX".into(),
            ));
        }

        let total_signed = total_unsigned as i64;
        if is_negative {
            Ok(Self(-total_signed))
        } else {
            Ok(Self(total_signed))
        }
    }

    /// Return the raw value in Paisa.
    #[must_use]
    pub const fn as_paisa(&self) -> i64 {
        self.0
    }

    /// Split Money into signed Rupees and unsigned Paisa components.
    #[must_use]
    pub fn to_rupees_and_paisa(&self) -> (i64, u32) {
        let rupees = self.0 / 100;
        let remainder = self.0 % 100;
        let paisa = remainder.unsigned_abs() as u32;
        (rupees, paisa)
    }

    /// Check if the monetary amount is zero.
    #[must_use]
    pub const fn is_zero(&self) -> bool {
        self.0 == 0
    }

    /// Check if the monetary amount is strictly positive (> 0).
    #[must_use]
    pub const fn is_positive(&self) -> bool {
        self.0 > 0
    }

    /// Check if the monetary amount is strictly negative (< 0).
    #[must_use]
    pub const fn is_negative(&self) -> bool {
        self.0 < 0
    }

    /// Absolute monetary value.
    ///
    /// # Errors
    /// Returns `CoreError::ArithmeticOverflow` on `i64::MIN`.
    pub fn abs(self) -> Result<Self, CoreError> {
        self.0
            .checked_abs()
            .map(Self)
            .ok_or_else(|| CoreError::ArithmeticOverflow("abs() overflow on i64::MIN".into()))
    }

    /// Checked addition.
    ///
    /// # Errors
    /// Returns `CoreError::ArithmeticOverflow` on overflow.
    pub fn checked_add(self, rhs: Self) -> Result<Self, CoreError> {
        self.0
            .checked_add(rhs.0)
            .map(Self)
            .ok_or_else(|| CoreError::ArithmeticOverflow(format!("Overflow adding {self} + {rhs}")))
    }

    /// Checked subtraction.
    ///
    /// # Errors
    /// Returns `CoreError::ArithmeticOverflow` on overflow/underflow.
    pub fn checked_sub(self, rhs: Self) -> Result<Self, CoreError> {
        self.0.checked_sub(rhs.0).map(Self).ok_or_else(|| {
            CoreError::ArithmeticOverflow(format!("Underflow subtracting {self} - {rhs}"))
        })
    }

    /// Checked scalar multiplication.
    ///
    /// # Errors
    /// Returns `CoreError::ArithmeticOverflow` on overflow.
    pub fn checked_mul(self, rhs: i64) -> Result<Self, CoreError> {
        self.0.checked_mul(rhs).map(Self).ok_or_else(|| {
            CoreError::ArithmeticOverflow(format!("Overflow multiplying {self} * {rhs}"))
        })
    }

    /// Checked integer division.
    ///
    /// # Errors
    /// Returns `CoreError::DivisionByZero` or `CoreError::ArithmeticOverflow`.
    pub fn checked_div(self, rhs: i64) -> Result<Self, CoreError> {
        if rhs == 0 {
            return Err(CoreError::DivisionByZero);
        }
        self.0
            .checked_div(rhs)
            .map(Self)
            .ok_or_else(|| CoreError::ArithmeticOverflow("Overflow in checked_div".into()))
    }

    /// Format according to the Indian Numbering System (e.g. `₹1,23,456.78` or `-₹50.00`).
    #[must_use]
    pub fn format_inr(&self) -> String {
        let (rupees, paisa) = self.to_rupees_and_paisa();
        let abs_rupees = rupees.unsigned_abs();
        let rupees_str = abs_rupees.to_string();

        let formatted_rupees = if rupees_str.len() <= 3 {
            rupees_str
        } else {
            let last_three = &rupees_str[rupees_str.len() - 3..];
            let remaining = &rupees_str[..rupees_str.len() - 3];
            let mut groups = Vec::new();
            let mut current = remaining;

            while !current.is_empty() {
                if current.len() <= 2 {
                    groups.push(current);
                    break;
                }
                let split_idx = current.len() - 2;
                groups.push(&current[split_idx..]);
                current = &current[..split_idx];
            }

            groups.reverse();
            format!("{},{last_three}", groups.join(","))
        };

        if self.0 < 0 {
            format!("-₹{formatted_rupees}.{paisa:02}")
        } else {
            format!("₹{formatted_rupees}.{paisa:02}")
        }
    }
}

impl fmt::Display for Money {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.format_inr())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_money_basic_creation_and_split() {
        let m = Money::from_rupees_and_paisa(100, 50).expect("valid");
        assert_eq!(m.as_paisa(), 10050);
        assert_eq!(m.to_rupees_and_paisa(), (100, 50));
        assert_eq!(m.format_inr(), "₹100.50");

        let neg = Money::from_rupees_and_paisa(-50, 75).expect("valid");
        assert_eq!(neg.as_paisa(), -5075);
        assert_eq!(neg.to_rupees_and_paisa(), (-50, 75));
        assert_eq!(neg.format_inr(), "-₹50.75");
    }

    #[test]
    fn test_indian_numbering_formatting() {
        let lakh = Money::from_paisa(10_000_000); // 1,00,000.00
        assert_eq!(lakh.format_inr(), "₹1,00,000.00");

        let crore = Money::from_paisa(12_345_678_950); // ₹12,34,56,789.50
        assert_eq!(crore.format_inr(), "₹12,34,56,789.50");

        let zero = Money::ZERO;
        assert_eq!(zero.format_inr(), "₹0.00");
    }

    #[test]
    fn test_checked_arithmetic_overflow() {
        let max = Money::from_paisa(i64::MAX);
        assert!(max.checked_add(Money::from_paisa(1)).is_err());

        let min = Money::from_paisa(i64::MIN);
        assert!(min.checked_sub(Money::from_paisa(1)).is_err());
        assert!(min.abs().is_err());
        assert!(Money::from_paisa(100).checked_div(0).is_err());
    }
}
