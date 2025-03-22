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

        // Net Income: computed as Income minus Expenses from the income statement.
        let income_total = self.total_income()?;
        let expense_total = self.total_expenses()?;
        let net_income = income_total - expense_total;

        // Adjustments for Non-Cash Items.
        let depreciation = self.expense_diff(ExpenseClassification::DepreciationExpense)?;
        let amortization = self.expense_diff(ExpenseClassification::AmortizationExpense)?;

        // Adjustments for Non-Operating (Non-Cash) Gains/Losses.
        let gain_on_sale = self.income_diff(IncomeClassification::GainOnSaleOfAssets)?;
        let loss_on_sale = self.expense_diff(ExpenseClassification::LossOnSaleOfAssets)?;
        // Non-cash gains reduce cash so subtract; non-cash losses add back.
        let sale_adjustment = -gain_on_sale + loss_on_sale;

        let fx_gain = self.income_diff(IncomeClassification::FxGain)?;
        let fx_loss = self.expense_diff(ExpenseClassification::FxLoss)?;
        let fx_adjustment = -fx_gain + fx_loss;

        let vat_income = self.income_diff(IncomeClassification::VatAdjustmentIncome)?;
        let vat_expense = self.expense_diff(ExpenseClassification::VatAdjustmentExpense)?;
        let vat_adjustment = -vat_income + vat_expense;

        // Changes in Working Capital.
        // For asset accounts, an increase uses cash (so we subtract the change);
        // for liabilities, an increase provides cash (so we add the change).
        let ar = self.asset_diff(AssetClassification::AccountsReceivable)?;
        let inventory = self.asset_diff(AssetClassification::Inventory)?;
        let prepaid = self.asset_diff(AssetClassification::PrepaidExpenses)?;
        let other_current_assets = self.asset_diff(AssetClassification::OtherCurrentAssets)?;
        let wc_assets = -(ar + inventory + prepaid + other_current_assets);

        let accounts_payable = self.liability_diff(LiabilityClassification::AccountsPayable)?;
        let accrued_expenses = self.liability_diff(LiabilityClassification::AccruedExpenses)?;
        let deferred_revenue = self.liability_diff(LiabilityClassification::DeferredRevenue)?;
        let other_current_liabilities =
            self.liability_diff(LiabilityClassification::OtherCurrentLiabilities)?;
        let wc_liabilities =
            accounts_payable + accrued_expenses + deferred_revenue + other_current_liabilities;

        let working_capital = wc_assets + wc_liabilities;

        // Net Cash Provided by Operating Activities.
        let operating_net = net_income
            + depreciation
            + amortization
            + sale_adjustment
            + fx_adjustment
            + vat_adjustment
            + working_capital;

        // -------------------------------------
        // INVESTING ACTIVITIES
        // -------------------------------------

        // Cash Outflows for Acquisitions of Long-Term Assets.
        // Sum of purchases for PPE, intangible assets, long-term investments/deposits, and other non-current assets.
        let ppe = self.asset_diff(AssetClassification::PropertyPlantEquipment)?;
        let intangible = self.asset_diff(AssetClassification::IntangibleAssets)?;
        let lt_investments = self.asset_diff(AssetClassification::LongTermInvestments)?;
        let lt_deposits = self.asset_diff(AssetClassification::LongTermDeposits)?;
        let other_noncurrent = self.asset_diff(AssetClassification::OtherNonCurrentAssets)?;
        let asset_acquisitions = ppe + intangible + lt_investments + lt_deposits + other_noncurrent;
        // Cash outflow (a purchase uses cash) is recorded as negative.
        let investing_outflow = -asset_acquisitions;

        // Cash Inflows from Disposals of Long-Term Assets.
        let investing_inflow = 0.0; // Can handle in future.

        let investing_net = investing_inflow + investing_outflow;

        // -------------------------------------
        // FINANCING ACTIVITIES
        // -------------------------------------

        // Cash Flows from Debt Transactions.
        let short_term_debt = self.liability_diff(LiabilityClassification::ShortTermDebt)?;
        let long_term_debt = self.liability_diff(LiabilityClassification::LongTermDebt)?;
        let debt_flow = short_term_debt + long_term_debt;

        // Cash Flows from Equity Transactions.
        let common_stock = self.equity_diff(EquityClassification::CommonStock)?;
        let preferred_stock = self.equity_diff(EquityClassification::PreferredStock)?;
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
        output.push_str(&format!("       Depreciation: {:.2}\n", depreciation));
        output.push_str(&format!("       Amortization: {:.2}\n", amortization));
        output.push_str(&format!(
            "  3. Adjustments for Non-Operating (Non-Cash) Gains/Losses:\n"
        ));
        output.push_str(&format!(
            "       Gain/Loss on Sale of Assets: {:.2}\n",
            sale_adjustment
        ));
        output.push_str(&format!(
            "       Foreign Exchange Adjustments: {:.2}\n",
            fx_adjustment
        ));
        output.push_str(&format!("       VAT Adjustments: {:.2}\n", vat_adjustment));
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
        let output = Command::new("hledger")
            .arg("-f")
            .arg(&self.ledger_path)
            .arg("-p")
            .arg(&self.period)
            .arg("balance")
            .arg(account)
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
        self.run_hledger_sum("Income")
    }

    fn total_expenses(&self) -> Result<f64, ServerError> {
        self.run_hledger_sum("Expenses")
    }

    fn expense_diff(&self, classification: ExpenseClassification) -> Result<f64, ServerError> {
        self.run_hledger_sum(&Into::<Account>::into(expense_tl(classification)).ledger())
    }

    fn income_diff(&self, classification: IncomeClassification) -> Result<f64, ServerError> {
        self.run_hledger_sum(&Into::<Account>::into(income_tl(classification)).ledger())
    }

    fn asset_diff(&self, classification: AssetClassification) -> Result<f64, ServerError> {
        self.run_hledger_sum(&Into::<Account>::into(asset_tl(classification)).ledger())
    }

    fn liability_diff(&self, classification: LiabilityClassification) -> Result<f64, ServerError> {
        self.run_hledger_sum(&Into::<Account>::into(liability_tl(classification)).ledger())
    }

    fn equity_diff(&self, classification: EquityClassification) -> Result<f64, ServerError> {
        self.run_hledger_sum(&Into::<Account>::into(equity_tl(classification)).ledger())
    }
}
