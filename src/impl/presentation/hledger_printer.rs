use crate::entities::{Account, FinancialRecords};

impl Account {
    fn ledger(&self) -> String {
        match self {
            Account::Asset(s) => format!("Assets:{}", s.0),
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
            ledger_output.push_str(&format!("{} {}\n", tx.date, tx.description));
            for posting in &tx.postings {
                ledger_output.push_str(&format!(
                    "    {:30} {:10.2} {}\n",
                    posting.account.ledger(),
                    posting.amount,
                    posting.commodity
                ));
            }
            for note in &tx.notes {
                ledger_output.push_str(&format!("    ; {}\n", note));
            }
            ledger_output.push('\n');
        }
        ledger_output
    }
}
