#[derive(Debug, PartialEq, Eq, Hash, Clone)]
pub enum Account {
    Asset(AssetAccount),
    Cash(CashAccount),
    Liability(LiabilityAccount),
    Income(IncomeAccount),
    Expense(ExpenseAccount),
    Equity(EquityAccount),
    CapitalStock(CapitalStockAccount),
}

#[derive(Debug, PartialEq, Eq, Hash, Clone)]
pub struct AssetAccount(pub(crate) String);

#[derive(Debug, PartialEq, Eq, Hash, Clone)]
pub struct CashAccount(pub(crate) String);

#[derive(Debug, PartialEq, Eq, Hash, Clone)]
pub struct LiabilityAccount(pub(crate) String);

#[derive(Debug, PartialEq, Eq, Hash, Clone)]
pub struct IncomeAccount(pub(crate) String);

#[derive(Debug, PartialEq, Eq, Hash, Clone)]
pub struct ExpenseAccount(pub(crate) String);

#[derive(Debug, PartialEq, Eq, Hash, Clone)]
pub struct EquityAccount(pub(crate) String);

#[derive(Debug, PartialEq, Eq, Hash, Clone)]
pub struct CapitalStockAccount(pub(crate) String);

// Shorthand constructors.

pub fn asset(name: impl Into<String>) -> AssetAccount {
    AssetAccount(name.into())
}

pub fn cash(name: impl Into<String>) -> CashAccount {
    CashAccount(name.into())
}

pub fn liability(name: impl Into<String>) -> LiabilityAccount {
    LiabilityAccount(name.into())
}

pub fn income(name: impl Into<String>) -> IncomeAccount {
    IncomeAccount(name.into())
}

pub fn expense(name: impl Into<String>) -> ExpenseAccount {
    ExpenseAccount(name.into())
}

pub fn equity(name: impl Into<String>) -> EquityAccount {
    EquityAccount(name.into())
}

pub fn shareholder(name: impl Into<String>) -> CapitalStockAccount {
    CapitalStockAccount(name.into())
}

// Easy conversion.

macro_rules! impl_into_account {
    ($typ:ty, $variant:ident) => {
        impl Into<Account> for $typ {
            fn into(self) -> Account {
                Account::$variant(self)
            }
        }
    };
}

impl_into_account!(AssetAccount, Asset);
impl_into_account!(CashAccount, Cash);
impl_into_account!(LiabilityAccount, Liability);
impl_into_account!(IncomeAccount, Income);
impl_into_account!(ExpenseAccount, Expense);
impl_into_account!(EquityAccount, Equity);
impl_into_account!(CapitalStockAccount, CapitalStock);
