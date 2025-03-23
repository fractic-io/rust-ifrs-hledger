pub enum CashflowTag {
    // Operating activities.
    // =========================================================================
    //
    // Non-cash expenses.
    Depreciation,
    Amortization,
    OtherNonCashExpense,
    //
    // Changes in working capital.
    DiffAccountsReceivable,
    DiffInventory,
    DiffPrepaidExpenses,
    DiffOtherCurrentAssets,
    DiffAccountsPayable,
    DiffAccruedExpenses,
    DiffDeferredRevenue,
    DiffOtherCurrentLiabilities,

    // Cash flows to reclassify.
    GainLossOnSaleOfAssets,

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
