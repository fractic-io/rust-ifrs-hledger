use std::str::FromStr as _;

use async_trait::async_trait;
use fractic_server_error::ServerError;
use ron::from_str;

use crate::{
    data::models::{
        accounting_amount_model::AccountingAmountModel,
        accounting_logic_model::AccountingLogicModel, backing_account_model::BackingAccountModel,
        iso_date_model::ISODateModel,
    },
    entities::{Annotation, Handlers, TransactionSpec, TransactionSpecId},
    errors::{InvalidCsv, InvalidRon, ReadError},
};

#[async_trait]
pub(crate) trait TransactionsCsvDatasource<H: Handlers>: Send + Sync {
    fn from_string(&self, s: &str) -> Result<Vec<TransactionSpec<H>>, ServerError>;

    async fn from_file<P>(&self, path: P) -> Result<Vec<TransactionSpec<H>>, ServerError>
    where
        P: AsRef<std::path::Path> + Send;
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

#[async_trait]
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
                    let raw_notes = r.get(10).unwrap_or("");

                    // Parse.
                    let accrual_start: ISODateModel = ISODateModel::from_str(raw_accrual_date)?;
                    let accrual_end: Option<ISODateModel> =
                        raw_until.map(ISODateModel::from_str).transpose()?;
                    let payment_date: ISODateModel = ISODateModel::from_str(raw_payment_date)?;
                    let accounting_logic: AccountingLogicModel<H::E, H::A, H::I, H::R, H::S> =
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
                    let custom_notes: Vec<Annotation> = if raw_notes.is_empty() {
                        vec![]
                    } else {
                        raw_notes
                            .split(',')
                            .map(|n| Annotation::Custom(n.into()))
                            .collect()
                    };

                    // Build.
                    Ok(TransactionSpec {
                        id: TransactionSpecId((i + 2) as u64),
                        accrual_start: accrual_start.into(),
                        accrual_end: accrual_end.map(Into::into),
                        payment_date: payment_date.into(),
                        accounting_logic: accounting_logic.into(),
                        decorators,
                        payee,
                        description,
                        amount: amount.into(),
                        commodity,
                        backing_account: backing_account.into(),
                        annotations: custom_notes,
                    })
                })
            })
            .collect()
    }

    async fn from_file<P>(&self, path: P) -> Result<Vec<TransactionSpec<H>>, ServerError>
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
