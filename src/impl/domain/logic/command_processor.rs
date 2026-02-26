use base64::{engine::general_purpose::STANDARD, Engine as _};
use chrono::Datelike;
use fractic_server_error::{CriticalError, ServerError};

use crate::{
    entities::{
        Assertion, Command, CommandLogic, CommodityHandler, EndOfYearEntry, FinancialRecords,
        FinancialRecords_Intermediate2, Handlers, MacroContext, MacroHandler, Transaction,
    },
    errors::InvalidCsvContent,
};

pub(crate) struct CommandProcessor<H: Handlers> {
    specs: FinancialRecords_Intermediate2<H>,
}

#[derive(Debug, Default)]
struct Delta {
    transactions: Vec<Transaction>,
    assertions: Vec<Assertion>,
    ledger_extensions: Vec<String>,
    eoy_entries: Vec<EndOfYearEntry>,
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

        // Collect deltas from each command.
        let deltas = commands
            .into_iter()
            .map(|command| {
                let delta = match command.exec {
                    CommandLogic::Close(_) => Self::process_close(command, &transactions)?,
                    CommandLogic::Correction(_) => {
                        Self::process_correction(command, &transactions)?
                    }
                };
                Ok(delta)
            })
            .collect::<Result<Vec<Delta>, ServerError>>()?;

        // Apply deltas to the current state.
        let (transactions, assertions, ledger_extensions, eoy_entries) = deltas.into_iter().fold(
            (transactions, assertions, ledger_extensions, Vec::new()),
            |(mut transactions, mut assertions, mut ledger_extensions, mut eoy_entries), delta| {
                transactions.extend(delta.transactions);
                assertions.extend(delta.assertions);
                ledger_extensions.extend(delta.ledger_extensions);
                eoy_entries.extend(delta.eoy_entries);
                (transactions, assertions, ledger_extensions, eoy_entries)
            },
        );

        // Return updated state.
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

    fn process_close(
        spec: Command<H>,
        _transactions: &Vec<Transaction>,
    ) -> Result<Delta, ServerError> {
        let Command {
            id,
            date,
            exec: CommandLogic::Close(logic),
            arguments,
            amount,
            commodity,
            ..
        } = spec
        else {
            return Err(CriticalError::new(
                "parant process() called incorrect sub-processor",
            ));
        };

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
                "Close entry for {} (id {}) must have argument field set to the closing tag, as \
                 output by the CloseRecordGenerator.",
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
        let entries = serde_json::from_slice::<Vec<(String, f64)>>(&content).map_err(|e| {
            InvalidCsvContent::with_debug(
                &format!(
                    "Close entry for {} (id {}) argument is not valid postings JSON.",
                    date.year(),
                    id
                ),
                &e,
            )
        })?;

        let eoy_entry = EndOfYearEntry::Close {
            date,
            postings: entries,
            logic,
            total: amount,
            currency: commodity.unwrap_or_else(|| H::M::default()).currency()?,
        };

        Ok(Delta {
            eoy_entries: vec![eoy_entry],
            ..Default::default()
        })
    }

    fn process_correction(
        spec: Command<H>,
        transactions: &Vec<Transaction>,
    ) -> Result<Delta, ServerError> {
        let Command {
            date,
            exec: CommandLogic::Correction(logic),
            arguments,
            description,
            amount,
            commodity,
            ..
        } = spec
        else {
            return Err(CriticalError::new(
                "parant process() called incorrect sub-processor",
            ));
        };

        let eoy_entry = EndOfYearEntry::Correction {
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
        };

        Ok(Delta {
            eoy_entries: vec![eoy_entry],
            ..Default::default()
        })
    }
}
