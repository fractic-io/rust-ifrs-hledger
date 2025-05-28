use chrono::NaiveDate;

use crate::entities::{Annotation, Assertion, Transaction};

use super::{
    account::Account,
    handlers::{CashHandler, Handlers, ReimbursableEntityHandler, ShareholderHandler},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TransactionSpecId(pub(crate) u64);

#[derive(Debug, serde_derive::Deserialize)]
pub enum CommonStockWhileUnpaid {
    ReceivableAsset,
    NegativeEquity,
}

#[derive(Debug)]
pub enum AccountingLogic<E, A, I, R, S> {
    CommonStock {
        subscriber: S,
        while_unpaid: CommonStockWhileUnpaid,
    },
    CostOfIssuingCommonStock(S),
    SimpleExpense(E),
    Capitalize(A),
    Amortize(A),
    FixedExpense(E),
    VariableExpense(E),
    VariableExpenseInit {
        account: E,
        estimate: i64,
    },
    ImmaterialIncome(I),
    ImmaterialExpense(E),
    Reimburse(R),
    ReimbursePartial(R),
    ClearVat {
        from: NaiveDate,
        to: NaiveDate,
    },
}

#[derive(Debug, Clone)]
pub enum BackingAccount<R, C, S> {
    Reimburse(R),
    Cash(C),
    ContributedSurplus(S),
}

#[derive(Debug)]
pub(crate) struct TransactionSpec<H: Handlers> {
    pub id: TransactionSpecId,
    pub accrual_start: NaiveDate,
    pub accrual_end: Option<NaiveDate>,
    pub payment_date: NaiveDate,
    pub accounting_logic: AccountingLogic<H::E, H::A, H::I, H::R, H::S>,
    pub decorators: Vec<H::D>,
    pub payee: H::P,
    pub description: String,
    pub amount: f64,
    pub commodity: H::M,
    pub backing_account: BackingAccount<H::R, H::C, H::S>,
    pub annotations: Vec<Annotation>,
}

#[derive(Debug)]
pub struct DecoratedTransactionSpec<H: Handlers> {
    pub id: TransactionSpecId,
    pub accrual_start: NaiveDate,
    pub accrual_end: Option<NaiveDate>,
    pub payment_date: NaiveDate,
    pub accounting_logic: AccountingLogic<H::E, H::A, H::I, H::R, H::S>,
    pub payee: H::P,
    pub description: String,
    pub amount: f64,
    pub commodity: H::M,
    pub backing_account: BackingAccount<H::R, H::C, H::S>,
    pub annotations: Vec<Annotation>,
    pub ext_transactions: Vec<Transaction>,
    pub ext_assertions: Vec<Assertion>,
}

// --

impl std::fmt::Display for TransactionSpecId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl<R, C, S> BackingAccount<R, C, S>
where
    R: ReimbursableEntityHandler,
    C: CashHandler,
    S: ShareholderHandler,
{
    pub fn account(&self) -> Account {
        match self {
            BackingAccount::Reimburse(r) => r.account().into(),
            BackingAccount::Cash(c) => c.account().into(),
            BackingAccount::ContributedSurplus(s) => s.upon_contributed_surplus().into(),
        }
    }

    pub fn is_reimburse(&self) -> bool {
        matches!(self, BackingAccount::Reimburse(_))
    }
}
