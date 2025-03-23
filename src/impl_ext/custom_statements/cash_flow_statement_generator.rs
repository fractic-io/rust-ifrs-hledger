use std::path::{Path, PathBuf};
use std::process::Command;

use fractic_server_error::{CriticalError, ServerError};

use crate::entities::{
    asset_tl, equity_tl, expense_tl, income_tl, liability_tl, Account, AssetClassification,
    EquityClassification, ExpenseClassification, IncomeClassification, LiabilityClassification,
};
use crate::errors::{HledgerBalanceInvalidTotal, HledgerCommandFailed, HledgerInvalidPath};

pub struct CashFlowStatementGenerator {
    ledger_path: PathBuf,
    period: String,
}

impl CashFlowStatementGenerator {
    pub fn new<P: AsRef<Path>>(ledger_path: P, period: &str) -> Result<Self, ServerError> {
        Ok(Self {
            ledger_path: ledger_path
                .as_ref()
                .canonicalize()
                .map_err(|e| {
                    HledgerInvalidPath::with_debug(
                        &ledger_path.as_ref().to_string_lossy().to_string(),
                        &e,
                    )
                })?
                .to_path_buf(),
            period: period.to_string(),
        })
    }

    pub fn generate(self) -> Result<String, ServerError> {
        // -------------------------------------
        // OPERATING ACTIVITIES
        // -------------------------------------

        let net_income = self.total_income()? - self.total_expenses()?;

        // Adjustments for Non-Cash Items.
        let depreciation_adj = self.expense(ExpenseClassification::DepreciationExpense)?;
        let amortization_adj = self.expense(ExpenseClassification::AmortizationExpense)?;

        // Adjustments for Non-Operating (Non-Cash) Gains/Losses.
        let gain_on_sale = self.income(IncomeClassification::GainOnSaleOfAssets)?;
        let loss_on_sale = self.expense(ExpenseClassification::LossOnSaleOfAssets)?;
        let sale_adj = loss_on_sale - gain_on_sale;

        let fx_gain = self.income(IncomeClassification::RealizedFxGain)?;
        let fx_loss = self.expense(ExpenseClassification::RealizedFxLoss)?;
        let fx_adj = fx_loss - fx_gain;

        let vat_income = self.income(IncomeClassification::VatAdjustmentIncome)?;
        let vat_expense = self.expense(ExpenseClassification::VatAdjustmentExpense)?;
        let vat_adj = vat_expense - vat_income;

        let non_cash_adjustments =
            depreciation_adj + amortization_adj + sale_adj + fx_adj + vat_adj;

        // Changes in Working Capital.
        //
        // For asset accounts, an increase uses cash (so we subtract the
        // change). For liabilities, an increase provides cash (so we add the
        // change).
        let ar = self.change_in_asset(AssetClassification::AccountsReceivable)?;
        let inventory = self.change_in_asset(AssetClassification::Inventory)?;
        let prepaid = self.change_in_asset(AssetClassification::PrepaidExpenses)?;
        let other_current_assets = self.change_in_asset(AssetClassification::OtherCurrentAssets)?;
        let wc_assets = -(ar + inventory + prepaid + other_current_assets);

        let accounts_payable =
            self.change_in_liability(LiabilityClassification::AccountsPayable)?;
        let accrued_expenses =
            self.change_in_liability(LiabilityClassification::AccruedExpenses)?;
        let deferred_revenue =
            self.change_in_liability(LiabilityClassification::DeferredRevenue)?;
        let other_current_liabilities =
            self.change_in_liability(LiabilityClassification::OtherCurrentLiabilities)?;
        let wc_liabilities =
            accounts_payable + accrued_expenses + deferred_revenue + other_current_liabilities;

        let working_capital_adjustments = wc_assets + wc_liabilities;

        // Net Cash Provided by Operating Activities.
        let operating_net = net_income + non_cash_adjustments + working_capital_adjustments;

        // -------------------------------------
        // INVESTING ACTIVITIES
        // -------------------------------------

        let ppe = self.change_in_asset(AssetClassification::PropertyPlantEquipment)?;
        let intangible = self.change_in_asset(AssetClassification::IntangibleAssets)?;
        let lt_investments = self.change_in_asset(AssetClassification::LongTermInvestments)?;
        let lt_deposits = self.change_in_asset(AssetClassification::LongTermDeposits)?;
        let other_noncurrent = self.change_in_asset(AssetClassification::OtherNonCurrentAssets)?;

        let mut investing_inflow = 0.0;
        let mut investing_outflow = 0.0;
        if ppe < 0.0 {
            investing_inflow += ppe.abs();
        } else {
            investing_outflow += ppe;
        }
        if intangible < 0.0 {
            investing_inflow += intangible.abs();
        } else {
            investing_outflow += intangible;
        }
        if lt_investments < 0.0 {
            investing_inflow += lt_investments.abs();
        } else {
            investing_outflow += lt_investments;
        }
        if lt_deposits < 0.0 {
            investing_inflow += lt_deposits.abs();
        } else {
            investing_outflow += lt_deposits;
        }
        if other_noncurrent < 0.0 {
            investing_inflow += other_noncurrent.abs();
        } else {
            investing_outflow += other_noncurrent;
        }

        let investing_net = -(ppe + intangible + lt_investments + lt_deposits + other_noncurrent);

        // -------------------------------------
        // FINANCING ACTIVITIES
        // -------------------------------------

        // Cash Flows from Debt Transactions.
        let short_term_debt = self.change_in_liability(LiabilityClassification::ShortTermDebt)?;
        let long_term_debt = self.change_in_liability(LiabilityClassification::LongTermDebt)?;
        let debt_flow = short_term_debt + long_term_debt;

        // Cash Flows from Equity Transactions.
        let common_stock = self.change_in_equity(EquityClassification::CommonStock)?;
        let preferred_stock = self.change_in_equity(EquityClassification::PreferredStock)?;
        let equity_flow = common_stock + preferred_stock;

        // Other Financing Activities are not explicitly tracked (assumed zero).
        let other_financing = 0.0;
        let financing_net = debt_flow + equity_flow + other_financing;

        // -------------------------------------
        // OUTPUT THE STATEMENT OF CASH FLOWS
        // -------------------------------------

        let mut output = String::new();
        output.push_str(&format!(
            "Statement of Cash Flows (Period: {})\n",
            self.period
        ));
        output.push_str("\n");
        output.push_str(&format!("Operating Activities:\n"));
        output.push_str(&format!("  1. Net Income: {:.2}\n", net_income));
        output.push_str(&format!("  2. Adjustments for Non-Cash Items:\n"));
        output.push_str(&format!("       Depreciation: {:.2}\n", depreciation_adj));
        output.push_str(&format!("       Amortization: {:.2}\n", amortization_adj));
        output.push_str(&format!(
            "  3. Adjustments for Non-Operating (Non-Cash) Gains/Losses:\n"
        ));
        output.push_str(&format!(
            "       Gain/Loss on Sale of Assets: {:.2}\n",
            sale_adj
        ));
        output.push_str(&format!(
            "       Foreign Exchange Adjustments: {:.2}\n",
            fx_adj
        ));
        output.push_str(&format!("       VAT Adjustments: {:.2}\n", vat_adj));
        output.push_str(&format!("  4. Changes in Working Capital:\n"));
        output.push_str(&format!("       Accounts Receivable: {:.2}\n", -ar));
        output.push_str(&format!("       Inventory: {:.2}\n", -inventory));
        output.push_str(&format!("       Prepaid Expenses: {:.2}\n", -prepaid));
        output.push_str(&format!(
            "       Other Current Assets: {:.2}\n",
            -other_current_assets
        ));
        output.push_str(&format!(
            "       Accounts Payable: {:.2}\n",
            accounts_payable
        ));
        output.push_str(&format!(
            "       Accrued Expenses: {:.2}\n",
            accrued_expenses
        ));
        output.push_str(&format!(
            "       Deferred Revenue: {:.2}\n",
            deferred_revenue
        ));
        output.push_str(&format!(
            "       Other Current Liabilities: {:.2}\n",
            other_current_liabilities
        ));
        output.push_str(&format!(
            "  5. Net Cash Provided by Operating Activities: {:.2}\n",
            operating_net
        ));
        output.push_str("\n");
        output.push_str(&format!("Investing Activities:\n"));
        output.push_str(&format!(
            "  1. Cash Outflows for Acquisitions of Long-Term Assets: {:.2}\n",
            investing_outflow
        ));
        output.push_str(&format!(
            "  2. Cash Inflows from Disposals of Long-Term Assets: {:.2}\n",
            investing_inflow
        ));
        output.push_str(&format!(
            "  3. Net Cash Used (or Provided) by Investing Activities: {:.2}\n",
            investing_net
        ));
        output.push_str("\n");
        output.push_str(&format!("Financing Activities:\n"));
        output.push_str(&format!(
            "  1. Cash Flows from Debt Transactions: {:.2}\n",
            debt_flow
        ));
        output.push_str(&format!(
            "  2. Cash Flows from Equity Transactions: {:.2}\n",
            equity_flow
        ));
        output.push_str(&format!(
            "  3. Other Financing Activities: {:.2}\n",
            other_financing
        ));
        output.push_str(&format!(
            "  4. Net Cash Provided (or Used) by Financing Activities: {:.2}\n",
            financing_net
        ));

        Ok(output)
    }

    fn run_hledger_sum(&self, account: &str) -> Result<f64, ServerError> {
        let account_query = format!("^{}($|:)", account);
        let output = Command::new("hledger")
            .arg("-f")
            .arg(&self.ledger_path)
            .arg("-p")
            .arg(&self.period)
            .arg("balance")
            .arg(account_query)
            .arg("--output-format=csv")
            .arg("--layout=bare")
            .output()
            .map_err(|e| {
                HledgerCommandFailed::with_debug(&self.ledger_path.display().to_string(), &e)
            })?;
        if !output.status.success() {
            return Err(HledgerCommandFailed::with_debug(
                &self.ledger_path.display().to_string(),
                &format!(
                    "failed with exit code: {}",
                    output.status.code().unwrap_or(-1)
                ),
            ));
        }
        let out_csv = String::from_utf8(output.stdout).map_err(|e| {
            CriticalError::with_debug("failed to parse hledger output as UTF-8", &e)
        })?;

        // Total is found in the last row, last column.
        let amount_str = csv::Reader::from_reader(out_csv.as_bytes())
            .records()
            .last()
            .and_then(|r| r.ok())
            .and_then(|r| r.iter().last().map(|s| s.to_owned()))
            .ok_or_else(|| HledgerBalanceInvalidTotal::new(account))?;
        let amount = amount_str
            .parse::<f64>()
            .map_err(|_| HledgerBalanceInvalidTotal::with_debug(account, &amount_str))?;

        Ok(amount)
    }

    fn total_income(&self) -> Result<f64, ServerError> {
        Ok(-self.run_hledger_sum("Income")?)
    }

    fn total_expenses(&self) -> Result<f64, ServerError> {
        self.run_hledger_sum("Expenses")
    }

    fn income(&self, classification: IncomeClassification) -> Result<f64, ServerError> {
        Ok(-self.run_hledger_sum(&Into::<Account>::into(income_tl(classification)).ledger())?)
    }

    fn expense(&self, classification: ExpenseClassification) -> Result<f64, ServerError> {
        self.run_hledger_sum(&Into::<Account>::into(expense_tl(classification)).ledger())
    }

    fn change_in_asset(&self, classification: AssetClassification) -> Result<f64, ServerError> {
        self.run_hledger_sum(&Into::<Account>::into(asset_tl(classification)).ledger())
    }

    fn change_in_liability(
        &self,
        classification: LiabilityClassification,
    ) -> Result<f64, ServerError> {
        Ok(-self.run_hledger_sum(&Into::<Account>::into(liability_tl(classification)).ledger())?)
    }

    fn change_in_equity(&self, classification: EquityClassification) -> Result<f64, ServerError> {
        Ok(-self.run_hledger_sum(&Into::<Account>::into(equity_tl(classification)).ledger())?)
    }
}
