use std::{fs, str::FromStr as _};

use fractic_server_error::ServerError;
use ron::from_str;

use crate::{
    data::models::{
        accounting_amount_model::AccountingAmountModel,
        accounting_logic_model::AccountingLogicModel, backing_account_model::BackingAccountModel,
        iso_date_model::ISODateModel,
    },
    entities::{Handlers, TransactionSpec, TransactionSpecId},
    errors::{InvalidCsv, InvalidRon, ReadError},
};

pub(crate) trait TransactionsCsvDatasource<H: Handlers> {
    fn from_string(&self, s: &str) -> Result<Vec<TransactionSpec<H>>, ServerError>;

    fn from_file<P>(&self, path: P) -> Result<Vec<TransactionSpec<H>>, ServerError>
    where
        P: AsRef<std::path::Path>;
}

pub(crate) struct TransactionsCsvDatasourceImpl<H: Handlers> {
    _phantom: std::marker::PhantomData<H>,
}

impl<H: Handlers> TransactionsCsvDatasourceImpl<H> {
    pub(crate) fn new() -> Self {
        Self {
            _phantom: std::marker::PhantomData,
        }
    }
}

impl<H: Handlers> TransactionsCsvDatasource<H> for TransactionsCsvDatasourceImpl<H> {
    fn from_string(&self, s: &str) -> Result<Vec<TransactionSpec<H>>, ServerError> {
        csv::Reader::from_reader(s.as_bytes())
            .records()
            .enumerate()
            .map(|(i, r)| {
                r.map_err(|e| InvalidCsv::with_debug(&e)).and_then(|r| {
                    // Extract from CSV record.
                    let raw_accrual_date = r.get(0).unwrap_or("");
                    let raw_until = match r.get(1) {
                        Some(s) if !s.is_empty() => Some(s),
                        _ => None,
                    };
                    let raw_payment_date = r.get(2).unwrap_or("");
                    let raw_accounting_logic = r.get(3).unwrap_or("");
                    let raw_decorators = r.get(4).unwrap_or("");
                    let raw_entity = r.get(5).unwrap_or("");
                    let raw_description = r.get(6).unwrap_or("");
                    let raw_amount = r.get(7).unwrap_or("0");
                    let raw_commodity = r.get(8).unwrap_or("");
                    let raw_backing_account = r.get(9).unwrap_or("");

                    // Parse.
                    let accrual_date: ISODateModel = ISODateModel::from_str(raw_accrual_date)?;
                    let until: Option<ISODateModel> =
                        raw_until.map(ISODateModel::from_str).transpose()?;
                    let payment_date: ISODateModel = ISODateModel::from_str(raw_payment_date)?;
                    let accounting_logic: AccountingLogicModel<H::E, H::A, H::I, H::R> =
                        from_str(raw_accounting_logic)
                            .map_err(|e| InvalidRon::with_debug("AccountingLogic", &e))?;
                    let decorators: Vec<H::D> = from_str(&format!("[{}]", raw_decorators))
                        .map_err(|e| InvalidRon::with_debug("Decorator", &e))?;
                    let payee: H::P =
                        from_str(raw_entity).map_err(|e| InvalidRon::with_debug("Payee", &e))?;
                    let description: String = raw_description.into();
                    let amount: AccountingAmountModel =
                        AccountingAmountModel::from_str(raw_amount)?;
                    let commodity: H::M = from_str(raw_commodity)
                        .map_err(|e| InvalidRon::with_debug("Commodity", &e))?;
                    let backing_account: BackingAccountModel<H::R, H::C> =
                        from_str(raw_backing_account)
                            .map_err(|e| InvalidRon::with_debug("BackingAccount", &e))?;

                    // Build.
                    Ok(TransactionSpec {
                        id: TransactionSpecId(i as u64),
                        accrual_date: accrual_date.into(),
                        until: until.map(Into::into),
                        payment_date: payment_date.into(),
                        accounting_logic: accounting_logic.into(),
                        decorators,
                        payee,
                        description,
                        amount: amount.into(),
                        commodity,
                        backing_account: backing_account.into(),
                    })
                })
            })
            .collect()
    }

    fn from_file<P>(&self, path: P) -> Result<Vec<TransactionSpec<H>>, ServerError>
    where
        P: AsRef<std::path::Path>,
    {
        self.from_string(&fs::read_to_string(path).map_err(|e| ReadError::with_debug(&e))?)
    }
}
