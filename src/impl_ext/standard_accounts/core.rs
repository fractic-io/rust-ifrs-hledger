use crate::entities::{
    asset, equity_tl, expense, expense_tl, income_tl, liability, AssetAccount, AssetClassification,
    EquityAccount, EquityClassification, ExpenseAccount, ExpenseClassification, IncomeAccount,
    IncomeClassification, LiabilityAccount, LiabilityClassification,
};
use std::sync::LazyLock;

// Equity-related.
// ----------------------------------------------------------------------------

pub static RETAINED_EARNINGS: LazyLock<EquityAccount> =
    LazyLock::new(|| equity_tl(EquityClassification::RetainedEarnings));

// For unpaid share capital, account depends on the business logic.
pub static UNPAID_SHARE_CAPITAL_AS_ASSET: LazyLock<AssetAccount> = LazyLock::new(|| {
    asset(
        "SubscribedCapitalReceivable",
        AssetClassification::OtherCurrentAssets,
    )
});
pub static UNPAID_SHARE_CAPITAL_AS_EQUITY: LazyLock<EquityAccount> =
    LazyLock::new(|| equity_tl(EquityClassification::UnpaidShareCapital));

// For recording costs of issuing shares.
pub static PREPAID_SHARE_ISSUANCE_COSTS: LazyLock<AssetAccount> = LazyLock::new(|| {
    asset(
        "PrepaidShareIssuanceCosts",
        AssetClassification::OtherCurrentAssets,
    )
});
pub static SHARE_ISSUANCE_COSTS_PAYABLE: LazyLock<LiabilityAccount> = LazyLock::new(|| {
    liability(
        "ShareIssuanceCostsPayable",
        LiabilityClassification::OtherCurrentLiabilities,
    )
});

// FX-related.
// ----------------------------------------------------------------------------

pub static REALIZED_FX_GAIN: LazyLock<IncomeAccount> =
    LazyLock::new(|| income_tl(IncomeClassification::RealizedFxGain));
pub static REALIZED_FX_LOSS: LazyLock<ExpenseAccount> =
    LazyLock::new(|| expense_tl(ExpenseClassification::RealizedFxLoss));
pub static FOREIGN_TRANSACTION_FEE: LazyLock<ExpenseAccount> = LazyLock::new(|| {
    expense(
        "ForeignTransactionFee",
        ExpenseClassification::GeneralAdministrativeExpenses,
    )
});

// Tax-related.
// ----------------------------------------------------------------------------

pub static FOREIGN_WITHHOLDING_TAX: LazyLock<ExpenseAccount> = LazyLock::new(|| {
    expense(
        "ForeignWithholdingTax",
        ExpenseClassification::OtherTaxExpense,
    )
});

// Miscelanious.
// ----------------------------------------------------------------------------

pub static PAYMENT_FEES: LazyLock<ExpenseAccount> = LazyLock::new(|| {
    expense(
        "PaymentFees",
        ExpenseClassification::GeneralAdministrativeExpenses,
    )
});
