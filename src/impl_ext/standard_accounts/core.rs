use crate::entities::{expense, ExpenseAccount};
use std::sync::LazyLock;

pub static PAYMENT_FEES: LazyLock<ExpenseAccount> = LazyLock::new(|| expense("PaymentFees"));
