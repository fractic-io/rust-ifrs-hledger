use fractic_server_error::ServerError;
use iso_currency::Currency;
use serde::Deserialize;

use crate::errors::InvalidIsoCurrencyCode;

use super::{
    account::{
        AssetAccount, CapitalStockAccount, CashAccount, ExpenseAccount, IncomeAccount,
        LiabilityAccount,
    },
    decorator_logic::DecoratorLogic,
};

// Account handlers.
// ---

pub trait AssetHandler:
    for<'de> Deserialize<'de> + std::fmt::Debug + Clone + Send + Sync + 'static
{
    fn account(&self) -> AssetAccount;
    fn while_prepaid(&self) -> AssetAccount;
    fn while_payable(&self) -> LiabilityAccount;
    fn upon_accrual(&self) -> Option<ExpenseAccount>;
}

pub trait IncomeHandler:
    for<'de> Deserialize<'de> + std::fmt::Debug + Clone + Send + Sync + 'static
{
    fn account(&self) -> IncomeAccount;
    fn while_prepaid(&self) -> LiabilityAccount;
    fn while_receivable(&self) -> AssetAccount;
}

pub trait ExpenseHandler:
    for<'de> Deserialize<'de> + std::fmt::Debug + Clone + Send + Sync + 'static
{
    fn account(&self) -> ExpenseAccount;
    fn while_prepaid(&self) -> AssetAccount;
    fn while_payable(&self) -> LiabilityAccount;
}

pub trait CashHandler:
    for<'de> Deserialize<'de> + std::fmt::Debug + Clone + Send + Sync + 'static
{
    fn account(&self) -> CashAccount;
}

pub trait ShareholderHandler:
    for<'de> Deserialize<'de> + std::fmt::Debug + Clone + Send + Sync + 'static
{
    fn account(&self) -> CapitalStockAccount;
}

// Other.
// ---

pub trait PayeeHandler:
    for<'de> Deserialize<'de> + std::fmt::Debug + Clone + Send + Sync + 'static
{
    fn name(&self) -> String;
}

pub trait ReimbursableEntityHandler:
    for<'de> Deserialize<'de> + std::fmt::Debug + Clone + Send + Sync + 'static
{
    fn account(&self) -> LiabilityAccount;
}

pub trait DecoratorHandler:
    for<'de> Deserialize<'de> + std::fmt::Debug + Clone + Send + Sync + 'static
{
    fn logic<H: Handlers>(&self) -> Result<Box<dyn DecoratorLogic<H>>, ServerError>;
}

pub trait CommodityHandler:
    for<'de> Deserialize<'de> + std::fmt::Debug + Clone + Send + Sync + 'static
{
    fn iso_symbol(&self) -> String;
    fn currency(&self) -> Result<Currency, ServerError> {
        let s = self.iso_symbol();
        Currency::from_code(&s).ok_or_else(|| InvalidIsoCurrencyCode::new(&s))
    }
    fn default() -> Self;
}

// Some type-magic to combine all handlers into a single type, greatly
// shortening type parameters throughout the repo.
// ---

pub trait Handlers: std::fmt::Debug + Send + Sync + 'static {
    type A: AssetHandler;
    type I: IncomeHandler;
    type E: ExpenseHandler;
    type R: ReimbursableEntityHandler;
    type C: CashHandler;
    type S: ShareholderHandler;
    type D: DecoratorHandler;
    type M: CommodityHandler;
    type P: PayeeHandler;
}

#[derive(Debug)]
pub(crate) struct HandlersImpl<A, I, E, R, C, S, D, M, P> {
    pub _phantom: std::marker::PhantomData<(A, I, E, R, C, S, D, M, P)>,
}

impl<A, I, E, R, C, S, D, M, P> Handlers for HandlersImpl<A, I, E, R, C, S, D, M, P>
where
    A: AssetHandler,
    I: IncomeHandler,
    E: ExpenseHandler,
    R: ReimbursableEntityHandler,
    C: CashHandler,
    S: ShareholderHandler,
    D: DecoratorHandler,
    M: CommodityHandler,
    P: PayeeHandler,
{
    type A = A;
    type I = I;
    type E = E;
    type R = R;
    type C = C;
    type S = S;
    type D = D;
    type M = M;
    type P = P;
}
