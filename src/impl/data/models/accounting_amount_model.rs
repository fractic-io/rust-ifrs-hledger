use std::str::FromStr;

use fractic_server_error::ServerError;

use crate::errors::InvalidAccountingAmount;

#[derive(Debug)]
pub(crate) struct AccountingAmountModel(pub f64);
impl FromStr for AccountingAmountModel {
    type Err = ServerError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let raw = s.replace(",", "");
        let is_negative = raw.trim().starts_with("(") && raw.trim().ends_with(")");
        let numeric_part = raw.trim_matches(|c| c == '(' || c == ')');
        let amount = numeric_part
            .parse::<f64>()
            .map_err(|_| InvalidAccountingAmount::new(numeric_part))?;
        Ok(AccountingAmountModel(if is_negative {
            -amount
        } else {
            amount
        }))
    }
}

impl Into<f64> for AccountingAmountModel {
    fn into(self) -> f64 {
        self.0
    }
}
