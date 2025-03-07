// Account handlers.
// ---

pub trait AssetHandler {
    fn account(&self) -> AssetAccount;
    fn while_prepaid(&self) -> AssetAccount;
    fn while_payable(&self) -> LiabilityAccount;
    fn upon_accrual(&self) -> Option<ExpenseAccount>;
}

pub trait IncomeHandler {
    fn account(&self) -> IncomeAccount;
    fn while_prepaid(&self) -> LiabilityAccount;
    fn while_receivable(&self) -> AssetAccount;
}

pub trait ExpenseHandler {
    fn account(&self) -> ExpenseAccount;
    fn while_prepaid(&self) -> AssetAccount;
    fn while_payable(&self) -> LiabilityAccount;
}

pub trait ReimbursableEntityHandler {
    fn account(&self) -> LiabilityAccount;
}

pub trait CashHandler {
    fn account(&self) -> AssetAccount;
}

// Other.
// ---

pub trait DecoratorHandler {
    fn logic(&self) -> DecoratorLogic;
}
