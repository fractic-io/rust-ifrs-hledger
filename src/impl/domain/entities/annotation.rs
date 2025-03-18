#[derive(Debug, Clone)]
pub enum Annotation {
    ImmaterialExpense,
    ImmaterialIncome,
    VariableExpense,
    VatKorea,
    VatKoreaUnrecoverable,
    VatKoreaReverseChargeExempt,
    CardFx,
    Custom(String),
}

impl std::fmt::Display for Annotation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Annotation::ImmaterialExpense => write!(f, "Expense recorded as immediately accrued on payment since the amount is considered immaterial."),
            Annotation::ImmaterialIncome => write!(f, "Income recorded as immediately accrued on receipt since the amount is considered immaterial."),
            Annotation::VariableExpense => write!(f, "Expense accrual at the end of each month was estimated using prior 90 days of historical data, then adjusted on payment to reflect any discrepancy."),
            Annotation::VatKorea => write!(f, "VAT is removed from transaction and separately recorded as VAT that is awaiting a proper receipt. Upon receipt, it is reclassified as a VAT receivable asset or payable liability. Upon VAT tax payment / refund, those accounts are cleared."),
            Annotation::VatKoreaUnrecoverable => write!(f, "Due to insufficient VAT receipts, the VAT charged for this purchase can not be claimed. As such, the entire cost of the purchase (including unrecoverable VAT) is recorded in the books. Any accrual logic or amortization is applied to the total cost."),
            Annotation::VatKoreaReverseChargeExempt => write!(f, "VAT was charged on a reverse-charge basis, meaning it is the company's responsibility to pay VAT through proxy payment. However, since the purchase is used for taxable business, the proxy payment is exempt. As such, the cost is simply recorded in the books without VAT."),
            Annotation::CardFx => write!(f, "Transaction amount was converted to the target currency using the latest available exchange rate at the time of payment. On settlement, the amount was adjusted to reflect the actual exchange rate."),
            Annotation::Custom(s) => write!(f, "{}", s),
        }
    }
}
