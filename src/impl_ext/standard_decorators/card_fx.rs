use std::str::FromStr as _;

use chrono::NaiveDate;
use fractic_server_error::ServerError;

use crate::{
    data::models::iso_date_model::ISODateModel,
    entities::{DecoratedTransactionSpec, DecoratorLogic, Handlers},
};

pub struct StandardDecoratorCardFx {
    to_currency: String,
    settle_date: NaiveDate,
    settle_amount: f64,
}

impl StandardDecoratorCardFx {
    pub fn new(
        to_currency: String,
        settle_date: &String,
        settle_amount: f64,
    ) -> Result<Self, ServerError> {
        Ok(Self {
            to_currency,
            settle_date: ISODateModel::from_str(settle_date)?.into(),
            settle_amount,
        })
    }
}

impl<'a, H: Handlers> DecoratorLogic<'a, H> for StandardDecoratorCardFx {
    fn apply(
        &self,
        tx: DecoratedTransactionSpec<H>,
    ) -> Result<DecoratedTransactionSpec<H>, ServerError> {
        todo!()
    }
}
