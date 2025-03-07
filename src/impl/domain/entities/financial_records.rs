pub struct FinancialRecordsSpec {
    pub accounts: Vec<Account>,
    pub transaction_specs: Vec<TransactionSpec>,
}

pub struct FinancialRecords {
    pub accounts: Vec<Account>,
    pub transactions: Vec<Transaction>,
}
