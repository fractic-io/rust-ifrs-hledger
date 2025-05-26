use crate::entities::{
    asset, equity_tl, expense, expense_tl, income_tl, AssetAccount, AssetClassification,
    EquityAccount, EquityClassification, ExpenseAccount, ExpenseClassification, IncomeAccount,
    IncomeClassification,
};
use std::sync::LazyLock;

// For unpaid share capital, account depends on the business logic.
pub static UNPAID_SHARE_CAPITAL_AS_ASSET: LazyLock<AssetAccount> = LazyLock::new(|| {
    asset(
        "SubscribedCapitalReceivable",
        AssetClassification::OtherCurrentAssets,
    )
});
pub static UNPAID_SHARE_CAPITAL_AS_EQUITY: LazyLock<EquityAccount> =
    LazyLock::new(|| equity_tl(EquityClassification::UnpaidShareCapital));

pub static RETAINED_EARNINGS: LazyLock<EquityAccount> =
    LazyLock::new(|| equity_tl(EquityClassification::RetainedEarnings));

pub static PAYMENT_FEES: LazyLock<ExpenseAccount> = LazyLock::new(|| {
    expense(
        "PaymentFees",
        ExpenseClassification::GeneralAdministrativeExpenses,
    )
});
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
