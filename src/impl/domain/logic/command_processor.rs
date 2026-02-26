use base64::{engine::general_purpose::STANDARD, Engine as _};
use chrono::Datelike;
use fractic_server_error::ServerError;

use crate::{
    entities::{
        Command, CommandLogic, CommodityHandler, EndOfYearEntry, FinancialRecords,
        FinancialRecords_Intermediate2, Handlers, MacroContext, MacroHandler, Transaction,
    },
    errors::InvalidCsvContent,
};

pub(crate) struct CommandProcessor<H: Handlers> {
    specs: FinancialRecords_Intermediate2<H>,
}

impl<H: Handlers> CommandProcessor<H> {
    pub(crate) fn new(specs: FinancialRecords_Intermediate2<H>) -> Self {
        Self { specs }
    }

    pub(crate) fn process(self) -> Result<FinancialRecords, ServerError> {
        let FinancialRecords_Intermediate2 {
            transactions,
            assertions,
            commands,
            ledger_extensions,
            label_lookup,
            annotations_lookup,
            unreimbursed_entries,
        } = self.specs;

        let eoy_entries = commands
            .into_iter()
            .map(|spec| Self::process_command(spec, &transactions))
            .collect::<Result<Vec<EndOfYearEntry>, ServerError>>()?;

        Ok(FinancialRecords {
            transactions,
            assertions,
            eoy_entries,
            ledger_extensions,
            label_lookup,
            annotations_lookup,
            unreimbursed_entries,
        })
    }

    fn process_command(
        spec: Command<H>,
        transactions: &Vec<Transaction>,
    ) -> Result<EndOfYearEntry, ServerError> {
        let Command {
            id,
            date,
            exec: operation_logic,
            arguments,
            description,
            amount,
            commodity,
        } = spec;
        match operation_logic {
            CommandLogic::Close(logic) => {
                if date.month() != 12 || date.day() != 31 {
                    return Err(InvalidCsvContent::with_debug(
                        &format!(
                            "Close entry for {} (id {}) date must be December 31.",
                            date.year(),
                            id
                        ),
                        &date,
                    ));
                }
                let Some(argument) = arguments.into_iter().next() else {
                    return Err(InvalidCsvContent::new(&format!(
                        "Close entry for {} (id {}) must have argument field set to the closing \
                         tag, as output by the CloseRecordGenerator.",
                        date.year(),
                        id
                    )));
                };
                let content = STANDARD.decode(argument).map_err(|e| {
                    InvalidCsvContent::with_debug(
                        &format!(
                            "Close entry for {} (id {}) argument is not valid base64.",
                            date.year(),
                            id
                        ),
                        &e,
                    )
                })?;
                let entries =
                    serde_json::from_slice::<Vec<(String, f64)>>(&content).map_err(|e| {
                        InvalidCsvContent::with_debug(
                            &format!(
                                "Close entry for {} (id {}) argument is not valid postings JSON.",
                                date.year(),
                                id
                            ),
                            &e,
                        )
                    })?;
                Ok(EndOfYearEntry::Close {
                    date,
                    postings: entries,
                    logic,
                    total: amount,
                    currency: commodity.unwrap_or_else(|| H::M::default()).currency()?,
                })
            }
            CommandLogic::Correction(logic) => Ok(EndOfYearEntry::Correction {
                date,
                macro_output: logic.compile(
                    date,
                    arguments,
                    Some(MacroContext {
                        description,
                        amount,
                        currency: commodity.map(|c| c.currency()).transpose()?,
                    }),
                    Some(transactions),
                )?,
            }),
        }
    }
}
