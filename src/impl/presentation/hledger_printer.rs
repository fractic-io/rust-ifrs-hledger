use crate::entities::{Account, FinancialRecords};

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
}

pub(crate) struct HledgerPrinter;

impl HledgerPrinter {
    pub(crate) fn new() -> Self {
        Self
    }

    pub(crate) fn print_ledger(&self, financial_records: &FinancialRecords) -> String {
        let mut ledger_output = String::new();
        for tx in &financial_records.transactions {
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
                    "    {:30} {:10.2} {}\n",
                    posting.account.ledger(),
                    posting.amount,
                    posting.currency
                ));
            }
            for annotation in financial_records
                .annotations_lookup
                .get(&tx.spec_id)
                .unwrap_or(&vec![])
            {
                ledger_output.push_str(&format!("    ; {}\n", annotation));
            }
            ledger_output.push('\n');
        }
        ledger_output
    }
}
