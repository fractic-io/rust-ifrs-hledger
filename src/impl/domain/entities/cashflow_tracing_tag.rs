pub enum CashflowTracingTag {
    // Operating activities.
    // =========================================================================
    //
    // Values for operating activities should be pulled from the income
    // statement; don't need to be tagged.
    //

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
    CashOutflowShareBuybacks,
    CashOutflowDividends,
    //
    // Other financing activities.
    CashInOutflowOtherFinancing,
}
