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

                    // Financing and non-operating revenues.
                    IncomeClassification::InterestIncome => "Financing:InterestIncome",
                    IncomeClassification::DividendIncome => "Financing:DividendIncome",
                    IncomeClassification::RentalIncome => "Financing:RentalIncome",

                    // Gains from non-core activities.
                    IncomeClassification::GainOnSaleOfAssets => "NonOperating:GainOnSaleOfAssets",
                    IncomeClassification::FxGain => "NonOperating:FxGain",
                    IncomeClassification::VatAdjustmentIncome => "NonOperating:VatAdjustmentIncome",

                    IncomeClassification::OtherIncome => "Other",
                },
                match &s.0 {
                    Some(name) => format!(":{}", name),
                    None => "".to_string(),
                }
            ),
            Account::Expense(s) => format!(
                "Expenses:{}{}",
                match &s.1 {
                    // Direct costs for production.
                    ExpenseClassification::CostOfGoodsSold => "Operating:CostOfGoodsSold",

                    // Operating expenses.
                    ExpenseClassification::SellingExpenses => "Operating:SellingExpenses",
                    ExpenseClassification::GeneralAdministrativeExpenses =>
                        "Operating:GeneralAdministrativeExpenses",
                    ExpenseClassification::ResearchAndDevelopmentExpenses =>
                        "Operating:ResearchAndDevelopmentExpenses",
                    ExpenseClassification::CloudServicesExpenses =>
                        "Operating:CloudServicesExpenses",

                    // Depreciation & Amortization.
                    ExpenseClassification::DepreciationExpense => "Operating:Depreciation",
                    ExpenseClassification::AmortizationExpense => "Operating:Amortization",

                    // Financing and non-operating expenses.
                    ExpenseClassification::InterestExpense => "Financing:InterestExpense",
                    ExpenseClassification::IncomeTaxExpense => "Tax:IncomeTaxExpense",
                    ExpenseClassification::OtherTaxExpense => "Tax:OtherTaxExpense",

                    // Losses from non-core activities.
                    ExpenseClassification::LossOnSaleOfAssets => "NonOperating:LossOnSaleOfAssets",
                    ExpenseClassification::FxLoss => "NonOperating:FxLoss",
                    ExpenseClassification::VatAdjustmentExpense =>
                        "NonOperating:VatAdjustmentExpense",

                    ExpenseClassification::OtherCashExpense
                    | ExpenseClassification::OtherNonCashExpense => "Other",
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
