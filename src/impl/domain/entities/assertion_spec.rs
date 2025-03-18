use chrono::NaiveDate;

use super::handlers::Handlers;

#[derive(Debug)]
pub struct AssertionSpec<H: Handlers> {
    pub date: NaiveDate,
    pub cash_handler: H::C,
    pub balance: f64,
    pub commodity: H::M,
}
