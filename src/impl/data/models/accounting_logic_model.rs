#[derive(Debug, serde_derive::Deserialize)]
pub enum AccountingLogic<E, A, I, R> {
    SimpleExpense(E),
    Capitalize(A),
    Amortize(A),
    FixedExpense(E),
    VariableExpense(E),
    VariableExpenseInit {
        account: E,
        estimate: i64,
    },
    ImmaterialIncome(I),
    ImmaterialExpense(E),
    Reimburse(R),
    ClearVat {
        from: ISODateModel,
        to: ISODateModel,
    },
}
