use std::str::FromStr as _;

use async_trait::async_trait;
use fractic_server_error::ServerError;
use ron::from_str;

use crate::{
    data::models::{accounting_amount_model::AccountingAmountModel, iso_date_model::ISODateModel},
    domain::entities::assertion_spec::AssertionSpec,
    entities::Handlers,
    errors::{InvalidCsv, InvalidRon, ReadError},
};

#[async_trait]
pub(crate) trait BalancesCsvDatasource<H: Handlers>: Send + Sync {
    fn from_string(&self, s: &str) -> Result<Vec<AssertionSpec<H>>, ServerError>;

    async fn from_file<P>(&self, path: P) -> Result<Vec<AssertionSpec<H>>, ServerError>
    where
        P: AsRef<std::path::Path> + Send;
}

pub(crate) struct BalancesCsvDatasourceImpl<H: Handlers> {
    _phantom: std::marker::PhantomData<H>,
}

impl<H: Handlers> BalancesCsvDatasourceImpl<H> {
    pub(crate) fn new() -> Self {
        Self {
            _phantom: std::marker::PhantomData,
        }
    }
}

#[async_trait]
impl<H: Handlers> BalancesCsvDatasource<H> for BalancesCsvDatasourceImpl<H> {
    fn from_string(&self, s: &str) -> Result<Vec<AssertionSpec<H>>, ServerError> {
        csv::Reader::from_reader(s.as_bytes())
            .records()
            .map(|r| {
                r.map_err(|e| InvalidCsv::with_debug(&e)).and_then(|r| {
                    // Extract from CSV record.
                    let raw_account = r.get(0).unwrap_or("");
                    let raw_date = r.get(1).unwrap_or("");
                    let raw_balance = r.get(2).unwrap_or("0");
                    let raw_commodity = r.get(3).unwrap_or("");

                    // Parse.
                    let date: ISODateModel = ISODateModel::from_str(raw_date)?;
                    let cash_handler: H::C =
                        from_str(raw_account).map_err(|e| InvalidRon::with_debug("Cash", &e))?;
                    let balance: AccountingAmountModel =
                        AccountingAmountModel::from_str(raw_balance)?;
                    let commodity: H::M = from_str(raw_commodity)
                        .map_err(|e| InvalidRon::with_debug("Commodity", &e))?;

                    // Build.
                    Ok(AssertionSpec {
                        date: date.into(),
                        cash_handler,
                        balance: balance.into(),
                        commodity,
                    })
                })
            })
            .collect()
    }

    async fn from_file<P>(&self, path: P) -> Result<Vec<AssertionSpec<H>>, ServerError>
    where
        P: AsRef<std::path::Path> + Send,
    {
        self.from_string(
            &tokio::fs::read_to_string(path)
                .await
                .map_err(|e| ReadError::with_debug(&e))?,
        )
    }
}
