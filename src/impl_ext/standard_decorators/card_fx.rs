use std::{path::PathBuf, str::FromStr as _};

use async_trait::async_trait;
use chrono::NaiveDate;
use fractic_server_error::ServerError;

use crate::{
    data::models::iso_date_model::ISODateModel,
    entities::{DecoratedTransactionSpec, DecoratorLogic, Handlers},
};

#[derive(Debug)]
pub struct StandardDecoratorCardFx {
    to_currency: String,
    settle_date: NaiveDate,
    settle_amount: f64,
    currency_conversion_cache_dir: PathBuf,
    currency_conversion_api_key: String,
}

impl StandardDecoratorCardFx {
    pub fn new(
        to_currency: impl Into<String>,
        settle_date: &String,
        settle_amount: f64,
        currency_conversion_cache_dir: impl Into<PathBuf>,
        currency_conversion_api_key: impl Into<String>,
    ) -> Result<Self, ServerError> {
        Ok(Self {
            to_currency: to_currency.into(),
            settle_date: ISODateModel::from_str(settle_date)?.into(),
            settle_amount,
            currency_conversion_cache_dir: currency_conversion_cache_dir.into(),
            currency_conversion_api_key: currency_conversion_api_key.into(),
        })
    }
}

#[async_trait]
impl<H: Handlers> DecoratorLogic<H> for StandardDecoratorCardFx {
    async fn apply(
        &self,
        tx: DecoratedTransactionSpec<H>,
    ) -> Result<DecoratedTransactionSpec<H>, ServerError> {
        Ok(tx)
    }
}
