use crate::entities::{Transaction, TransactionSpec};

use super::{
    account::Account,
    assertion::Assertion,
    assertion_spec::AssertionSpec,
    handlers::{
        AssetHandler, CashHandler, CommodityHandler, DecoratorHandler, ExpenseHandler,
        IncomeHandler, ReimbursableEntityHandler,
    },
};

// Before IFRS processing.
// ---

pub struct FinancialRecordSpecs<A, I, E, C, R, D, M>
where
    A: AssetHandler,
    I: IncomeHandler,
    E: ExpenseHandler,
    C: CashHandler,
    R: ReimbursableEntityHandler,
    D: DecoratorHandler,
    M: CommodityHandler,
{
    pub transaction_specs: Vec<TransactionSpec<A, I, E, C, R, D, M>>,
    pub assertion_specs: Vec<AssertionSpec<C, M>>,
}

// After IFRS processing.
// ---

pub struct FinancialRecords {
    pub accounts: Vec<Account>,
    pub transactions: Vec<Transaction>,
    pub assertions: Vec<Assertion>,
    pub notes: Vec<String>,
}
