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
