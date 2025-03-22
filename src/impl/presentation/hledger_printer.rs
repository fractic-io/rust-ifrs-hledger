use std::collections::HashSet;

use iso_currency::Currency;
use num_format::{Locale, ToFormattedString};

use crate::entities::{Account, Assertion, FinancialRecords, Transaction};

pub(crate) struct HledgerPrinter;

impl HledgerPrinter {
    pub(crate) fn new() -> Self {
        Self
    }

    pub(crate) fn print_ledger(&self, financial_records: &FinancialRecords) -> String {
        let mut ledger_output = String::new();

        ledger_output.push_str(
            "; --- Accounts -------------------------------------------------------------------------------------\n\n",
        );
        self.print_accounts(&mut ledger_output, financial_records);
        ledger_output.push_str("\n\n");

        ledger_output.push_str(
            "; --- Commodities ----------------------------------------------------------------------------------\n\n",
        );
        self.print_commodities(&mut ledger_output, financial_records);
        ledger_output.push_str("\n\n");

        ledger_output.push_str(
            "; --- Payees ---------------------------------------------------------------------------------------\n\n",
        );
        self.print_payees(&mut ledger_output, financial_records);
        ledger_output.push_str("\n\n");

        ledger_output.push_str(
            "; --- Transactions ---------------------------------------------------------------------------------\n\n",
        );
        self.print_transactions(&mut ledger_output, financial_records);
        ledger_output.push_str("\n\n");

        ledger_output.push_str(
            "; --- Assertions -----------------------------------------------------------------------------------\n\n",
        );
        self.print_assertions(&mut ledger_output, financial_records);

        ledger_output
    }

    fn print_accounts(&self, ledger_output: &mut String, financial_records: &FinancialRecords) {
        let accounts: HashSet<&Account> = financial_records
            .transactions
            .iter()
            .flat_map(|tx| tx.postings.iter().map(|p| &p.account))
            .chain(financial_records.assertions.iter().map(|a| &a.account))
            .collect();
        let sorted_account_declarations = {
            let mut v: Vec<String> = accounts
                .iter()
                .map(|account| {
                    format!(
                        "account {:81}  ; type: {}\n",
                        account.ledger(),
                        account.type_tag()
                    )
                })
                .collect();
            v.sort();
            v
        };
        for d in sorted_account_declarations {
            ledger_output.push_str(&d);
        }
    }

    fn print_commodities(&self, ledger_output: &mut String, financial_records: &FinancialRecords) {
        const SAMPLE_AMOUNT: f64 = 1000.0;
        let currencies: HashSet<Currency> = financial_records
            .transactions
            .iter()
            .flat_map(|tx| tx.postings.iter().map(|p| p.currency))
            .collect();
        let sorted_commodity_declarations = {
            let mut v: Vec<String> = currencies
                .iter()
                .map(|c| {
                    format!(
                        "commodity {}\n",
                        self.format_amount(SAMPLE_AMOUNT, *c, true)
                    )
                })
                .collect();
            v.sort();
            v
        };
        ledger_output.push_str("decimal-mark .\n");
        for d in sorted_commodity_declarations {
            ledger_output.push_str(&d);
        }
    }

    fn print_payees(&self, ledger_output: &mut String, financial_records: &FinancialRecords) {
        let payees: HashSet<&String> = financial_records
            .label_lookup
            .values()
            .map(|label| &label.payee)
            .collect();
        let sorted_payee_declarations = {
            let mut v: Vec<String> = payees.iter().map(|p| format!("payee {}\n", p)).collect();
            v.sort();
            v
        };
        for p in sorted_payee_declarations {
            ledger_output.push_str(&p);
        }
    }

    fn print_transactions(&self, ledger_output: &mut String, financial_records: &FinancialRecords) {
        let sorted_transactions = {
            let mut v: Vec<&Transaction> = financial_records.transactions.iter().collect();
            v.sort_by_key(|tx| tx.date);
            v
        };
        for tx in sorted_transactions {
            let label = financial_records.label_lookup.get(&tx.spec_id).map_or(
                "(unknown)".to_string(),
                |label| {
                    if let Some(comment) = &tx.comment {
                        format!("{} | {}: {}", label.payee, comment, label.description)
                    } else {
                        format!("{} | {}", label.payee, label.description)
                    }
                },
            );
            ledger_output.push_str(&format!("{} ({}) {}\n", tx.date, tx.spec_id, label));
            for posting in &tx.postings {
                ledger_output.push_str(&format!(
                    "    {:75} {:>20}\n",
                    posting.account.ledger(),
                    self.format_amount(posting.amount, posting.currency, false),
                ));
            }
            for annotation in financial_records
                .annotations_lookup
                .get(&tx.spec_id)
                .unwrap_or(&vec![])
            {
                let s = annotation.to_string();
                let wrapped = textwrap::wrap(&s, 94);
                let prefix = "    ;";
                ledger_output.push_str(&format!("{}\n", prefix));
                for line in wrapped {
                    ledger_output.push_str(&format!("{} {}\n", prefix, line));
                }
            }
            ledger_output.push('\n');
        }
    }

    fn print_assertions(&self, ledger_output: &mut String, financial_records: &FinancialRecords) {
        let sorted_assertions = {
            let mut v: Vec<&Assertion> = financial_records.assertions.iter().collect();
            v.sort_by_key(|tx| tx.date);
            v
        };
        for assertion in sorted_assertions {
            ledger_output.push_str(&format!("{} <assertion>\n", assertion.date));
            ledger_output.push_str(&format!(
                "    {:70} 0 == {:>20}\n",
                assertion.account.ledger(),
                self.format_amount(assertion.balance, assertion.currency, false),
            ));
            ledger_output.push('\n');
        }
    }

    /// Standard number decimal places for the given currency
    /// (ex. JPY = 0, USD = 2).
    fn decimal_places(&self, currency: Currency) -> usize {
        currency.exponent().unwrap_or(0) as usize
    }

    /// Format cash amount with currency symbol, correct number of decimal places,
    /// and proper thousands separators.
    ///
    /// For consistency, uses en locale ('.' as decimal mark, i.e. 1,000.00)
    /// regardless of user's locale or currency. Could be generalized in the
    /// future.
    ///
    /// For currencies with 0 decimal places, a decimal mark is only included if
    /// 'trailing_decimal' is true. For other currencies, this flag has no
    /// effect.
    fn format_amount(&self, amount: f64, currency: Currency, trailing_decimal: bool) -> String {
        let decimal_places = self.decimal_places(currency);
        if decimal_places == 0 {
            let amount_rounded = (amount.round() as i64).to_formatted_string(&Locale::en);
            return format!(
                "{}{} {}",
                amount_rounded,
                if trailing_decimal { "." } else { "" },
                currency.symbol()
            );
        } else {
            let amount_integer_part = (amount.trunc() as i64).to_formatted_string(&Locale::en);
            let amount_fractional_part = format!("{:.decimal_places$}", amount.fract())
                .split('.')
                .nth(1)
                .map(|f| f.to_string())
                .unwrap_or_default();
            format!(
                "{}.{:0decimal_places$} {}",
                amount_integer_part,
                amount_fractional_part,
                currency.symbol(),
            )
        }
    }
}
