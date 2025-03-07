use chrono::NaiveDate;

use super::handlers::{
    AssetHandler, CashHandler, CommodityHandler, DecoratorHandler, ExpenseHandler, IncomeHandler,
    ReimbursableEntityHandler,
};

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

pub enum BackingAccount<R, C> {
    Reimburse(R),
    Cash(C),
}

pub struct TransactionSpec<A, I, E, C, R, D, M>
where
    A: AssetHandler,
    I: IncomeHandler,
    E: ExpenseHandler,
    C: CashHandler,
    R: ReimbursableEntityHandler,
    D: DecoratorHandler,
    M: CommodityHandler,
{
    pub accrual_date: NaiveDate,
    pub until: Option<NaiveDate>,
    pub payment_date: NaiveDate,
    pub accounting_logic: AccountingLogic<E, A, I, R>,
    pub decorators: Vec<D>,
    pub entity: String,
    pub description: String,
    pub amount: f64,
    pub commodity: M,
    pub backing_account: BackingAccount<R, C>,
}
