use chrono::{Datelike, NaiveDate};
use iso_currency::Currency;

use crate::entities::CloseLogic;

#[derive(Debug, Clone)]
pub enum EndOfYearEntry {
    Close {
        /// Date on which to apply the closing entry.
        date: NaiveDate,
        /// Accounts to be cleared, and their amounts.
        postings: Vec<(String, f64)>,
        /// Optional additional validation of the total amount. If not provided,
        /// it will be omitted in the final ledger, in which case hledger
        /// auto-calculates this value on evaluation.
        total: Option<f64>,
        /// The filing logic to be applied.
        logic: CloseLogic,
        /// Currency used to format entry & total amounts.
        currency: Currency,
    },
    Correction {
        /// Date on which to apply the correction entry.
        date: NaiveDate,
        /// Description from original command properties (if any).
        description: Option<String>,
        /// Notes from the original command properties (if any).
        notes: Vec<String>,
        /// Ledger content to be inserted verbatim (although formatting may be
        /// altered).
        macro_output: String,
    },
}

// --

impl EndOfYearEntry {
    pub fn year(&self) -> i32 {
        match self {
            EndOfYearEntry::Close { date, .. } => date.year(),
            EndOfYearEntry::Correction { date, .. } => date.year(),
        }
    }

    pub fn date(&self) -> &NaiveDate {
        match self {
            EndOfYearEntry::Close { date, .. } => date,
            EndOfYearEntry::Correction { date, .. } => date,
        }
    }
}
