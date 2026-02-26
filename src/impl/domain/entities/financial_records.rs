// Before processing.
// ---

use std::collections::HashMap;

use crate::{
    domain::logic::spec_processor::UnreimbursedEntry,
    entities::{
        Annotation, Assertion, AssertionSpec, Command, DecoratedTransactionSpec, MetaEntry,
        Transaction, TransactionLabel, TransactionSpec, TransactionSpecId,
    },
};

use super::{account::LiabilityAccount, handlers::Handlers};

// 0. Before processing.
// ---

#[derive(Debug)]
pub(crate) struct FinancialRecordSpecs<H: Handlers> {
    pub transaction_specs: Vec<TransactionSpec<H>>,
    pub assertion_specs: Vec<AssertionSpec<H>>,
    pub commands: Vec<Command<H>>,
}

// 1. After decorator processing.
// ---

#[derive(Debug)]
#[allow(non_camel_case_types)]
pub(crate) struct FinancialRecords_Intermediate1<H: Handlers> {
    pub transaction_specs: Vec<DecoratedTransactionSpec<H>>,
    pub assertion_specs: Vec<AssertionSpec<H>>,
    pub commands: Vec<Command<H>>,
}

// 2. After spec processing.
// ---

#[derive(Debug)]
#[allow(non_camel_case_types)]
pub(crate) struct FinancialRecords_Intermediate2<H: Handlers> {
    pub transactions: Vec<Transaction>,
    pub assertions: Vec<Assertion>,
    pub commands: Vec<Command<H>>,
    pub ext_raw: Vec<String>,
    pub label_lookup: HashMap<TransactionSpecId, TransactionLabel>,
    pub annotations_lookup: HashMap<TransactionSpecId, Vec<Annotation>>,
    pub unreimbursed_entries: Vec<(LiabilityAccount, UnreimbursedEntry)>,
}

// 3. After command processing.
// ---

#[derive(Debug, Clone)]
pub struct FinancialRecords {
    pub transactions: Vec<Transaction>,
    pub assertions: Vec<Assertion>,
    pub meta_entries: Vec<MetaEntry>,
    pub ext_raw: Vec<String>,
    pub label_lookup: HashMap<TransactionSpecId, TransactionLabel>,
    pub annotations_lookup: HashMap<TransactionSpecId, Vec<Annotation>>,
    pub unreimbursed_entries: Vec<(LiabilityAccount, UnreimbursedEntry)>,
}

#[derive(Debug, Clone)]
pub struct NotesToFinancialRecords {
    pub transaction_notes: Vec<(String, String)>,
    pub general_notes: Vec<(String, String)>,
}
