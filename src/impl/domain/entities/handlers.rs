use fractic_server_error::ServerError;
use serde::Deserialize;

use super::{
    account::{AssetAccount, CashAccount, ExpenseAccount, IncomeAccount, LiabilityAccount},
    decorator_logic::DecoratorLogic,
};

// Account handlers.
// ---

pub trait AssetHandler: for<'de> Deserialize<'de> + std::fmt::Debug + Clone {
    fn account(&self) -> AssetAccount;
    fn while_prepaid(&self) -> AssetAccount;
    fn while_payable(&self) -> LiabilityAccount;
    fn upon_accrual(&self) -> Option<ExpenseAccount>;
}

pub trait IncomeHandler: for<'de> Deserialize<'de> + std::fmt::Debug + Clone {
    fn account(&self) -> IncomeAccount;
    fn while_prepaid(&self) -> LiabilityAccount;
    fn while_receivable(&self) -> AssetAccount;
}

pub trait ExpenseHandler: for<'de> Deserialize<'de> + std::fmt::Debug + Clone {
    fn account(&self) -> ExpenseAccount;
    fn while_prepaid(&self) -> AssetAccount;
    fn while_payable(&self) -> LiabilityAccount;
}

pub trait CashHandler: for<'de> Deserialize<'de> + std::fmt::Debug + Clone {
    fn account(&self) -> CashAccount;
}

// Other.
// ---

pub trait PayeeHandler: for<'de> Deserialize<'de> + std::fmt::Debug + Clone {
    fn name(&self) -> String;
}

pub trait ReimbursableEntityHandler: for<'de> Deserialize<'de> + std::fmt::Debug + Clone {
    fn account(&self) -> LiabilityAccount;
}

pub trait DecoratorHandler: for<'de> Deserialize<'de> + std::fmt::Debug + Clone {
    fn logic<H: Handlers>(&self) -> Result<Box<dyn DecoratorLogic<H>>, ServerError>;
}

pub trait CommodityHandler: for<'de> Deserialize<'de> + std::fmt::Debug + Clone {
    fn iso_symbol(&self) -> String;
}

// Some type-magic to combine all handlers into a single type, greatly
// shortening type parameters throughout the repo.
// ---

pub trait Handlers: std::fmt::Debug {
    type A: AssetHandler;
    type I: IncomeHandler;
    type E: ExpenseHandler;
    type R: ReimbursableEntityHandler;
    type C: CashHandler;
    type D: DecoratorHandler;
    type M: CommodityHandler;
    type P: PayeeHandler;
}

#[derive(Debug)]
pub(crate) struct HandlersImpl<A, I, E, R, C, D, M, P> {
    pub _phantom: std::marker::PhantomData<(A, I, E, R, C, D, M, P)>,
}

impl<A, I, E, R, C, D, M, P> Handlers for HandlersImpl<A, I, E, R, C, D, M, P>
where
    A: AssetHandler,
    I: IncomeHandler,
    E: ExpenseHandler,
    R: ReimbursableEntityHandler,
    C: CashHandler,
    D: DecoratorHandler,
    M: CommodityHandler,
    P: PayeeHandler,
{
    type A = A;
    type I = I;
    type E = E;
    type R = R;
    type C = C;
    type D = D;
    type M = M;
    type P = P;
}
