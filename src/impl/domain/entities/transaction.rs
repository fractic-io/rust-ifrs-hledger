use chrono::NaiveDate;
use iso_currency::Currency;

use super::{account::Account, transaction_spec::TransactionSpecId};

#[derive(Debug, Clone)]
pub struct TransactionLabel {
    pub payee: String,
    pub description: String,
}

#[derive(Debug, Clone)]
pub struct TransactionPosting {
    pub account: Account,
    /// In case of a linked posting, the source posting it's based on (to enable
    /// "tracing through" the posting for proper cashflow tagging).
    pub source_account: Option<Account>,
    pub amount: f64,
    pub currency: Currency,
}

#[derive(Debug, Clone)]
pub struct Transaction {
    pub spec_id: TransactionSpecId,
    pub date: NaiveDate,
    pub postings: Vec<TransactionPosting>,
    pub comment: Option<String>,
}

// --

impl TransactionPosting {
    pub fn new(account: Account, amount: f64, currency: Currency) -> Self {
        Self {
            account,
            source_account: None,
            amount,
            currency,
        }
    }

    pub fn linked(
        account: Account,
        source_account: Account,
        amount: f64,
        currency: Currency,
    ) -> Self {
        Self {
            account,
            source_account: Some(source_account),
            amount,
            currency,
        }
    }
}
