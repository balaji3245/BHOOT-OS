use crate::error::CoreError;
use crate::money::Money;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

/// Time horizon for a financial budget.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum BudgetPeriod {
    Monthly,
    Quarterly,
    Annual,
    Custom,
}

/// Alert state of category spending against allocated budget.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum BudgetStatus {
    /// Spending is within normal limits (< threshold).
    OnTrack,
    /// Spending has crossed warning threshold (e.g., >= 80%).
    Warning,
    /// Spending has equaled or exceeded 100% of allocation.
    Exceeded,
}

/// Budget limit allocated to an individual spending category.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CategoryBudget {
    pub category: String,
    pub allocated: Money,
    pub alert_threshold_pct: u8,
}

impl CategoryBudget {
    /// Create a new category allocation.
    ///
    /// # Errors
    /// Returns `CoreError::InvalidAmount` if allocated money is negative.
    pub fn new(
        category: impl Into<String>,
        allocated: Money,
        alert_threshold_pct: u8,
    ) -> Result<Self, CoreError> {
        if allocated.is_negative() {
            return Err(CoreError::InvalidAmount(
                "Budget category allocation cannot be negative".into(),
            ));
        }
        let threshold = if alert_threshold_pct == 0 || alert_threshold_pct > 100 {
            80
        } else {
            alert_threshold_pct
        };

        Ok(Self {
            category: category.into(),
            allocated,
            alert_threshold_pct: threshold,
        })
    }
}

/// Comprehensive category spending and budget utilization report.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CategoryBudgetReport {
    pub category: String,
    pub allocated: Money,
    pub spent: Money,
    pub remaining: Money,
    pub utilization_basis_points: u32,
    pub status: BudgetStatus,
}

/// Master budget container covering a defined period.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Budget {
    pub id: Uuid,
    pub name: String,
    pub period: BudgetPeriod,
    pub start_date: DateTime<Utc>,
    pub end_date: DateTime<Utc>,
    pub categories: Vec<CategoryBudget>,
    pub notes: Option<String>,
}

/// Performance analysis of a budget evaluated against realized spending.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BudgetReport {
    pub budget_id: Uuid,
    pub budget_name: String,
    pub period: BudgetPeriod,
    pub total_allocated: Money,
    pub total_spent: Money,
    pub total_remaining: Money,
    pub overall_utilization_basis_points: u32,
    pub category_reports: Vec<CategoryBudgetReport>,
}

impl Budget {
    /// Create a new budget plan.
    ///
    /// # Errors
    /// Returns `CoreError::InvalidAmount` if end date is before start date.
    pub fn new(
        name: impl Into<String>,
        period: BudgetPeriod,
        start_date: DateTime<Utc>,
        end_date: DateTime<Utc>,
        categories: Vec<CategoryBudget>,
        notes: Option<String>,
    ) -> Result<Self, CoreError> {
        if end_date <= start_date {
            return Err(CoreError::InvalidAmount(
                "Budget end date must be strictly after start date".into(),
            ));
        }

        Ok(Self {
            id: Uuid::new_v4(),
            name: name.into(),
            period,
            start_date,
            end_date,
            categories,
            notes,
        })
    }

    /// Calculate aggregate allocated budget across all categories.
    ///
    /// # Errors
    /// Returns `CoreError::ArithmeticOverflow` on integer overflow.
    pub fn total_allocated(&self) -> Result<Money, CoreError> {
        let mut total = Money::ZERO;
        for cat in &self.categories {
            total = total.checked_add(cat.allocated)?;
        }
        Ok(total)
    }

    /// Evaluate budget execution against realized spending per category.
    ///
    /// # Errors
    /// Returns `CoreError::ArithmeticOverflow` on calculation overflow.
    pub fn evaluate_spending(
        &self,
        spending_by_category: &HashMap<String, Money>,
    ) -> Result<BudgetReport, CoreError> {
        let mut category_reports = Vec::with_capacity(self.categories.len());
        let mut total_allocated = Money::ZERO;
        let mut total_spent = Money::ZERO;

        for cat in &self.categories {
            let spent = spending_by_category
                .get(&cat.category)
                .copied()
                .unwrap_or(Money::ZERO);

            total_allocated = total_allocated.checked_add(cat.allocated)?;
            total_spent = total_spent.checked_add(spent)?;

            let remaining = cat.allocated.checked_sub(spent)?;

            let alloc_paisa = cat.allocated.as_paisa();
            let spent_paisa = spent.as_paisa();

            let utilization_bps = if alloc_paisa > 0 {
                let scaled = (spent_paisa as i128 * 10000) / alloc_paisa as i128;
                scaled.clamp(0, u32::MAX as i128) as u32
            } else if spent_paisa > 0 {
                10000
            } else {
                0
            };

            let threshold_bps = (cat.alert_threshold_pct as u32) * 100;
            let status = if utilization_bps >= 10000 {
                BudgetStatus::Exceeded
            } else if utilization_bps >= threshold_bps {
                BudgetStatus::Warning
            } else {
                BudgetStatus::OnTrack
            };

            category_reports.push(CategoryBudgetReport {
                category: cat.category.clone(),
                allocated: cat.allocated,
                spent,
                remaining,
                utilization_basis_points: utilization_bps,
                status,
            });
        }

        let total_remaining = total_allocated.checked_sub(total_spent)?;

        let total_alloc_paisa = total_allocated.as_paisa();
        let total_spent_paisa = total_spent.as_paisa();

        let overall_utilization_bps = if total_alloc_paisa > 0 {
            let scaled = (total_spent_paisa as i128 * 10000) / total_alloc_paisa as i128;
            scaled.clamp(0, u32::MAX as i128) as u32
        } else if total_spent_paisa > 0 {
            10000
        } else {
            0
        };

        Ok(BudgetReport {
            budget_id: self.id,
            budget_name: self.name.clone(),
            period: self.period,
            total_allocated,
            total_spent,
            total_remaining,
            overall_utilization_basis_points: overall_utilization_bps,
            category_reports,
        })
    }
}
