use chrono::NaiveDate;

use super::account::Account;

pub struct Assertion {
    pub date: NaiveDate,
    pub account: Account,
    pub balance: f64,
}
