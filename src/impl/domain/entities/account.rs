#[derive(Debug, PartialEq, Eq, Hash, Clone)]
pub enum Account {
    Asset(AssetAccount),
    Liability(LiabilityAccount),
    Income(IncomeAccount),
    Expense(ExpenseAccount),
    Equity(EquityAccount),
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
    RealizedFxGain,
    /// VAT adjustments (discounts, rounding gains).
    VatAdjustmentIncome,

    // Other.
    // =========================================================================
    //
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
    SellingExpenses,
    /// Overhead costs, including office expenses, rent, utilities, and salaries
    /// of non-sales staff.
    GeneralAdministrativeExpenses,
    /// Costs incurred for R&D activities.
    ResearchAndDevelopmentExpenses,
    /// Costs incurred for cloud services (ex. AWS).
    CloudServicesExpenses,

    // Financing and non-operating expenses.
    // =========================================================================
    //
    /// Costs incurred on borrowings.
    InterestExpense,
    /// Income tax expense.
    IncomeTaxExpense,
    /// Other tax expenses.
    OtherTaxExpense,

    // Losses from non-core activities.
    // =========================================================================
    //
    /// Losses from the sale of long-term assets.
    LossOnSaleOfAssets,
    /// Losses from foreign exchange transactions.
    RealizedFxLoss,
    /// VAT adjustments (rounding losses)
    VatAdjustmentExpense,

    // Non-cash expenses.
    // =========================================================================
    //
    /// Allocation of cost for tangible assets.
    DepreciationExpense,
    /// Allocation of cost for intangible assets.
    AmortizationExpense,

    // Other.
    // =========================================================================
    //
    OtherCashExpense,
    OtherNonCashExpense,
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
pub struct AssetAccount(pub(crate) Option<String>, pub(crate) AssetClassification);

#[derive(Debug, PartialEq, Eq, Hash, Clone)]
pub struct LiabilityAccount(
    pub(crate) Option<String>,
    pub(crate) LiabilityClassification,
);

#[derive(Debug, PartialEq, Eq, Hash, Clone)]
pub struct IncomeAccount(pub(crate) Option<String>, pub(crate) IncomeClassification);

#[derive(Debug, PartialEq, Eq, Hash, Clone)]
pub struct ExpenseAccount(pub(crate) Option<String>, pub(crate) ExpenseClassification);

#[derive(Debug, PartialEq, Eq, Hash, Clone)]
pub struct EquityAccount(pub(crate) Option<String>, pub(crate) EquityClassification);

// Shorthand constructors.

pub fn asset(name: impl Into<String>, classification: AssetClassification) -> AssetAccount {
    AssetAccount(Some(name.into()), classification)
}
pub fn asset_tl(classification: AssetClassification) -> AssetAccount {
    AssetAccount(None, classification)
}

pub fn liability(
    name: impl Into<String>,
    classification: LiabilityClassification,
) -> LiabilityAccount {
    LiabilityAccount(Some(name.into()), classification)
}
pub fn liability_tl(classification: LiabilityClassification) -> LiabilityAccount {
    LiabilityAccount(None, classification)
}

pub fn income(name: impl Into<String>, classification: IncomeClassification) -> IncomeAccount {
    IncomeAccount(Some(name.into()), classification)
}
pub fn income_tl(classification: IncomeClassification) -> IncomeAccount {
    IncomeAccount(None, classification)
}

pub fn expense(name: impl Into<String>, classification: ExpenseClassification) -> ExpenseAccount {
    ExpenseAccount(Some(name.into()), classification)
}
pub fn expense_tl(classification: ExpenseClassification) -> ExpenseAccount {
    ExpenseAccount(None, classification)
}

pub fn equity(name: impl Into<String>, classification: EquityClassification) -> EquityAccount {
    EquityAccount(Some(name.into()), classification)
}
pub fn equity_tl(classification: EquityClassification) -> EquityAccount {
    EquityAccount(None, classification)
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
impl_into_account!(EquityAccount, Equity);
