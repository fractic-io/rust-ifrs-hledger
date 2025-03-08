use std::{fs, str::FromStr as _};

use fractic_server_error::ServerError;
use ron::from_str;

use crate::{
    data::models::{accounting_amount_model::AccountingAmountModel, iso_date_model::ISODateModel},
    domain::entities::assertion_spec::AssertionSpec,
    entities::Handlers,
    errors::{InvalidCsv, InvalidRon, ReadError},
};

pub(crate) trait BalancesCsvDatasource<H: Handlers> {
    fn from_string(&self, s: &str) -> Result<Vec<AssertionSpec<H>>, ServerError>;

    fn from_file<P>(&self, path: P) -> Result<Vec<AssertionSpec<H>>, ServerError>
    where
        P: AsRef<std::path::Path>;
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
                    let account: H::C =
                        from_str(raw_account).map_err(|e| InvalidRon::with_debug("Cash", &e))?;
                    let balance: AccountingAmountModel =
                        AccountingAmountModel::from_str(raw_balance)?;
                    let commodity: H::M = from_str(raw_commodity)
                        .map_err(|e| InvalidRon::with_debug("Commodity", &e))?;

                    // Build.
                    Ok(AssertionSpec {
                        date: date.into(),
                        cash_handler: account,
                        balance: balance.into(),
                        commodity,
                    })
                })
            })
            .collect()
    }

    fn from_file<P>(&self, path: P) -> Result<Vec<AssertionSpec<H>>, ServerError>
    where
        P: AsRef<std::path::Path>,
    {
        self.from_string(&fs::read_to_string(path).map_err(|e| ReadError::with_debug(&e))?)
    }
}
