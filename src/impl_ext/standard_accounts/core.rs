use crate::entities::{expense, income, ExpenseAccount, IncomeAccount};
use std::sync::LazyLock;

pub static PAYMENT_FEES: LazyLock<ExpenseAccount> = LazyLock::new(|| expense("PaymentFees"));
pub static FX_GAIN: LazyLock<IncomeAccount> = LazyLock::new(|| income("FxGain"));
pub static FX_LOSS: LazyLock<ExpenseAccount> = LazyLock::new(|| expense("FxLoss"));
