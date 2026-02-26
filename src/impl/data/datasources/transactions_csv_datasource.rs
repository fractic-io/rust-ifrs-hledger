use std::str::FromStr as _;

use async_trait::async_trait;
use fractic_server_error::ServerError;
use ron::from_str;

use crate::{
    data::models::{
        accounting_amount_model::AccountingAmountModel,
        accounting_logic_model::AccountingLogicModel, backing_account_model::BackingAccountModel,
        command_logic_model::CommandLogicModel, iso_date_model::ISODateModel,
    },
    entities::{Annotation, Command, CommandSpecId, Handlers, TransactionSpec, TransactionSpecId},
    errors::{InvalidCsv, InvalidCsvContent, InvalidRon, ReadError},
};

#[async_trait]
pub(crate) trait TransactionsCsvDatasource<H: Handlers>: Send + Sync {
    fn from_string(
        &self,
        s: &str,
    ) -> Result<(Vec<TransactionSpec<H>>, Vec<Command<H>>), ServerError>;

    async fn from_file<P>(
        &self,
        path: P,
    ) -> Result<(Vec<TransactionSpec<H>>, Vec<Command<H>>), ServerError>
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
    fn from_string(
        &self,
        s: &str,
    ) -> Result<(Vec<TransactionSpec<H>>, Vec<Command<H>>), ServerError> {
        csv::Reader::from_reader(s.as_bytes())
            .records()
            .enumerate()
            .try_fold(
                (Vec::new(), Vec::new()),
                |(mut transaction_specs, mut commands), (i, r)| {
                    let r = r.map_err(|e| InvalidCsv::with_debug(&e))?;
                    let first_char = r
                        .get(0)
                        .and_then(|c| c.trim().chars().next())
                        .unwrap_or(' ');

                    // Skip empty/whitespace-only lines.
                    if r.iter().all(|cell| cell.trim().is_empty()) {
                        return Ok((transaction_specs, commands));
                    }
                    // Skip comment lines.
                    if first_char == ';' {
                        return Ok((transaction_specs, commands));
                    }

                    if first_char == ':' {
                        // Parse command entry.
                        // --

                        // Extract from CSV record.
                        let raw_date = r.get(2).unwrap_or("").trim();
                        let raw_exec = r.get(3).unwrap_or("").trim();
                        let raw_arguments = r.get(4).unwrap_or("").trim();
                        let raw_description = r.get(6).unwrap_or("").trim();
                        let raw_amount = r.get(7).unwrap_or("").trim();
                        let raw_commodity = r.get(8).unwrap_or("").trim();
                        let raw_notes = r.get(10).unwrap_or("");

                        // Parse.
                        let date: ISODateModel = ISODateModel::from_str(raw_date)?;
                        let exec: CommandLogicModel<H::F> = from_str(raw_exec)
                            .map_err(|e| InvalidRon::with_debug("CommandLogic", &e))?;
                        let arguments: Vec<String> =
                            raw_arguments.split(',').map(|s| s.to_string()).collect();
                        let description: Option<String> = if raw_description.is_empty() {
                            None
                        } else {
                            Some(raw_description.into())
                        };
                        let amount: Option<AccountingAmountModel> = if raw_amount.is_empty() {
                            None
                        } else {
                            Some(AccountingAmountModel::from_str(raw_amount)?)
                        };
                        let commodity: Option<H::M> = if raw_commodity.is_empty() {
                            None
                        } else {
                            Some(
                                from_str(raw_commodity)
                                    .map_err(|e| InvalidRon::with_debug("Commodity", &e))?,
                            )
                        };
                        let notes: Vec<String> = if raw_notes.is_empty() {
                            vec![]
                        } else {
                            raw_notes.lines().map(|n| n.into()).collect()
                        };

                        // Build.
                        commands.push(Command {
                            id: CommandSpecId((i + 2) as u64),
                            date: date.into(),
                            exec: exec.into(),
                            arguments,
                            description,
                            amount: amount.map(Into::into),
                            commodity,
                            notes,
                        });
                    } else {
                        // Parse transaction entry.
                        // --

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
                        let payee: H::P = from_str(raw_entity)
                            .map_err(|e| InvalidRon::with_debug("Payee", &e))?;
                        let description: String = raw_description.into();
                        let amount: AccountingAmountModel =
                            AccountingAmountModel::from_str(raw_amount)?;
                        let commodity: H::M = from_str(raw_commodity)
                            .map_err(|e| InvalidRon::with_debug("Commodity", &e))?;
                        let backing_account: BackingAccountModel<H::R, H::C, H::S> =
                            from_str(raw_backing_account)
                                .map_err(|e| InvalidRon::with_debug("BackingAccount", &e))?;
                        let custom_notes: Vec<Annotation> = if raw_notes.is_empty() {
                            vec![]
                        } else {
                            raw_notes
                                .lines()
                                .map(|n| Annotation::Custom(n.into()))
                                .collect()
                        };

                        // Build.
                        let spec = TransactionSpec {
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
                        };

                        // Run assertions.
                        self.validate(&spec)?;
                        transaction_specs.push(spec);
                    }

                    Ok((transaction_specs, commands))
                },
            )
    }

    async fn from_file<P>(
        &self,
        path: P,
    ) -> Result<(Vec<TransactionSpec<H>>, Vec<Command<H>>), ServerError>
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

impl<H: Handlers> TransactionsCsvDatasourceImpl<H> {
    fn validate(&self, spec: &TransactionSpec<H>) -> Result<(), ServerError> {
        if let Some(until) = spec.accrual_end {
            if until < spec.accrual_start {
                return Err(InvalidCsvContent::with_debug(
                    "Until date must be after start date.",
                    &spec,
                )
                .into());
            }
        }
        Ok(())
    }
}
