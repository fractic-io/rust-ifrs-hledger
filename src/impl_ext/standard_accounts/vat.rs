use crate::entities::{asset, liability, AssetAccount, LiabilityAccount};
use std::sync::LazyLock;

pub static VAT_RECEIVABLE: LazyLock<AssetAccount> = LazyLock::new(|| asset("VatReceivable"));
pub static VAT_PAYABLE: LazyLock<LiabilityAccount> = LazyLock::new(|| liability("VatPayable"));
