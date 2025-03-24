use std::collections::HashMap;

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
    pub custom_tags: HashMap<String, String>,
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
            custom_tags: HashMap::new(),
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
            custom_tags: HashMap::new(),
        }
    }

    pub fn non_cash_reclassification(account: Account, amount: f64, currency: Currency) -> Self {
        Self {
            account,
            source_account: None,
            amount,
            currency,
            custom_tags: vec![("s".to_string(), "non_cash_reclassification".to_string())]
                .into_iter()
                .collect(),
        }
    }
}
