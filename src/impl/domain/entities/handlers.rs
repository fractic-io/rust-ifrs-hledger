use serde::Deserialize;

use super::{
    account::{AssetAccount, ExpenseAccount, IncomeAccount, LiabilityAccount},
    decorator_logic::DecoratorLogic,
};

// Account handlers.
// ---

pub trait AssetHandler: for<'de> Deserialize<'de> {
    fn account(&self) -> AssetAccount;
    fn while_prepaid(&self) -> AssetAccount;
    fn while_payable(&self) -> LiabilityAccount;
    fn upon_accrual(&self) -> Option<ExpenseAccount>;
}

pub trait IncomeHandler: for<'de> Deserialize<'de> {
    fn account(&self) -> IncomeAccount;
    fn while_prepaid(&self) -> LiabilityAccount;
    fn while_receivable(&self) -> AssetAccount;
}

pub trait ExpenseHandler: for<'de> Deserialize<'de> {
    fn account(&self) -> ExpenseAccount;
    fn while_prepaid(&self) -> AssetAccount;
    fn while_payable(&self) -> LiabilityAccount;
}

pub trait ReimbursableEntityHandler: for<'de> Deserialize<'de> {
    fn account(&self) -> LiabilityAccount;
}

pub trait CashHandler: for<'de> Deserialize<'de> {
    fn account(&self) -> AssetAccount;
}

// Other.
// ---

pub trait DecoratorHandler: for<'de> Deserialize<'de> {
    fn logic(&self) -> Box<dyn DecoratorLogic>;
}

pub trait CommodityHandler: for<'de> Deserialize<'de> {
    fn iso_symbol(&self) -> String;
}

// Some type-magic to combine all handlers into a single type, greatly
// shortening type parameters.
// ---

pub trait Handlers {
    type A: AssetHandler;
    type I: IncomeHandler;
    type E: ExpenseHandler;
    type R: ReimbursableEntityHandler;
    type C: CashHandler;
    type D: DecoratorHandler;
    type M: CommodityHandler;
}

pub(crate) struct HandlersImpl<A, I, E, R, C, D, M> {
    pub _phantom: std::marker::PhantomData<(A, I, E, R, C, D, M)>,
}

impl<A, I, E, R, C, D, M> Handlers for HandlersImpl<A, I, E, R, C, D, M>
where
    A: AssetHandler,
    I: IncomeHandler,
    E: ExpenseHandler,
    R: ReimbursableEntityHandler,
    C: CashHandler,
    D: DecoratorHandler,
    M: CommodityHandler,
{
    type A = A;
    type I = I;
    type E = E;
    type R = R;
    type C = C;
    type D = D;
    type M = M;
}
