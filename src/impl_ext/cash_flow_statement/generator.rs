use std::path::Path;
use std::process::Command;

use fractic_server_error::{CriticalError, ServerError};

use crate::errors::{HledgerBalanceInvalidTotal, HledgerCommandFailed, HledgerInvalidPath};

fn run_hledger_sum<P: AsRef<Path>>(
    ledger_path: P,
    account: &str,
    period: &str,
) -> Result<f64, ServerError> {
    let output = Command::new("hledger")
        .arg("-f")
        .arg(ledger_path.as_ref())
        .arg("-p")
        .arg(period)
        .arg("balance")
        .arg(account)
        .arg("--output-format=csv")
        .arg("--layout=bare")
        .output()
        .map_err(|e| {
            HledgerCommandFailed::with_debug(
                &ledger_path.as_ref().to_string_lossy().to_string(),
                &e,
            )
        })?;
    if !output.status.success() {
        return Err(HledgerCommandFailed::with_debug(
            &ledger_path.as_ref().to_string_lossy().to_string(),
            &format!(
                "failed with exit code: {}",
                output.status.code().unwrap_or(-1)
            ),
        ));
    }
    let out_csv = String::from_utf8(output.stdout)
        .map_err(|e| CriticalError::with_debug("failed to parse hledger output as UTF-8", &e))?;

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

pub fn generate_cash_flow_statement<P: AsRef<Path>>(
    ledger_path: P,
    period: &str,
) -> Result<String, ServerError> {
    let canonic_ledger_path = ledger_path.as_ref().canonicalize().map_err(|e| {
        HledgerInvalidPath::with_debug(&ledger_path.as_ref().to_string_lossy().to_string(), &e)
    })?;
    let get_amount = |account: &str| -> Result<f64, ServerError> {
        run_hledger_sum(&canonic_ledger_path, account, period)
    };

    // -------------------------------------
    // OPERATING ACTIVITIES
    // -------------------------------------

    // Net Income: computed as Income minus Expenses from the income statement.
    let income_total = get_amount("Income")?;
    let expense_total = get_amount("Expenses")?;
    let net_income = income_total - expense_total;

    // Adjustments for Non-Cash Items.
    let depreciation = get_amount("Expenses:Operating:Depreciation")?;
    let amortization = get_amount("Expenses:Operating:Amortization")?;

    // Adjustments for Non-Operating (Non-Cash) Gains/Losses.
    let gain_on_sale = get_amount("Income:NonOperating:GainOnSaleOfAssets")?;
    let loss_on_sale = get_amount("Expenses:NonOperating:LossOnSaleOfAssets")?;
    // Non-cash gains reduce cash so subtract; non-cash losses add back.
    let sale_adjustment = -gain_on_sale + loss_on_sale;

    let fx_gain = get_amount("Income:NonOperating:FxGain")?;
    let fx_loss = get_amount("Expenses:NonOperating:FxLoss")?;
    let fx_adjustment = -fx_gain + fx_loss;

    let vat_income = get_amount("Income:NonOperating:VatAdjustmentIncome")?;
    let vat_expense = get_amount("Expenses:NonOperating:VatAdjustmentExpense")?;
    let vat_adjustment = -vat_income + vat_expense;

    // Changes in Working Capital.
    // For asset accounts, an increase uses cash (so we subtract the change);
    // for liabilities, an increase provides cash (so we add the change).
    let ar = get_amount("Assets:Current:AccountsReceivable")?;
    let inventory = get_amount("Assets:Current:Inventory")?;
    let prepaid = get_amount("Assets:Current:PrepaidExpenses")?;
    let other_current_assets = get_amount("Assets:Current:OtherCurrentAssets")?;
    let wc_assets = -(ar + inventory + prepaid + other_current_assets);

    let accounts_payable = get_amount("Liabilities:Current:AccountsPayable")?;
    let accrued_expenses = get_amount("Liabilities:Current:AccruedExpenses")?;
    let deferred_revenue = get_amount("Liabilities:Current:DeferredRevenue")?;
    let other_current_liabilities = get_amount("Liabilities:Current:OtherCurrentLiabilities")?;
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
    let ppe = get_amount("Assets:NonCurrent:PropertyPlantEquipment")?;
    let intangible = get_amount("Assets:NonCurrent:IntangibleAssets")?;
    let lt_investments = get_amount("Assets:NonCurrent:LongTermInvestments")?;
    let lt_deposits = get_amount("Assets:NonCurrent:LongTermDeposits")?;
    let other_noncurrent = get_amount("Assets:NonCurrent:OtherNonCurrentAssets")?;
    let asset_acquisitions = ppe + intangible + lt_investments + lt_deposits + other_noncurrent;
    // Cash outflow (a purchase uses cash) is recorded as negative.
    let investing_outflow = -asset_acquisitions;

    // Cash Inflows from Disposals of Long-Term Assets.
    // (Assuming disposals are recorded under this ledger; if not present, defaults to 0.0)
    let investing_inflow = get_amount("Assets:NonCurrent:Disposals")?;

    let investing_net = investing_inflow + investing_outflow;

    // -------------------------------------
    // FINANCING ACTIVITIES
    // -------------------------------------

    // Cash Flows from Debt Transactions.
    let short_term_debt = get_amount("Liabilities:Current:ShortTermDebt")?;
    let long_term_debt = get_amount("Liabilities:NonCurrent:LongTermDebt")?;
    let debt_flow = short_term_debt + long_term_debt;

    // Cash Flows from Equity Transactions.
    let common_stock = get_amount("Equity:ContributedCapital:CommonStock")?;
    let preferred_stock = get_amount("Equity:ContributedCapital:PreferredStock")?;
    let equity_flow = common_stock + preferred_stock;

    // Other Financing Activities are not explicitly tracked (assumed zero).
    let other_financing = 0.0;
    let financing_net = debt_flow + equity_flow + other_financing;

    // -------------------------------------
    // OUTPUT THE STATEMENT OF CASH FLOWS
    // -------------------------------------

    let mut output = String::new();
    output.push_str(&format!("Statement of Cash Flows (Period: {})\n", period));
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
