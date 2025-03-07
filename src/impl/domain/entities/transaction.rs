use chrono::NaiveDate;

use super::account::Account;

pub struct Posting {
    pub account: Account,
    pub amount: f64,
    pub commodity: String,
}

pub struct Transaction {
    pub date: NaiveDate,
    pub description: String,
    pub postings: Vec<Posting>,
    pub notes: Vec<String>,
}
