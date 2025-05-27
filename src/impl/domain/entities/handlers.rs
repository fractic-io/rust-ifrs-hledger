use fractic_server_error::ServerError;
use iso_currency::Currency;
use serde::Deserialize;

use crate::errors::InvalidIsoCurrencyCode;

use super::{
    account::{
        asset, asset_tl, equity, equity_tl, liability, liability_tl, AssetAccount,
        AssetClassification, EquityAccount, EquityClassification, ExpenseAccount, IncomeAccount,
        LiabilityAccount, LiabilityClassification,
    },
    decorator_logic::DecoratorLogic,
};

// Account handlers.
// ---

pub trait AssetHandler:
    for<'de> Deserialize<'de> + std::fmt::Debug + Clone + Send + Sync + 'static
{
    fn account(&self) -> AssetAccount;
    fn while_prepaid(&self) -> AssetAccount {
        match self.account().0 {
            Some(name) => asset(name, AssetClassification::PrepaidExpenses),
            None => asset_tl(AssetClassification::PrepaidExpenses),
        }
    }
    fn while_payable(&self) -> LiabilityAccount {
        match self.account().0 {
            Some(name) => liability(name, LiabilityClassification::AccountsPayable),
            None => liability_tl(LiabilityClassification::AccountsPayable),
        }
    }

    /// IMPORTANT: For correct cash flow statement generation, the destination
    /// of accrual adjustments should depend on whether the asset is current or
    /// non-current.
    ///
    /// For current assets:
    ///   The destination should be a regular cash expense account.
    ///
    /// For non-current assets:
    ///   The destination should be a depreciation, amortization, or other
    ///   non-cash expense account.
    fn upon_accrual(&self) -> Option<ExpenseAccount> {
        None
    }
}

pub trait IncomeHandler:
    for<'de> Deserialize<'de> + std::fmt::Debug + Clone + Send + Sync + 'static
{
    fn account(&self) -> IncomeAccount;
    fn while_prepaid(&self) -> LiabilityAccount {
        match self.account().0 {
            Some(name) => liability(name, LiabilityClassification::DeferredRevenue),
            None => liability_tl(LiabilityClassification::DeferredRevenue),
        }
    }
    fn while_receivable(&self) -> AssetAccount {
        match self.account().0 {
            Some(name) => asset(name, AssetClassification::AccountsReceivable),
            None => asset_tl(AssetClassification::AccountsReceivable),
        }
    }
}

pub trait ExpenseHandler:
    for<'de> Deserialize<'de> + std::fmt::Debug + Clone + Send + Sync + 'static
{
    fn account(&self) -> ExpenseAccount;
    fn while_prepaid(&self) -> AssetAccount {
        match self.account().0 {
            Some(name) => asset(name, AssetClassification::PrepaidExpenses),
            None => asset_tl(AssetClassification::PrepaidExpenses),
        }
    }
    fn while_payable(&self) -> LiabilityAccount {
        match self.account().0 {
            Some(name) => liability(name, LiabilityClassification::AccountsPayable),
            None => liability_tl(LiabilityClassification::AccountsPayable),
        }
    }
}

pub trait CashHandler:
    for<'de> Deserialize<'de> + std::fmt::Debug + Clone + Send + Sync + 'static
{
    fn account(&self) -> AssetAccount;
}

pub trait ShareholderHandler:
    for<'de> Deserialize<'de> + std::fmt::Debug + Clone + Send + Sync + 'static
{
    fn account(&self) -> EquityAccount;

    fn upon_contributed_surplus(&self) -> EquityAccount {
        match self.account().0 {
            Some(name) => equity(name, EquityClassification::ContributedSurplus),
            None => equity_tl(EquityClassification::ContributedSurplus),
        }
    }
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

    /// Smallest value that would display as a non-zero number.
    ///
    /// For example, for USD, this would be 0.005, since this is the smallest
    /// number that rounds up to a displayable value. For KRW, this would be
    /// 0.5.
    fn precision_cutoff(&self) -> Result<f64, ServerError> {
        let exp = self.currency()?.exponent().unwrap_or(0) as i32;
        Ok(1f64 / 10f64.powi(exp) / 2f64)
    }
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
