use chrono::NaiveDate;
use iso_currency::Currency;

use super::account::Account;

#[derive(Debug, Clone)]
pub struct Assertion {
    pub date: NaiveDate,
    pub account: Account,
    pub balance: f64,
    pub currency: Currency,
}
