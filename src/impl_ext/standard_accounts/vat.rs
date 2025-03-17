use crate::entities::{asset, liability, AssetAccount, LiabilityAccount};
use std::sync::LazyLock;

pub(crate) static VAT_RECEIVABLE: LazyLock<AssetAccount> = LazyLock::new(|| asset("VatReceivable"));
pub(crate) static VAT_PAYABLE: LazyLock<LiabilityAccount> =
    LazyLock::new(|| liability("VatPayable"));
