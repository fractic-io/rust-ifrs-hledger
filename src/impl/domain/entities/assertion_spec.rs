use chrono::NaiveDate;

use super::{
    account::Account,
    handlers::{AssetHandler, CashHandler, Handlers, ReimbursableEntityHandler},
};

#[derive(Debug)]
#[allow(dead_code)]
pub(crate) enum AssertableHandler<H: Handlers> {
    Asset(H::A),
    ReimbursableEntity(H::R),
    Cash(H::C),
    Custom(Account),
}

#[derive(Debug)]
pub(crate) struct AssertionSpec<H: Handlers> {
    pub date: NaiveDate,
    pub handler: AssertableHandler<H>,
    pub balance: f64,
    pub commodity: H::M,
}

// --

impl<H: Handlers> AssertableHandler<H> {
    pub(crate) fn account(&self) -> Account {
        match self {
            AssertableHandler::Asset(a) => a.account().into(),
            AssertableHandler::ReimbursableEntity(r) => r.account().into(),
            AssertableHandler::Cash(c) => c.account().into(),
            AssertableHandler::Custom(a) => a.clone(),
        }
    }
}
