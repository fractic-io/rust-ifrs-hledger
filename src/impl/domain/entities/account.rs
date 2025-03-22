#[derive(Debug, PartialEq, Eq, Hash, Clone)]
pub enum Account {
    Asset(AssetAccount, AssetClassification),
    Liability(LiabilityAccount, LiabilityClassification),
    Income(IncomeAccount, IncomeClassification),
    Expense(ExpenseAccount, ExpenseClassification),
    Equity(EquityAccount, EquityClassification),
}

#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy)]
pub enum AssetClassification {
    // Current.
    // =========================================================================
    //
    CashAndCashEquivalents,
    AccountsReceivable,
    Inventory,
    PrepaidExpenses,
    ShortTermInvestments,
    ShortTermDeposits,
    OtherCurrentAssets,

    // Non-current.
    // =========================================================================
    //
    PropertyPlantEquipment,
    IntangibleAssets,
    LongTermInvestments,
    LongTermDeposits,
    DeferredIncomeTax,
    OtherNonCurrentAssets,
}

#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy)]
pub enum LiabilityClassification {
    // Current.
    // =========================================================================
    //
    AccountsPayable,
    AccruedExpenses,
    DeferredRevenue,
    ShortTermDebt,
    OtherCurrentLiabilities,

    // Non-current.
    // =========================================================================
    //
    LongTermDebt,
    DeferredIncomeTax,
    OtherNonCurrentLiabilities,
}

#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy)]
pub enum IncomeClassification {
    // Operating (core business) revenues.
    // =========================================================================
    //
    /// Revenue from selling goods.
    SalesRevenue,
    /// Revenue from providing services.
    ServiceRevenue,

    // Financing and non-operating revenues.
    // =========================================================================
    //
    /// Interest earned on deposits or investments.
    InterestIncome,
    /// Dividends received from investments.
    DividendIncome,
    /// Income from rental properties.
    RentalIncome,

    // Gains from non-core activities.
    // =========================================================================
    //
    /// Gains from the sale of long-term assets.
    GainOnSaleOfAssets,
    /// Gains from foreign exchange transactions.
    FxGain,

    OtherIncome,
}

#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy)]
pub enum ExpenseClassification {
    // Direct costs for production.
    // =========================================================================
    //
    /// Direct costs associated with production or purchase of goods.
    CostOfGoodsSold,

    // Operating expenses.
    // =========================================================================
    //
    /// Expenses related to selling, such as marketing, advertising, and sales
    /// commissions.
    SalesExpense,
    /// Overhead costs, including office expenses, rent, utilities, and salaries
    /// of non-sales staff.
    GeneralAdministrativeExpense,
    /// Costs incurred for R&D activities.
    ResearchAndDevelopmentExpense,
    /// Costs incurred for cloud services (ex. AWS).
    CloudServicesExpense,

    // Depreciation & Amortization.
    // =========================================================================
    //
    /// Allocation of cost for tangible assets.
    DepreciationExpense,
    /// Allocation of cost for intangible assets.
    AmortizationExpense,

    // Financing and non-operating expenses.
    // =========================================================================
    //
    /// Costs incurred on borrowings.
    InterestExpense,
    /// Income tax expense.
    IncomeTaxExpense,

    // Losses from non-core activities.
    // =========================================================================
    //
    /// Losses from the sale of long-term assets.
    LossOnSaleOfAssets,
    /// Losses from foreign exchange transactions.
    FxLoss,

    OtherExpense,
}

#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy)]
pub enum EquityClassification {
    // Contributed capital.
    // =========================================================================
    //
    CommonStock,
    PreferredStock,
    TreasuryStock,

    // Earned capital.
    // =========================================================================
    //
    RetainedEarnings,
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
