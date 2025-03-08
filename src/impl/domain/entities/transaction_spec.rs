use chrono::NaiveDate;

use super::handlers::Handlers;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TransactionSpecId(pub(crate) u64);

#[derive(Debug)]
pub enum AccountingLogic<E, A, I, R> {
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
pub enum BackingAccount<R, C> {
    Reimburse(R),
    Cash(C),
}

#[derive(Debug)]
pub struct TransactionSpec<H: Handlers> {
    pub id: TransactionSpecId,
    pub accrual_date: NaiveDate,
    pub until: Option<NaiveDate>,
    pub payment_date: NaiveDate,
    pub accounting_logic: AccountingLogic<H::E, H::A, H::I, H::R>,
    pub decorators: Vec<H::D>,
    pub payee: H::P,
    pub description: String,
    pub amount: f64,
    pub commodity: H::M,
    pub backing_account: BackingAccount<H::R, H::C>,
}
