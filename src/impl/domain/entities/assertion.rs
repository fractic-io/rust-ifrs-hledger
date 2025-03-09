use chrono::NaiveDate;

use super::account::Account;

#[derive(Debug, Clone)]
pub struct Assertion {
    pub date: NaiveDate,
    pub account: Account,
    pub balance: f64,
    pub currency: String,
}
