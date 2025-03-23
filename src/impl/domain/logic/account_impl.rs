use crate::entities::{
    Account, AssetAccount, AssetClassification, CashflowTracingTag, EquityAccount,
    EquityClassification, LiabilityAccount, LiabilityClassification,
};

#[derive(Debug, PartialEq, Eq, Hash, Clone)]
pub enum Direction {
    Inflow,
    Outflow,
}

impl From<f64> for Direction {
    fn from(posting_amount: f64) -> Self {
        if posting_amount >= 0.0 {
            Direction::Outflow
        } else {
            Direction::Inflow
        }
    }
}

impl Account {
    pub(crate) fn cashflow_tag(
        &self,
        direction: impl Into<Direction>,
    ) -> Option<CashflowTracingTag> {
        match self {
            Account::Asset(AssetAccount(_, classification)) => match classification {
                AssetClassification::CashAndCashEquivalents
                | AssetClassification::AccountsReceivable
                | AssetClassification::Inventory
                | AssetClassification::PrepaidExpenses
                | AssetClassification::ShortTermInvestments
                | AssetClassification::ShortTermDeposits
                | AssetClassification::OtherCurrentAssets
                | AssetClassification::DeferredIncomeTax => None,

                AssetClassification::PropertyPlantEquipment => match direction.into() {
                    Direction::Inflow => Some(CashflowTracingTag::CashInflowPpe),
                    Direction::Outflow => Some(CashflowTracingTag::CashOutflowPpe),
                },
                AssetClassification::IntangibleAssets => match direction.into() {
                    Direction::Inflow => Some(CashflowTracingTag::CashInflowIntangibleAssets),
                    Direction::Outflow => Some(CashflowTracingTag::CashOutflowIntangibleAssets),
                },
                AssetClassification::LongTermInvestments => match direction.into() {
                    Direction::Inflow => Some(CashflowTracingTag::CashInflowInvestmentSecurities),
                    Direction::Outflow => Some(CashflowTracingTag::CashOutflowInvestmentSecurities),
                },
                AssetClassification::LongTermDeposits => match direction.into() {
                    Direction::Inflow => Some(CashflowTracingTag::CashInflowLongTermDeposits),
                    Direction::Outflow => Some(CashflowTracingTag::CashOutflowLongTermDeposits),
                },
                AssetClassification::OtherNonCurrentAssets => match direction.into() {
                    Direction::Inflow => Some(CashflowTracingTag::CashInflowOtherInvesting),
                    Direction::Outflow => Some(CashflowTracingTag::CashOutflowOtherInvesting),
                },
            },
            Account::Liability(LiabilityAccount(_, classification)) => match classification {
                LiabilityClassification::AccountsPayable
                | LiabilityClassification::AccruedExpenses
                | LiabilityClassification::DeferredRevenue
                | LiabilityClassification::ShortTermDebt
                | LiabilityClassification::OtherCurrentLiabilities
                | LiabilityClassification::DeferredIncomeTax => None,

                LiabilityClassification::LongTermDebt => match direction.into() {
                    Direction::Inflow => Some(CashflowTracingTag::CashInflowBorrowings),
                    Direction::Outflow => Some(CashflowTracingTag::CashOutflowBorrowings),
                },
                LiabilityClassification::OtherNonCurrentLiabilities => {
                    Some(CashflowTracingTag::CashInOutflowOtherFinancing)
                }
            },
            Account::Income(_) => None,
            Account::Expense(_) => None,
            Account::Equity(EquityAccount(_, classification)) => match classification {
                EquityClassification::CommonStock => match direction.into() {
                    Direction::Inflow => Some(CashflowTracingTag::CashInflowIssuanceShares),
                    Direction::Outflow => None,
                },
                EquityClassification::PreferredStock => match direction.into() {
                    Direction::Inflow => Some(CashflowTracingTag::CashInflowIssuanceShares),
                    Direction::Outflow => None,
                },
                EquityClassification::TreasuryStock => match direction.into() {
                    Direction::Inflow => None,
                    Direction::Outflow => Some(CashflowTracingTag::CashOutflowShareBuybacks),
                },
                EquityClassification::RetainedEarnings => None,
            },
        }
    }
}
