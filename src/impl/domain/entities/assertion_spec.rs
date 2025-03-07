use chrono::NaiveDate;

use super::handlers::Handlers;

pub struct AssertionSpec<H: Handlers> {
    pub date: NaiveDate,
    pub account: H::C,
    pub balance: f64,
    pub commodity: H::M,
}
