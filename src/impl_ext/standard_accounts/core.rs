use crate::entities::{
    asset, equity, expense, income, AssetAccount, EquityAccount, ExpenseAccount, IncomeAccount,
};
use std::sync::LazyLock;

pub static SHARE_CAPITAL_RECEIVABLE: LazyLock<AssetAccount> =
    LazyLock::new(|| asset("ShareCapitalReceivable"));
pub static RETAINED_EARNINGS: LazyLock<EquityAccount> =
    LazyLock::new(|| equity("RetainedEarnings"));

pub static PAYMENT_FEES: LazyLock<ExpenseAccount> = LazyLock::new(|| expense("PaymentFees"));
pub static FX_GAIN: LazyLock<IncomeAccount> = LazyLock::new(|| income("FxGain"));
pub static FX_LOSS: LazyLock<ExpenseAccount> = LazyLock::new(|| expense("FxLoss"));
