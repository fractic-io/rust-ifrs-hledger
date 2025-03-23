use crate::entities::CashflowTag;

impl CashflowTag {
    pub(crate) fn key() -> &'static str {
        "cf"
    }

    pub(crate) fn value(&self) -> String {
        match self {
            CashflowTag::Depreciation => "exp_depreciation",
            CashflowTag::Amortization => "exp_amortization",
            CashflowTag::OtherNonCashExpense => "exp_other_non_cash",
            CashflowTag::DiffAccountsReceivable => "diff_accounts_receivable",
            CashflowTag::DiffInventory => "diff_inventory",
            CashflowTag::DiffPrepaidExpenses => "diff_prepaid_expenses",
            CashflowTag::DiffOtherCurrentAssets => "diff_other_current_assets",
            CashflowTag::DiffAccountsPayable => "diff_accounts_payable",
            CashflowTag::DiffAccruedExpenses => "diff_accrued_expenses",
            CashflowTag::DiffDeferredRevenue => "diff_deferred_revenue",
            CashflowTag::DiffOtherCurrentLiabilities => "diff_other_current_liabilities",
            CashflowTag::GainLossOnSaleOfAssets => "gain_loss_sale_assets",
            CashflowTag::CashOutflowPpe => "cash_out_ppe",
            CashflowTag::CashOutflowIntangibleAssets => "cash_out_intangible_assets",
            CashflowTag::CashOutflowInvestmentSecurities => "cash_out_investment_securities",
            CashflowTag::CashOutflowLongTermDeposits => "cash_out_long_term_deposits",
            CashflowTag::CashOutflowOtherInvesting => "cash_out_other_investing",
            CashflowTag::CashInflowPpe => "cash_in_ppe",
            CashflowTag::CashInflowIntangibleAssets => "cash_in_intangible_assets",
            CashflowTag::CashInflowInvestmentSecurities => "cash_in_investment_securities",
            CashflowTag::CashInflowLongTermDeposits => "cash_in_long_term_deposits",
            CashflowTag::CashInflowOtherInvesting => "cash_in_other_investing",
            CashflowTag::CashInflowBorrowings => "cash_in_borrowings",
            CashflowTag::CashOutflowBorrowings => "cash_out_borrowings",
            CashflowTag::CashInflowIssuanceShares => "cash_in_issuance_shares",
            CashflowTag::CashOutflowShareBuybacks => "cash_out_share_buybacks",
            CashflowTag::CashOutflowDividends => "cash_out_dividends",
            CashflowTag::CashInOutflowOtherFinancing => "cash_in_out_other_financing",
        }
        .into()
    }
}
