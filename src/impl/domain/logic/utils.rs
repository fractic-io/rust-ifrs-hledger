use std::collections::VecDeque;

use chrono::{Datelike, Duration, NaiveDate};
use fractic_server_error::{CriticalError, ServerError};
use iso_currency::Currency;

use crate::{
    entities::{BackingAccount, CashHandler, ReimbursableEntityHandler, Transaction},
    errors::ReimbursementTracingError,
};

use super::spec_processor::{
    ExpenseHistoryPriceRecord, ReimbursementStateDelta, UnreimbursedEntry,
};

/// Returns the last day of each month between the given dates.
pub(crate) fn month_end_dates(
    start: NaiveDate,
    end: NaiveDate,
) -> Result<Vec<NaiveDate>, ServerError> {
    let mut dates = Vec::new();
    let mut current = start;
    while current <= end {
        // Compute first day of next month.
        let (year, month) = if current.month() == 12 {
            (current.year() + 1, 1)
        } else {
            (current.year(), current.month() + 1)
        };
        let next_month = NaiveDate::from_ymd_opt(year, month, 1).ok_or_else(|| {
            CriticalError::with_debug(
                "last-date-of-month calculation unexpectedly resulted in invalid date",
                &format!("year: {}, month: {}", year, month),
            )
        })?;
        let last_day = next_month - Duration::days(1);
        if last_day >= start && last_day <= end {
            dates.push(last_day);
        }
        current = next_month;
    }
    Ok(dates)
}

/// Returns the first day of the month of the given date.
pub(crate) fn month_start_date(date: NaiveDate) -> NaiveDate {
    NaiveDate::from_ymd_opt(date.year(), date.month(), 1)
        .expect("copying a NaiveDate with overridden day=1 should never fail")
}

pub(crate) struct MonthlyAccrualPeriod {
    /// Period over which the accrual is computed in the given month (usually
    /// the month start and end dates themselves, but can be shorter).
    pub(crate) period_start: NaiveDate,
    pub(crate) period_end: NaiveDate,
    pub(crate) num_days: i64,
    /// Date on which the accrual adjustment should be recorded (usually the
    /// last day of the month).
    pub(crate) adjustment_date: NaiveDate,
}
/// Returns the monthly accrual periods in the given range. Each period
/// corresponds to a calendar month (and returns the period for which accrual
/// occurred in that given month).
pub(crate) fn monthly_accrual_periods(
    start: NaiveDate,
    end: NaiveDate,
) -> Result<Vec<MonthlyAccrualPeriod>, ServerError> {
    month_end_dates(start, end)?
        .into_iter()
        .map(|month_end| {
            let month_start = month_start_date(month_end);
            let period_start = std::cmp::max(start, month_start);
            let period_end = std::cmp::min(end, month_end);
            let num_days = (period_end - period_start).num_days() + 1;
            Ok(MonthlyAccrualPeriod {
                period_start,
                period_end,
                num_days,
                // For consistency, record all accrual adjustments on the last
                // day of each month.
                adjustment_date: month_end,
            })
        })
        .collect()
}

/// Similar to `monthly_accrual_periods`, but returns the end-of-period adjust
/// amounts, and date on which the adjustment should be recorded.
///
/// Adjustment amounts are rounded to the currency's decimal places, and any
/// remaining rounding error is corrected on the last period.
pub(crate) struct MonthlyAccrualAdjustment {
    pub(crate) period_start: NaiveDate,
    pub(crate) period_end: NaiveDate,
    pub(crate) adjustment_amount: f64,
    pub(crate) adjustment_date: NaiveDate,
}
pub(crate) fn monthly_accrual_adjustments(
    start: NaiveDate,
    end: NaiveDate,
    total: f64,
    currency: Currency,
) -> Result<Vec<MonthlyAccrualAdjustment>, ServerError> {
    let accrual_days = (end - start).num_days() + 1;
    let daily_rate = total / (accrual_days as f64);

    // Determine the number of decimal places from the currency.
    let decimal_places = currency.exponent().unwrap_or(0) as i32;
    let factor = 10_f64.powi(decimal_places);
    let periods = monthly_accrual_periods(start, end)?;
    if periods.is_empty() {
        return Ok(vec![]);
    }

    // Compute the rounded accrual amounts for each period.
    let mut adjustments = Vec::with_capacity(periods.len());
    let mut running_total = 0f64;
    for (i, period) in periods.iter().enumerate() {
        let unrounded = daily_rate * (period.num_days as f64);
        if i < periods.len() - 1 {
            let rounded = (unrounded * factor).round() / factor;
            adjustments.push(MonthlyAccrualAdjustment {
                period_start: period.period_start,
                period_end: period.period_end,
                adjustment_amount: rounded,
                adjustment_date: period.adjustment_date,
            });
            running_total += rounded;
        } else {
            // Compute the final adjustment directly so that the sum is exact.
            let final_adjustment = total - running_total;
            adjustments.push(MonthlyAccrualAdjustment {
                period_start: period.period_start,
                period_end: period.period_end,
                adjustment_amount: final_adjustment,
                adjustment_date: period.adjustment_date,
            });
        }
    }
    Ok(adjustments)
}

/// Given a slice of variable expense records and a window defined by
/// [window_start, window_end], this computes the “effective” daily accrual rate
/// by summing the contributions of all overlapping records. Days not covered by
/// any record count as zero.
pub(crate) fn compute_daily_average(
    records: &[ExpenseHistoryPriceRecord],
    window_start: NaiveDate,
    window_end: NaiveDate,
) -> Option<f64> {
    let total_window_days = (window_end - window_start).num_days() + 1;
    if total_window_days <= 0 {
        return None;
    }
    let mut total_amount = 0.0;

    // For each record, add (daily_rate * number_of_overlapping_days).
    for record in records {
        let overlap_start = std::cmp::max(record.start, window_start);
        let overlap_end = std::cmp::min(record.end, window_end);
        if overlap_start <= overlap_end {
            let overlap_days = (overlap_end - overlap_start).num_days() + 1;
            total_amount += record.daily_rate * (overlap_days as f64);
        }
    }

    // If no days contributed (i.e. no overlapping records) then we have no history.
    if total_amount == 0.0 {
        None
    } else {
        Some(total_amount / (total_window_days as f64))
    }
}

pub(crate) fn track_unreimbursed_entries<R: ReimbursableEntityHandler, C: CashHandler>(
    backing_account: &BackingAccount<R, C>,
    transactions: &Vec<Transaction>,
    ext_transactions: &Vec<Transaction>,
) -> Result<Option<ReimbursementStateDelta>, ServerError> {
    if let BackingAccount::Reimburse(r_handler) = backing_account {
        let mut reimbursable_entries = Vec::new();
        for tx in transactions.iter().chain(ext_transactions.iter()) {
            let reimbursable_debits = tx
                .postings
                .iter()
                .filter_map(|p| {
                    if p.amount < 0.0 && p.account == r_handler.account().into() {
                        Some(p.amount.abs())
                    } else {
                        None
                    }
                })
                .sum::<f64>();
            if reimbursable_debits > 0.0 {
                let credit_postings = tx
                    .postings
                    .iter()
                    .filter(|p| p.amount > 0.0)
                    .cloned()
                    .collect::<Vec<_>>();
                if credit_postings.iter().map(|p| p.amount).sum::<f64>() != reimbursable_debits {
                    return Err(ReimbursementTracingError::with_debug(
                        "some transactions included a combination of reimbursable and non-reimbursable debits, which is not yet supported",
                        &backing_account,
                    ));
                }
                reimbursable_entries.push(UnreimbursedEntry {
                    total_amount: reimbursable_debits,
                    credit_postings,
                });
            }
        }
        if reimbursable_entries.is_empty() {
            Ok(None)
        } else {
            Ok(Some(ReimbursementStateDelta::Push {
                account: r_handler.account(),
                entries: reimbursable_entries,
            }))
        }
    } else {
        Ok(None)
    }
}

pub(crate) trait PopByAmount {
    type Entry;

    /// Peak the first entries in the queue accumulating to exactly 'amount',
    /// and the sum of the remaining unpeaked amount. Returns an error if we run
    /// out of entries to peak before reaching the amount, or if the amount
    /// cannot be satisfied in an whole number of entries.
    fn peak_until_exactly(&self, amount: f64) -> Result<(Vec<&Self::Entry>, f64), ServerError>;

    /// Pop from the front of the queue until an accumulated amount of exactly
    /// 'amount'. Returns an error if we run out of entries to pop before
    /// reaching the amount, or if the amount cannot be satisfied in an whole
    /// number of entries.
    fn pop_until_exactly(&mut self, amount: f64) -> Result<(), ServerError>;
}

impl PopByAmount for VecDeque<UnreimbursedEntry> {
    type Entry = UnreimbursedEntry;

    fn peak_until_exactly(&self, amount: f64) -> Result<(Vec<&Self::Entry>, f64), ServerError> {
        const EPSILON: f64 = 1e-9;
        let mut remaining = amount;
        let mut entries = Vec::new();

        for entry in self.iter() {
            // If we've essentially reached zero, we can stop.
            if remaining.abs() < EPSILON {
                break;
            }

            // If the next entry is larger than the remaining (considering
            // tolerance), we cannot satisfy the exact amount.
            if entry.total_amount > remaining + EPSILON {
                return Err(ReimbursementTracingError::with_debug(
                    &format!(
                        "amount cannot be satisfied with a whole number of entries; remaining: {:.10}, next entry: {:.10}",
                        remaining, entry.total_amount
                    ),
                    &self,
                ));
            }

            entries.push(entry);
            remaining -= entry.total_amount;
        }

        // If after iterating the remaining amount is still significant, we ran
        // out of entries.
        if remaining.abs() > EPSILON {
            return Err(ReimbursementTracingError::with_debug(
                &format!(
                    "ran out of entries before reaching the expected amount; remaining: {:.10}",
                    remaining
                ),
                &self,
            ));
        }

        Ok((entries, remaining))
    }

    fn pop_until_exactly(&mut self, amount: f64) -> Result<(), ServerError> {
        let (entries, _) = self.peak_until_exactly(amount)?;
        self.drain(..entries.len());
        Ok(())
    }
}
