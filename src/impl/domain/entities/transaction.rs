use chrono::NaiveDate;

use crate::entities::TransactionSpecId;

use super::account::Account;

#[derive(Debug)]
pub struct Posting {
    pub account: Account,
    pub amount: f64,
    pub currency: String,
}

#[derive(Debug)]
pub struct Transaction {
    pub spec_id: TransactionSpecId,
    pub date: NaiveDate,
    pub entity: String,
    pub description: String,
    pub postings: Vec<Posting>,
    pub notes: Vec<String>,
}
