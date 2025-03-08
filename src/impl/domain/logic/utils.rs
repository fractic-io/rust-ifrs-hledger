use chrono::{Datelike, Duration, NaiveDate};
use fractic_server_error::{CriticalError, ServerError};

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

pub(crate) fn month_start(date: NaiveDate) -> NaiveDate {
    NaiveDate::from_ymd_opt(date.year(), date.month(), 1)
        .expect("copying a NaiveDate with overridden day=1 should never fail")
}
