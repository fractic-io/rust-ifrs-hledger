#[derive(Debug, PartialEq, Eq, Hash)]
pub enum Account {
    Asset(AssetAccount),
    Liability(LiabilityAccount),
    Income(IncomeAccount),
    Expense(ExpenseAccount),
}

#[derive(Debug, PartialEq, Eq, Hash)]
pub struct AssetAccount(pub(crate) String);

#[derive(Debug, PartialEq, Eq, Hash)]
pub struct LiabilityAccount(pub(crate) String);

#[derive(Debug, PartialEq, Eq, Hash)]
pub struct IncomeAccount(pub(crate) String);

#[derive(Debug, PartialEq, Eq, Hash)]
pub struct ExpenseAccount(pub(crate) String);

// Shorthand constructors.

pub fn asset(name: impl Into<String>) -> AssetAccount {
    AssetAccount(name.into())
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
impl_into_account!(LiabilityAccount, Liability);
impl_into_account!(IncomeAccount, Income);
impl_into_account!(ExpenseAccount, Expense);
