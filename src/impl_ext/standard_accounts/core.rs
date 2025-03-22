use crate::entities::{
    asset, equity_tl, expense, expense_tl, income_tl, AssetAccount, AssetClassification,
    EquityAccount, EquityClassification, ExpenseAccount, ExpenseClassification, IncomeAccount,
    IncomeClassification,
};
use std::sync::LazyLock;

pub static SHARE_CAPITAL_RECEIVABLE: LazyLock<AssetAccount> = LazyLock::new(|| {
    asset(
        "SubscribedCapitalReceivable",
        AssetClassification::OtherCurrentAssets,
    )
});
pub static RETAINED_EARNINGS: LazyLock<EquityAccount> =
    LazyLock::new(|| equity_tl(EquityClassification::RetainedEarnings));

pub static PAYMENT_FEES: LazyLock<ExpenseAccount> = LazyLock::new(|| {
    expense(
        "PaymentFees",
        ExpenseClassification::GeneralAdministrativeExpenses,
    )
});
pub static FX_GAIN: LazyLock<IncomeAccount> =
    LazyLock::new(|| income_tl(IncomeClassification::FxGain));
pub static FX_LOSS: LazyLock<ExpenseAccount> =
    LazyLock::new(|| expense_tl(ExpenseClassification::FxLoss));
