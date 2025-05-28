use crate::entities::CashflowTracingTag;

impl CashflowTracingTag {
    pub(crate) fn key() -> &'static str {
        "cft"
    }

    pub(crate) fn value(&self) -> String {
        match self {
            // Operating.
            CashflowTracingTag::NonCashExpenseDepreciation => "nce_depreciation",
            CashflowTracingTag::NonCashExpenseAmortization => "nce_amortization",
            CashflowTracingTag::NonCashExpenseOther => "nce_other",
            CashflowTracingTag::NonCashPayment => "non_cash_payment",
            CashflowTracingTag::ReclassifyGainLossOnSaleOfAssets => {
                "rcl_gain_loss_on_sale_of_assets"
            }

            // Investing.
            CashflowTracingTag::CashOutflowPpe => "out_ppe",
            CashflowTracingTag::CashOutflowIntangibleAssets => "out_intangible_assets",
            CashflowTracingTag::CashOutflowInvestmentSecurities => "out_investment_securities",
            CashflowTracingTag::CashOutflowLongTermDeposits => "out_long_term_deposits",
            CashflowTracingTag::CashOutflowOtherInvesting => "out_other_investing",
            CashflowTracingTag::CashInflowPpe => "in_ppe",
            CashflowTracingTag::CashInflowIntangibleAssets => "in_intangible_assets",
            CashflowTracingTag::CashInflowInvestmentSecurities => "in_investment_securities",
            CashflowTracingTag::CashInflowLongTermDeposits => "in_long_term_deposits",
            CashflowTracingTag::CashInflowOtherInvesting => "in_other_investing",

            // Financing.
            CashflowTracingTag::CashInflowBorrowings => "in_borrowings",
            CashflowTracingTag::CashOutflowBorrowings => "out_borrowings",
            CashflowTracingTag::CashInflowIssuanceShares => "in_issuance_shares",
            CashflowTracingTag::CashOutflowShareIssuanceCosts => "out_share_issuance_costs",
            CashflowTracingTag::CashOutflowShareBuybacks => "out_share_buybacks",
            CashflowTracingTag::CashOutflowDividends => "out_dividends",
            CashflowTracingTag::CashInOutflowOtherFinancing => "in_out_other_financing",
        }
        .into()
    }
}
