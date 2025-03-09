// Before processing.
// ---

use std::collections::HashMap;

use crate::entities::{
    Annotation, Assertion, AssertionSpec, DecoratedTransactionSpec, Transaction, TransactionLabel,
    TransactionSpec, TransactionSpecId,
};

use super::{account::Account, handlers::Handlers};

#[derive(Debug)]
pub(crate) struct FinancialRecordSpecs<H: Handlers> {
    pub transaction_specs: Vec<TransactionSpec<H>>,
    pub assertion_specs: Vec<AssertionSpec<H>>,
}

// After decorator processing.
// ---

#[derive(Debug)]
pub(crate) struct DecoratedFinancialRecordSpecs<H: Handlers> {
    pub transaction_specs: Vec<DecoratedTransactionSpec<H>>,
    pub assertion_specs: Vec<AssertionSpec<H>>,
}

// After spec + note processing.
// ---

#[derive(Debug, Clone)]
pub struct FinancialRecords {
    pub accounts: Vec<Account>,
    pub transactions: Vec<Transaction>,
    pub assertions: Vec<Assertion>,
    pub label_lookup: HashMap<TransactionSpecId, TransactionLabel>,
    pub annotations_lookup: HashMap<TransactionSpecId, Vec<Annotation>>,
}

#[derive(Debug, Clone)]
pub struct NotesToFinancialRecords {
    pub transaction_notes: Vec<String>,
    pub accounting_notes: Vec<String>,
}
