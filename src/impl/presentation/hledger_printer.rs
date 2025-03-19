use std::collections::HashSet;

use iso_currency::Currency;
use num_format::{Locale, ToFormattedString};

use crate::entities::{Account, Assertion, FinancialRecords, Transaction};

impl Account {
    fn ledger(&self) -> String {
        match self {
            Account::Asset(s) => format!("Assets:{}", s.0),
            Account::Cash(s) => format!("Assets:Cash:{}", s.0),
            Account::Liability(s) => format!("Liabilities:{}", s.0),
            Account::Income(s) => format!("Income:{}", s.0),
            Account::Expense(s) => format!("Expenses:{}", s.0),
        }
    }

    fn type_tag(&self) -> char {
        match self {
            Account::Asset(_) => 'A',
            Account::Cash(_) => 'C',
            Account::Liability(_) => 'L',
            Account::Income(_) => 'R',
            Account::Expense(_) => 'X',
        }
    }
}

pub(crate) struct HledgerPrinter;

impl HledgerPrinter {
    pub(crate) fn new() -> Self {
        Self
    }

    pub(crate) fn print_ledger(&self, financial_records: &FinancialRecords) -> String {
        let mut ledger_output = String::new();

        ledger_output.push_str(
            "; --- Accounts -----------------------------------------------------------------\n\n",
        );
        self.print_accounts(&mut ledger_output, financial_records);
        ledger_output.push_str("\n\n");

        ledger_output.push_str(
            "; --- Commodities --------------------------------------------------------------\n\n",
        );
        self.print_commodities(&mut ledger_output, financial_records);
        ledger_output.push_str("\n\n");

        ledger_output.push_str(
            "; --- Payees -------------------------------------------------------------------\n\n",
        );
        self.print_payees(&mut ledger_output, financial_records);
        ledger_output.push_str("\n\n");

        ledger_output.push_str(
            "; --- Transactions -------------------------------------------------------------\n\n",
        );
        self.print_transactions(&mut ledger_output, financial_records);
        ledger_output.push_str("\n\n");

        ledger_output.push_str(
            "; --- Assertions ---------------------------------------------------------------\n\n",
        );
        self.print_assertions(&mut ledger_output, financial_records);

        ledger_output
    }

    fn print_accounts(&self, ledger_output: &mut String, financial_records: &FinancialRecords) {
        let accounts: HashSet<Account> = financial_records
            .transactions
            .iter()
            .flat_map(|tx| tx.postings.iter().map(|p| &p.account))
            .cloned()
            .collect();
        let sorted_account_declarations = {
            let mut v: Vec<String> = accounts
                .iter()
                .map(|account| {
                    format!(
                        "account {:61}  ; type: {}\n",
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
        let currencies: HashSet<Currency> = financial_records
            .transactions
            .iter()
            .flat_map(|tx| tx.postings.iter().map(|p| p.currency))
            .collect();
        let sorted_commodity_declarations = {
            let mut v: Vec<String> = currencies
                .iter()
                .map(|c| format!("commodity {}\n", self.format_amount(1000.0, *c)))
                .collect();
            v.sort();
            v
        };
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
                    "    {:55} {:>20}\n",
                    posting.account.ledger(),
                    self.format_amount(posting.amount, posting.currency),
                ));
            }
            for annotation in financial_records
                .annotations_lookup
                .get(&tx.spec_id)
                .unwrap_or(&vec![])
            {
                let s = annotation.to_string();
                let wrapped = textwrap::wrap(&s, 74);
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
                "    {:50} 0 == {:>20}\n",
                assertion.account.ledger(),
                self.format_amount(assertion.balance, assertion.currency),
            ));
            ledger_output.push('\n');
        }
    }

    fn format_amount(&self, amount: f64, currency: Currency) -> String {
        let decimal_places = currency.exponent().unwrap_or(0) as usize;
        let amount_integer_part = (amount.trunc() as i64).to_formatted_string(&Locale::en);
        let amount_fractional_part = format!("{:.decimal_places$}", amount.fract())
            .split('.')
            .nth(1)
            .map(|f| f.to_string())
            .unwrap_or_default();
        if decimal_places == 0 {
            return format!("{} {}", amount_integer_part, currency.symbol());
        } else {
            format!(
                "{}.{:0decimal_places$} {}",
                amount_integer_part,
                amount_fractional_part,
                currency.symbol(),
            )
        }
    }
}
