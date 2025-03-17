use chrono::{Datelike, Duration, NaiveDate};
use fractic_server_error::{CriticalError, ServerError};

use super::spec_processor::VariableExpenseRecord;

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

/// Given a slice of variable expense records and a window defined by
/// [window_start, window_end], this computes the “effective” daily accrual
/// rate by summing the contributions of all overlapping records. Days not
/// covered by any record count as zero.
pub(crate) fn compute_daily_average(
    records: &[VariableExpenseRecord],
    window_start: NaiveDate,
    window_end: NaiveDate,
) -> Option<f64> {
    let total_window_days = (window_end - window_start).num_days() + 1;
    if total_window_days <= 0 {
        return None;
    }
    let mut total_amount = 0.0;
    // For each record, add (daily_rate * number_of_overlapping_days)
    for record in records {
        let overlap_start = std::cmp::max(record.start, window_start);
        let overlap_end = std::cmp::min(record.end, window_end);
        if overlap_start <= overlap_end {
            let days = (overlap_end - overlap_start).num_days() + 1;
            total_amount += record.daily_rate * (days as f64);
        }
    }
    // If no days contributed (i.e. no overlapping records) then we have no history.
    if total_amount == 0.0 {
        None
    } else {
        Some(total_amount / (total_window_days as f64))
    }
}
