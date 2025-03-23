use crate::entities::CashflowTag;

impl CashflowTag {
    pub(crate) fn key() -> &'static str {
        "cft"
    }

    pub(crate) fn value(&self) -> String {
        match self {
            // Investing.
            CashflowTag::CashOutflowPpe => "out_ppe",
            CashflowTag::CashOutflowIntangibleAssets => "out_intangible_assets",
            CashflowTag::CashOutflowInvestmentSecurities => "out_investment_securities",
            CashflowTag::CashOutflowLongTermDeposits => "out_long_term_deposits",
            CashflowTag::CashOutflowOtherInvesting => "out_other_investing",
            CashflowTag::CashInflowPpe => "in_ppe",
            CashflowTag::CashInflowIntangibleAssets => "in_intangible_assets",
            CashflowTag::CashInflowInvestmentSecurities => "in_investment_securities",
            CashflowTag::CashInflowLongTermDeposits => "in_long_term_deposits",
            CashflowTag::CashInflowOtherInvesting => "in_other_investing",

            // Financing.
            CashflowTag::CashInflowBorrowings => "in_borrowings",
            CashflowTag::CashOutflowBorrowings => "out_borrowings",
            CashflowTag::CashInflowIssuanceShares => "in_issuance_shares",
            CashflowTag::CashOutflowShareBuybacks => "out_share_buybacks",
            CashflowTag::CashOutflowDividends => "out_dividends",
            CashflowTag::CashInOutflowOtherFinancing => "in_out_other_financing",
        }
        .into()
    }
}
