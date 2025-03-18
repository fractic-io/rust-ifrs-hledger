use crate::entities::{asset, expense, liability, AssetAccount, ExpenseAccount, LiabilityAccount};
use std::sync::LazyLock;

pub static VAT_RECEIVABLE: LazyLock<AssetAccount> = LazyLock::new(|| asset("VatReceivable"));
pub static VAT_PAYABLE: LazyLock<LiabilityAccount> = LazyLock::new(|| liability("VatPayable"));
pub static VAT_PENDING_RECEIPT: LazyLock<AssetAccount> =
    LazyLock::new(|| asset("Temporary:VatPendingReceipt"));
pub static VAT_UNRECOVERABLE: LazyLock<ExpenseAccount> =
    LazyLock::new(|| expense("VatUnrecoverable"));
