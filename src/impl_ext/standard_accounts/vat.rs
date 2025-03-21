use crate::entities::{
    asset, expense, income, liability, AssetAccount, ExpenseAccount, IncomeAccount,
    LiabilityAccount,
};
use std::sync::LazyLock;

pub static VAT_RECEIVABLE: LazyLock<AssetAccount> = LazyLock::new(|| asset("VatReceivable"));
pub static VAT_PAYABLE: LazyLock<LiabilityAccount> = LazyLock::new(|| liability("VatPayable"));
pub static VAT_PENDING_RECEIPT: LazyLock<AssetAccount> =
    LazyLock::new(|| asset("Temporary:VatPendingReceipt"));
pub static VAT_UNRECOVERABLE: LazyLock<ExpenseAccount> =
    LazyLock::new(|| expense("VatUnrecoverable"));
pub static VAT_ADJUSTMENT_INCOME: LazyLock<IncomeAccount> =
    LazyLock::new(|| income("VatAdjustmentIncome"));
pub static VAT_ADJUSTMENT_EXPENSE: LazyLock<ExpenseAccount> =
    LazyLock::new(|| expense("VatAdjustmentExpense"));
