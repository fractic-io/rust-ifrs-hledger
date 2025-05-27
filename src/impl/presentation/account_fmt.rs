use crate::entities::{
    Account, AssetAccount, AssetClassification, EquityClassification, ExpenseClassification,
    IncomeClassification, LiabilityClassification,
};

impl Account {
    pub(crate) fn ledger(&self) -> String {
        match self {
            Account::Asset(s) => format!(
                "Assets:{}{}",
                match &s.1 {
                    // Current.
                    AssetClassification::CashAndCashEquivalents => "Current:CashAndCashEquivalents",
                    AssetClassification::AccountsReceivable => "Current:AccountsReceivable",
                    AssetClassification::Inventory => "Current:Inventory",
                    AssetClassification::PrepaidExpenses => "Current:PrepaidExpenses",
                    AssetClassification::ShortTermInvestments => "Current:ShortTermInvestments",
                    AssetClassification::ShortTermDeposits => "Current:ShortTermDeposits",
                    AssetClassification::OtherCurrentAssets => "Current:OtherCurrentAssets",

                    // Non-current.
                    AssetClassification::PropertyPlantEquipment =>
                        "NonCurrent:PropertyPlantEquipment",
                    AssetClassification::IntangibleAssets => "NonCurrent:IntangibleAssets",
                    AssetClassification::LongTermInvestments => "NonCurrent:LongTermInvestments",
                    AssetClassification::LongTermDeposits => "NonCurrent:LongTermDeposits",
                    AssetClassification::DeferredIncomeTax => "NonCurrent:DeferredIncomeTax",
                    AssetClassification::OtherNonCurrentAssets =>
                        "NonCurrent:OtherNonCurrentAssets",
                },
                match &s.0 {
                    Some(name) => format!(":{}", name),
                    None => "".to_string(),
                }
            ),
            Account::Liability(s) => format!(
                "Liabilities:{}{}",
                match &s.1 {
                    // Current.
                    LiabilityClassification::AccountsPayable => "Current:AccountsPayable",
                    LiabilityClassification::AccruedExpenses => "Current:AccruedExpenses",
                    LiabilityClassification::DeferredRevenue => "Current:DeferredRevenue",
                    LiabilityClassification::ShortTermDebt => "Current:ShortTermDebt",
                    LiabilityClassification::OtherCurrentLiabilities =>
                        "Current:OtherCurrentLiabilities",

                    // Non-current.
                    LiabilityClassification::LongTermDebt => "NonCurrent:LongTermDebt",
                    LiabilityClassification::DeferredIncomeTax => "NonCurrent:DeferredIncomeTax",
                    LiabilityClassification::OtherNonCurrentLiabilities =>
                        "NonCurrent:OtherNonCurrentLiabilities",
                },
                match &s.0 {
                    Some(name) => format!(":{}", name),
                    None => "".to_string(),
                }
            ),
            Account::Income(s) => format!(
                "Income:{}{}",
                match &s.1 {
                    // Operating (core business) revenues.
                    IncomeClassification::SalesRevenue => "Operating:SalesRevenue",
                    IncomeClassification::ServiceRevenue => "Operating:ServiceRevenue",

                    // Financing revenues.
                    IncomeClassification::InterestIncome => "Financing:InterestIncome",
                    IncomeClassification::DividendIncome => "Financing:DividendIncome",
                    IncomeClassification::RentalIncome => "Financing:RentalIncome",

                    // Non-operating income.
                    IncomeClassification::NonCoreInterestIncome => "NonOperating:InterestIncome",
                    IncomeClassification::NonCoreDividendIncome => "NonOperating:DividendIncome",
                    IncomeClassification::NonCoreRentalIncome => "NonOperating:RentalIncome",
                    IncomeClassification::GainOnSaleOfAssets => "NonOperating:GainOnSaleOfAssets",
                    IncomeClassification::RealizedFxGain => "NonOperating:RealizedFxGain",
                    IncomeClassification::VatRefundGain => "NonOperating:VatRefundGain",
                    IncomeClassification::OtherNonOperatingIncome => "NonOperating:Other",
                },
                match &s.0 {
                    Some(name) => format!(":{}", name),
                    None => "".to_string(),
                }
            ),
            Account::Expense(s) => format!(
                "Expenses:{}{}",
                match &s.1 {
                    // Operating expenses.
                    ExpenseClassification::CostOfGoodsSold => "Operating:CostOfGoodsSold",
                    ExpenseClassification::SellingExpenses => "Operating:SellingExpenses",
                    ExpenseClassification::GeneralAdministrativeExpenses =>
                        "Operating:GeneralAdministrativeExpenses",
                    ExpenseClassification::ResearchAndDevelopmentExpenses =>
                        "Operating:ResearchAndDevelopmentExpenses",
                    ExpenseClassification::CloudServicesExpenses =>
                        "Operating:CloudServicesExpenses",
                    ExpenseClassification::DepreciationExpense => "Operating:Depreciation",
                    ExpenseClassification::AmortizationExpense => "Operating:Amortization",

                    // Financing expenses.
                    ExpenseClassification::InterestExpense => "Financing:InterestExpense",

                    // Tax expenses.
                    ExpenseClassification::IncomeTaxExpense => "Tax:IncomeTaxExpense",
                    ExpenseClassification::OtherTaxExpense => "Tax:OtherTaxExpense",

                    // Non-operating expenses.
                    ExpenseClassification::NonCoreInterestExpense => "NonOperating:InterestExpense",
                    ExpenseClassification::LossOnSaleOfAssets => "NonOperating:LossOnSaleOfAssets",
                    ExpenseClassification::RealizedFxLoss => "NonOperating:RealizedFxLoss",
                    ExpenseClassification::VatRefundLoss => "NonOperating:VatRefundLoss",
                    ExpenseClassification::OtherNonOperatingCashExpense
                    | ExpenseClassification::OtherNonOperatingNonCashExpense =>
                        "NonOperating:Other",
                },
                match &s.0 {
                    Some(name) => format!(":{}", name),
                    None => "".to_string(),
                }
            ),
            Account::Equity(s) => format!(
                "Equity:{}{}",
                match &s.1 {
                    // Contributed capital.
                    EquityClassification::CommonStock => "ContributedCapital:CommonStock",
                    EquityClassification::PreferredStock => "ContributedCapital:PreferredStock",
                    EquityClassification::TreasuryStock => "ContributedCapital:TreasuryStock",

                    // Earned capital.
                    EquityClassification::RetainedEarnings => "EarnedCapital:RetainedEarnings",

                    // Other.
                    EquityClassification::ShareIssuanceCosts => "Other:ShareIssuanceCosts",
                    EquityClassification::ContributedSurplus => "Other:ContributedSurplus",
                    EquityClassification::UnpaidShareCapital => "Other:UnpaidShareCapital",
                },
                match &s.0 {
                    Some(name) => format!(":{}", name),
                    None => "".to_string(),
                }
            ),
        }
    }

    pub(crate) fn type_tag(&self) -> char {
        match self {
            Account::Asset(AssetAccount(_, AssetClassification::CashAndCashEquivalents)) => 'C',
            Account::Asset(_) => 'A',
            Account::Liability(_) => 'L',
            Account::Income(_) => 'R',
            Account::Expense(_) => 'X',
            Account::Equity(_) => 'E',
        }
    }
}
