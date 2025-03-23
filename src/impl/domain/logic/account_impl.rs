use crate::entities::{
    Account, AssetAccount, AssetClassification, CashflowTag, EquityAccount, EquityClassification,
    LiabilityAccount, LiabilityClassification,
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
    pub(crate) fn cashflow_tag(&self, direction: impl Into<Direction>) -> Option<CashflowTag> {
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
                    Direction::Inflow => Some(CashflowTag::CashInflowPpe),
                    Direction::Outflow => Some(CashflowTag::CashOutflowPpe),
                },
                AssetClassification::IntangibleAssets => match direction.into() {
                    Direction::Inflow => Some(CashflowTag::CashInflowIntangibleAssets),
                    Direction::Outflow => Some(CashflowTag::CashOutflowIntangibleAssets),
                },
                AssetClassification::LongTermInvestments => match direction.into() {
                    Direction::Inflow => Some(CashflowTag::CashInflowInvestmentSecurities),
                    Direction::Outflow => Some(CashflowTag::CashOutflowInvestmentSecurities),
                },
                AssetClassification::LongTermDeposits => match direction.into() {
                    Direction::Inflow => Some(CashflowTag::CashInflowLongTermDeposits),
                    Direction::Outflow => Some(CashflowTag::CashOutflowLongTermDeposits),
                },
                AssetClassification::OtherNonCurrentAssets => match direction.into() {
                    Direction::Inflow => Some(CashflowTag::CashInflowOtherInvesting),
                    Direction::Outflow => Some(CashflowTag::CashOutflowOtherInvesting),
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
                    Direction::Inflow => Some(CashflowTag::CashInflowBorrowings),
                    Direction::Outflow => Some(CashflowTag::CashOutflowBorrowings),
                },
                LiabilityClassification::OtherNonCurrentLiabilities => {
                    Some(CashflowTag::CashInOutflowOtherFinancing)
                }
            },
            Account::Income(_) => None,
            Account::Expense(_) => None,
            Account::Equity(EquityAccount(_, classification)) => match classification {
                EquityClassification::CommonStock => match direction.into() {
                    Direction::Inflow => Some(CashflowTag::CashInflowIssuanceShares),
                    Direction::Outflow => None,
                },
                EquityClassification::PreferredStock => match direction.into() {
                    Direction::Inflow => Some(CashflowTag::CashInflowIssuanceShares),
                    Direction::Outflow => None,
                },
                EquityClassification::TreasuryStock => match direction.into() {
                    Direction::Inflow => None,
                    Direction::Outflow => Some(CashflowTag::CashOutflowShareBuybacks),
                },
                EquityClassification::RetainedEarnings => None,
            },
        }
    }
}
