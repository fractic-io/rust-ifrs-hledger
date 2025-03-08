use crate::entities::{Transaction, TransactionSpec};

use super::{
    account::Account, assertion::Assertion, assertion_spec::AssertionSpec, handlers::Handlers,
    note::Note,
};

// Before IFRS processing.
// ---

#[derive(Debug)]
pub struct FinancialRecordSpecs<H: Handlers> {
    pub transaction_specs: Vec<TransactionSpec<H>>,
    pub assertion_specs: Vec<AssertionSpec<H>>,
}

// After IFRS processing.
// ---

#[derive(Debug)]
pub struct FinancialRecords {
    pub accounts: Vec<Account>,
    pub transactions: Vec<Transaction>,
    pub assertions: Vec<Assertion>,
    pub notes: Vec<Note>,
}
