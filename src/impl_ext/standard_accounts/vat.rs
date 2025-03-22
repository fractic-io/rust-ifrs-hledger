use crate::entities::{
    asset, expense_tl, income_tl, liability, AssetAccount, AssetClassification, ExpenseAccount,
    ExpenseClassification, IncomeAccount, IncomeClassification, LiabilityAccount,
    LiabilityClassification,
};
use std::sync::LazyLock;

pub static VAT_PENDING_RECEIPT: LazyLock<AssetAccount> =
    LazyLock::new(|| asset("VatPendingReceipt", AssetClassification::OtherCurrentAssets));
pub static VAT_RECEIVABLE: LazyLock<AssetAccount> =
    LazyLock::new(|| asset("VatReceivable", AssetClassification::OtherCurrentAssets));
pub static VAT_PAYABLE: LazyLock<LiabilityAccount> = LazyLock::new(|| {
    liability(
        "VatPayable",
        LiabilityClassification::OtherCurrentLiabilities,
    )
});
pub static VAT_ADJUSTMENT_INCOME: LazyLock<IncomeAccount> =
    LazyLock::new(|| income_tl(IncomeClassification::VatAdjustmentIncome));
pub static VAT_ADJUSTMENT_EXPENSE: LazyLock<ExpenseAccount> =
    LazyLock::new(|| expense_tl(ExpenseClassification::VatAdjustmentExpense));
