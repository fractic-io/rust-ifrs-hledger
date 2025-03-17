use chrono::NaiveDate;

use crate::entities::{Annotation, AssertionSpec, Transaction};

use super::handlers::Handlers;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TransactionSpecId(pub(crate) u64);

#[derive(Debug)]
pub(crate) enum AccountingLogic<E, A, I, R> {
    SimpleExpense(E),
    Capitalize(A),
    Amortize(A),
    FixedExpense(E),
    VariableExpense(E),
    VariableExpenseInit { account: E, estimate: i64 },
    ImmaterialIncome(I),
    ImmaterialExpense(E),
    Reimburse(R),
    ClearVat { from: NaiveDate, to: NaiveDate },
}

#[derive(Debug)]
pub(crate) enum BackingAccount<R, C> {
    Reimburse(R),
    Cash(C),
}

#[derive(Debug)]
pub(crate) struct TransactionSpec<H: Handlers> {
    pub id: TransactionSpecId,
    pub accrual_start: NaiveDate,
    pub accrual_end: Option<NaiveDate>,
    pub payment_date: NaiveDate,
    pub accounting_logic: AccountingLogic<H::E, H::A, H::I, H::R>,
    pub decorators: Vec<H::D>,
    pub payee: H::P,
    pub description: String,
    pub amount: f64,
    pub commodity: H::M,
    pub backing_account: BackingAccount<H::R, H::C>,
    pub annotations: Vec<Annotation>,
}

#[derive(Debug)]
pub(crate) struct DecoratedTransactionSpec<H: Handlers> {
    pub id: TransactionSpecId,
    pub accrual_start: NaiveDate,
    pub accrual_end: Option<NaiveDate>,
    pub payment_date: NaiveDate,
    pub accounting_logic: AccountingLogic<H::E, H::A, H::I, H::R>,
    pub payee: H::P,
    pub description: String,
    pub amount: f64,
    pub commodity: H::M,
    pub backing_account: BackingAccount<H::R, H::C>,
    pub annotations: Vec<Annotation>,
    pub ext_transaction_specs: Vec<DecoratedTransactionSpec<H>>,
    pub ext_assertion_specs: Vec<AssertionSpec<H>>,
}

// --

impl std::fmt::Display for TransactionSpecId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}
