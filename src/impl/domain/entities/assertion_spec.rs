use chrono::NaiveDate;

use super::handlers::{CashHandler, CommodityHandler};

pub struct AssertionSpec<C, M>
where
    C: CashHandler,
    M: CommodityHandler,
{
    pub date: NaiveDate,
    pub account: C,
    pub balance: f64,
    pub commodity: M,
}
