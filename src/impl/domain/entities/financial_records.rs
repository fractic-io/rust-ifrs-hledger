// Before processing.
// ---

use std::collections::HashMap;

use crate::{
    domain::logic::spec_processor::UnreimbursedEntry,
    entities::{
        Annotation, Assertion, AssertionSpec, DecoratedTransactionSpec, Transaction,
        TransactionLabel, TransactionSpec, TransactionSpecId,
    },
};

use super::{account::LiabilityAccount, handlers::Handlers};

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
    pub transactions: Vec<Transaction>,
    pub assertions: Vec<Assertion>,
    pub label_lookup: HashMap<TransactionSpecId, TransactionLabel>,
    pub annotations_lookup: HashMap<TransactionSpecId, Vec<Annotation>>,
    pub unreimbursed_entries: Vec<(LiabilityAccount, UnreimbursedEntry)>,
}

#[derive(Debug, Clone)]
pub struct NotesToFinancialRecords {
    pub transaction_notes: Vec<(String, String)>,
    pub general_notes: Vec<(String, String)>,
}
