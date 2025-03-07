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
