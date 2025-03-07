// Account handlers.
// ---

use serde::Deserialize;

use super::{
    account::{AssetAccount, ExpenseAccount, IncomeAccount, LiabilityAccount},
    decorator_logic::DecoratorLogic,
};

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
