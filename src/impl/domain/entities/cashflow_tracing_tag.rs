#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CashflowTracingTag {
    // Operating activities.
    // =========================================================================
    //
    // Since we will use the indirect method, we only need to tag adjustments to
    // net income.
    //
    // Non-cash expenses.
    NonCashExpenseDepreciation,
    NonCashExpenseAmortization,
    NonCashExpenseOther,
    //
    // Cash expenses made with non-cash payment methods (ex. contributed
    // surplus) should also be treated as non-cash expenses.
    NonCashPayment,
    //
    // Cash flows to be pull out for reclassification.
    ReclassifyGainLossOnSaleOfAssets,

    // Investing activities.
    // =========================================================================
    //
    // Cash outflows.
    CashOutflowPpe,
    CashOutflowIntangibleAssets,
    CashOutflowInvestmentSecurities,
    CashOutflowLongTermDeposits,
    CashOutflowOtherInvesting,
    //
    // Cash inflows.
    CashInflowPpe,
    CashInflowIntangibleAssets,
    CashInflowInvestmentSecurities,
    CashInflowLongTermDeposits,
    CashInflowOtherInvesting,

    // Financing activities.
    // =========================================================================
    //
    // Debt-related cash flows.
    CashInflowBorrowings,
    CashOutflowBorrowings,
    //
    // Equity-related cash flows.
    CashInflowIssuanceShares,
    CashOutflowShareIssuanceCosts,
    CashOutflowShareBuybacks,
    CashOutflowDividends,
    //
    // Other financing activities.
    CashInOutflowOtherFinancing,
}
