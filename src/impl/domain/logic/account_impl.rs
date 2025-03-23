use crate::entities::{
    Account, AssetAccount, AssetClassification, CashflowTag, EquityAccount, EquityClassification,
    ExpenseAccount, ExpenseClassification, IncomeAccount, IncomeClassification, LiabilityAccount,
    LiabilityClassification,
};

#[derive(Debug, PartialEq, Eq, Hash, Clone)]
pub enum Direction {
    Inflow,
    Outflow,
}

impl From<f64> for Direction {
    fn from(cash_amount: f64) -> Self {
        if cash_amount >= 0.0 {
            Direction::Inflow
        } else {
            Direction::Outflow
        }
    }
}

impl Account {
    pub(crate) fn cashflow_tag(&self, direction: impl Into<Direction>) -> Option<CashflowTag> {
        match self {
            Account::Asset(AssetAccount(_, classification)) => match classification {
                AssetClassification::CashAndCashEquivalents => None,
                AssetClassification::AccountsReceivable => {
                    Some(CashflowTag::DiffAccountsReceivable)
                }
                AssetClassification::Inventory => Some(CashflowTag::DiffInventory),
                AssetClassification::PrepaidExpenses => Some(CashflowTag::DiffPrepaidExpenses),
                AssetClassification::ShortTermInvestments => {
                    Some(CashflowTag::DiffOtherCurrentAssets)
                }
                AssetClassification::ShortTermDeposits => Some(CashflowTag::DiffOtherCurrentAssets),
                AssetClassification::OtherCurrentAssets => {
                    Some(CashflowTag::DiffOtherCurrentAssets)
                }
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
                AssetClassification::DeferredIncomeTax => None,
                AssetClassification::OtherNonCurrentAssets => match direction.into() {
                    Direction::Inflow => Some(CashflowTag::CashInflowOtherInvesting),
                    Direction::Outflow => Some(CashflowTag::CashOutflowOtherInvesting),
                },
            },
            Account::Liability(LiabilityAccount(_, classification)) => match classification {
                LiabilityClassification::AccountsPayable => Some(CashflowTag::DiffAccountsPayable),
                LiabilityClassification::AccruedExpenses => Some(CashflowTag::DiffAccruedExpenses),
                LiabilityClassification::DeferredRevenue => Some(CashflowTag::DiffDeferredRevenue),
                LiabilityClassification::ShortTermDebt => {
                    Some(CashflowTag::DiffOtherCurrentLiabilities)
                }
                LiabilityClassification::OtherCurrentLiabilities => {
                    Some(CashflowTag::DiffOtherCurrentLiabilities)
                }
                LiabilityClassification::LongTermDebt => match direction.into() {
                    Direction::Inflow => Some(CashflowTag::CashInflowBorrowings),
                    Direction::Outflow => Some(CashflowTag::CashOutflowBorrowings),
                },
                LiabilityClassification::DeferredIncomeTax => None,
                LiabilityClassification::OtherNonCurrentLiabilities => {
                    Some(CashflowTag::CashInOutflowOtherFinancing)
                }
            },
            Account::Income(IncomeAccount(_, classification)) => match classification {
                IncomeClassification::SalesRevenue => None,
                IncomeClassification::ServiceRevenue => None,
                IncomeClassification::InterestIncome => None,
                IncomeClassification::DividendIncome => None,
                IncomeClassification::RentalIncome => None,
                IncomeClassification::NonCoreInterestIncome => None,
                IncomeClassification::NonCoreDividendIncome => None,
                IncomeClassification::NonCoreRentalIncome => None,
                IncomeClassification::GainOnSaleOfAssets => {
                    Some(CashflowTag::GainLossOnSaleOfAssets)
                }
                IncomeClassification::RealizedFxGain => None,
                IncomeClassification::VatRefundGain => None,
                IncomeClassification::OtherNonOperatingIncome => None,
            },
            Account::Expense(ExpenseAccount(_, classification)) => match classification {
                ExpenseClassification::CostOfGoodsSold => None,
                ExpenseClassification::SellingExpenses => None,
                ExpenseClassification::GeneralAdministrativeExpenses => None,
                ExpenseClassification::ResearchAndDevelopmentExpenses => None,
                ExpenseClassification::CloudServicesExpenses => None,
                ExpenseClassification::DepreciationExpense => Some(CashflowTag::Depreciation),
                ExpenseClassification::AmortizationExpense => Some(CashflowTag::Amortization),
                ExpenseClassification::InterestExpense => None,
                ExpenseClassification::IncomeTaxExpense => None,
                ExpenseClassification::OtherTaxExpense => None,
                ExpenseClassification::NonCoreInterestExpense => None,
                ExpenseClassification::LossOnSaleOfAssets => {
                    Some(CashflowTag::GainLossOnSaleOfAssets)
                }
                ExpenseClassification::RealizedFxLoss => None,
                ExpenseClassification::VatRefundLoss => None,
                ExpenseClassification::OtherNonOperatingCashExpense => None,
                ExpenseClassification::OtherNonOperatingNonCashExpense => {
                    Some(CashflowTag::OtherNonCashExpense)
                }
            },
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
