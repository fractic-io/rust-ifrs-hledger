#[derive(Debug, Clone)]
pub enum Annotation {
    ImmaterialExpense,
    ImmaterialIncome,
    Custom(String),
}

impl std::fmt::Display for Annotation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Annotation::ImmaterialExpense => write!(f, "Expense recorded as immediately accrued on payment since the amount is considered immaterial."),
            Annotation::ImmaterialIncome => write!(f, "Income recorded as immediately accrued on receipt since the amount is considered immaterial."),
            Annotation::Custom(s) => write!(f, "{}", s),
        }
    }
}
