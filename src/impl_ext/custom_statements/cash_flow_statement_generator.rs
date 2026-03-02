use std::collections::HashMap;
use std::iter::zip;
use std::path::{Path, PathBuf};

use fractic_server_error::ServerError;
use iso_currency::Currency;

use crate::entities::{
    asset_tl, liability_tl, Account, AssetClassification, CashflowTracingTag,
    LiabilityClassification,
};
use crate::errors::{HledgerInvalidPath, InvalidCashFlowStatementPeriods, InvalidIsoCurrencyCode};
use crate::presentation::utils::format_amount;

use super::utils::{
    hledger, hledger_register, replace_all_placeholders_in_string, split_sections, Query,
    RegisterOutput, RegisterQuery, Return,
};

pub struct CashFlowStatementGenerator {
    ledger_path: PathBuf,
    periods: Vec<String>,
    currency: Currency,
}

struct PeriodReport {
    period: String,
    amounts: HashMap<&'static str, f64>,
    non_cash_reclassifications: Vec<String>,
}

const START_INDEX: usize = 64; // (65th char)
const COL_PADDING_LEFT: usize = 5;
const COL_PADDING_RIGHT: usize = 2;
const COL_SEPARATOR_CHAR: char = '│';
const HR_CHAR: char = '─';
const HR_INTERSECTION_CHAR: char = '┼';

/// Row-ranges of the statement that should have lines extended to include the
/// '|' period-separating borders, even if those lines themselves don't contain
/// any placeholders.
const TABLE_SCAFFOLD_RANGES: &[(usize, usize)] = &[(0, 50), (53, 60)]; // 0-based inclusive

const PLACEHOLDER_KEYS: [&str; 37] = [
    "net_income",
    "nce_depreciation",
    "nce_amortization",
    "nce_other",
    "diff_accounts_receivable",
    "diff_inventory",
    "diff_prepaid_expenses",
    "diff_other_current_assets",
    "diff_accounts_payable",
    "diff_accrued_expenses",
    "diff_deferred_revenue",
    "diff_other_current_liabilities",
    "gain_loss_sale_assets",
    "net_operating",
    "out_ppe",
    "out_intangible_assets",
    "out_investment_securities",
    "out_long_term_deposits",
    "out_other_investing",
    "in_ppe",
    "in_intangible_assets",
    "in_investment_securities",
    "in_long_term_deposits",
    "in_other_investing",
    "net_investing",
    "in_borrowings",
    "out_borrowings",
    "net_issuance_shares",
    "out_share_buybacks",
    "out_dividends",
    "in_out_other_financing",
    "net_financing",
    "balance_opening",
    "balance_change",
    "balance_before_exchange",
    "exchange_rate_effects",
    "balance_closing",
];

struct ReportLayout {
    /// Number of columns.
    period_count: usize,
    /// Unpadded column width -- uniformly applied to all columns.
    column_width: usize,
}

impl ReportLayout {
    fn column_left_padding(&self, _column_idx: usize) -> usize {
        COL_PADDING_LEFT
    }

    fn column_right_padding(&self, column_idx: usize) -> usize {
        if column_idx + 1 < self.period_count {
            COL_PADDING_RIGHT
        } else {
            0
        }
    }

    fn rendered_columns_width(&self) -> usize {
        if self.period_count == 0 {
            return 0;
        }
        let mut width = 0;
        for column_idx in 0..self.period_count {
            width += self.column_left_padding(column_idx) + self.column_width;
            if column_idx + 1 < self.period_count {
                width += self.column_right_padding(column_idx) + 1; // separator char
            }
        }
        width
    }

    fn border_positions(&self) -> Vec<usize> {
        let mut positions = Vec::with_capacity(self.period_count.saturating_sub(1));
        let mut cursor = 0;
        for column_idx in 0..self.period_count {
            cursor += self.column_left_padding(column_idx) + self.column_width;
            if column_idx + 1 < self.period_count {
                cursor += self.column_right_padding(column_idx);
                positions.push(cursor);
                cursor += 1; // separator char
            }
        }
        positions
    }

    fn scaffold_end(&self) -> usize {
        self.border_positions().last().map_or(0, |pos| pos + 1)
    }
}

pub trait IntoCurrency {
    fn try_into(self) -> Result<Currency, ServerError>;
}
impl IntoCurrency for &str {
    fn try_into(self) -> Result<Currency, ServerError> {
        Currency::from_code(self).ok_or_else(|| InvalidIsoCurrencyCode::new(self))
    }
}
impl IntoCurrency for String {
    fn try_into(self) -> Result<Currency, ServerError> {
        Currency::from_code(&self).ok_or_else(|| InvalidIsoCurrencyCode::new(&self))
    }
}
impl IntoCurrency for Currency {
    fn try_into(self) -> Result<Currency, ServerError> {
        Ok(self)
    }
}

impl CashFlowStatementGenerator {
    pub fn new<P, I, S>(
        ledger_path: P,
        periods: I,
        currency: impl IntoCurrency,
    ) -> Result<Self, ServerError>
    where
        P: AsRef<Path>,
        I: IntoIterator<Item = S>,
        S: AsRef<str>,
    {
        let periods: Vec<String> = periods
            .into_iter()
            .map(|p| p.as_ref().to_string())
            .collect();
        if periods.is_empty() {
            return Err(InvalidCashFlowStatementPeriods::new());
        }
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
            periods,
            currency: currency.try_into()?,
        })
    }

    pub fn generate(self) -> Result<String, ServerError> {
        let reports = self
            .periods
            .iter()
            .map(|period| self.generate_period_report(period))
            .collect::<Result<Vec<PeriodReport>, ServerError>>()?;
        let template_bytes = include_bytes!("../../../res/cash_flow_statement_template.txt");
        let template = String::from_utf8_lossy(template_bytes).to_string();
        let layout = self.build_report_layout(&reports);
        let placeholder_map = self.build_placeholder_map(&reports, &layout);
        let filled = replace_all_placeholders_in_string(template, &placeholder_map, true)?;
        Ok(self.extend_column_separators_in_section(filled, &layout))
    }

    fn generate_period_report(&self, period: &str) -> Result<PeriodReport, ServerError> {
        // -------------------------------------
        // OPERATING ACTIVITIES
        // -------------------------------------

        let net_income = self.net_income(period)?;

        // Adjustments for non-cash items.
        //
        let nce_depreciation =
            self.expense_by_tag(period, CashflowTracingTag::NonCashExpenseDepreciation)?;
        let nce_amortization =
            self.expense_by_tag(period, CashflowTracingTag::NonCashExpenseAmortization)?;
        let nce_other = self.expense_by_tag(period, CashflowTracingTag::NonCashExpenseOther)?
            - self.expenses_paid_with_non_cash_payment(period)?;

        // Changes in working capital.
        //
        let diff_accounts_receivable =
            self.change_in_asset(period, AssetClassification::AccountsReceivable)?;
        let diff_inventory = self.change_in_asset(period, AssetClassification::Inventory)?;
        let diff_prepaid_expenses =
            self.change_in_asset(period, AssetClassification::PrepaidExpenses)?;
        let diff_other_current_assets = self
            .change_in_asset(period, AssetClassification::ShortTermInvestments)?
            + self.change_in_asset(period, AssetClassification::ShortTermDeposits)?
            + self.change_in_asset(period, AssetClassification::OtherCurrentAssets)?;
        //
        let diff_accounts_payable =
            self.change_in_liability(period, LiabilityClassification::AccountsPayable)?;
        let diff_accrued_expenses =
            self.change_in_liability(period, LiabilityClassification::AccruedExpenses)?;
        let diff_deferred_revenue =
            self.change_in_liability(period, LiabilityClassification::DeferredRevenue)?;
        let diff_other_current_liabilities = self
            .change_in_liability(period, LiabilityClassification::ShortTermDebt)?
            + self.change_in_liability(period, LiabilityClassification::OtherCurrentLiabilities)?;

        // Cash flows included in investing or financing activities.
        //
        let gain_loss_sale_assets =
            self.income_by_tag(period, CashflowTracingTag::ReclassifyGainLossOnSaleOfAssets)?;

        // Net cash provided by operating activities.
        //
        let net_operating = net_income + nce_depreciation + nce_amortization + nce_other
            - diff_accounts_receivable
            - diff_inventory
            - diff_prepaid_expenses
            - diff_other_current_assets
            + diff_accounts_payable
            + diff_accrued_expenses
            + diff_deferred_revenue
            + diff_other_current_liabilities
            - gain_loss_sale_assets;

        // -------------------------------------
        // INVESTING ACTIVITIES
        // -------------------------------------

        let out_ppe = self.cash_outflow_by_tag(period, CashflowTracingTag::CashOutflowPpe)?;
        let out_intangible_assets =
            self.cash_outflow_by_tag(period, CashflowTracingTag::CashOutflowIntangibleAssets)?;
        let out_investment_securities =
            self.cash_outflow_by_tag(period, CashflowTracingTag::CashOutflowInvestmentSecurities)?;
        let out_long_term_deposits =
            self.cash_outflow_by_tag(period, CashflowTracingTag::CashOutflowLongTermDeposits)?;
        let out_other_investing =
            self.cash_outflow_by_tag(period, CashflowTracingTag::CashOutflowOtherInvesting)?;

        let in_ppe = self.cash_inflow_by_tag(period, CashflowTracingTag::CashInflowPpe)?;
        let in_intangible_assets =
            self.cash_inflow_by_tag(period, CashflowTracingTag::CashInflowIntangibleAssets)?;
        let in_investment_securities =
            self.cash_inflow_by_tag(period, CashflowTracingTag::CashInflowInvestmentSecurities)?;
        let in_long_term_deposits =
            self.cash_inflow_by_tag(period, CashflowTracingTag::CashInflowLongTermDeposits)?;
        let in_other_investing =
            self.cash_inflow_by_tag(period, CashflowTracingTag::CashInflowOtherInvesting)?;

        let net_investing = -out_ppe
            - out_intangible_assets
            - out_investment_securities
            - out_long_term_deposits
            - out_other_investing
            + in_ppe
            + in_intangible_assets
            + in_investment_securities
            + in_long_term_deposits
            + in_other_investing;

        // -------------------------------------
        // FINANCING ACTIVITIES
        // -------------------------------------

        // Debt-related cash flows.
        //
        let in_borrowings =
            self.cash_inflow_by_tag(period, CashflowTracingTag::CashInflowBorrowings)?;
        let out_borrowings =
            self.cash_outflow_by_tag(period, CashflowTracingTag::CashOutflowBorrowings)?;

        // Equity-related cash flows.
        //
        let in_issuance_shares =
            self.cash_inflow_by_tag(period, CashflowTracingTag::CashInflowIssuanceShares)?;
        let out_share_issuance_costs =
            self.cash_outflow_by_tag(period, CashflowTracingTag::CashOutflowShareIssuanceCosts)?;
        let out_share_buybacks =
            self.cash_outflow_by_tag(period, CashflowTracingTag::CashOutflowShareBuybacks)?;
        let out_dividends =
            self.cash_outflow_by_tag(period, CashflowTracingTag::CashOutflowDividends)?;

        // Other financing activities.
        //
        let in_out_other_financing =
            self.cash_inflow_by_tag(period, CashflowTracingTag::CashInOutflowOtherFinancing)?;

        let net_issuance_shares = in_issuance_shares - out_share_issuance_costs;
        let net_financing = in_borrowings - out_borrowings + net_issuance_shares
            - out_share_buybacks
            - out_dividends
            + in_out_other_financing;

        // -------------------------------------
        // RECONCILIATION
        // -------------------------------------

        let balance_opening = self.period_start_balance(period)?;
        let balance_change = net_operating + net_investing + net_financing;
        let balance_before_exchange = balance_opening + balance_change;
        let exchange_rate_effects = 0.0;
        let balance_closing = balance_before_exchange + exchange_rate_effects;

        // -------------------------------------
        // ADDITIONAL DISCLOSURES
        // -------------------------------------

        let non_cash_reclassifications = self.non_cash_reclassifications(period)?;

        // -------------------------------------
        // BUILD REPORT
        // -------------------------------------

        Ok(PeriodReport {
            period: period.to_string(),
            amounts: vec![
                ("net_income", net_income),
                ("nce_depreciation", nce_depreciation),
                ("nce_amortization", nce_amortization),
                ("nce_other", nce_other),
                ("diff_accounts_receivable", diff_accounts_receivable),
                ("diff_inventory", diff_inventory),
                ("diff_prepaid_expenses", diff_prepaid_expenses),
                ("diff_other_current_assets", diff_other_current_assets),
                ("diff_accounts_payable", diff_accounts_payable),
                ("diff_accrued_expenses", diff_accrued_expenses),
                ("diff_deferred_revenue", diff_deferred_revenue),
                (
                    "diff_other_current_liabilities",
                    diff_other_current_liabilities,
                ),
                ("gain_loss_sale_assets", gain_loss_sale_assets),
                ("net_operating", net_operating),
                ("out_ppe", out_ppe),
                ("out_intangible_assets", out_intangible_assets),
                ("out_investment_securities", out_investment_securities),
                ("out_long_term_deposits", out_long_term_deposits),
                ("out_other_investing", out_other_investing),
                ("in_ppe", in_ppe),
                ("in_intangible_assets", in_intangible_assets),
                ("in_investment_securities", in_investment_securities),
                ("in_long_term_deposits", in_long_term_deposits),
                ("in_other_investing", in_other_investing),
                ("net_investing", net_investing),
                ("in_borrowings", in_borrowings),
                ("out_borrowings", out_borrowings),
                ("net_issuance_shares", net_issuance_shares),
                ("out_share_buybacks", out_share_buybacks),
                ("out_dividends", out_dividends),
                ("in_out_other_financing", in_out_other_financing),
                ("net_financing", net_financing),
                ("balance_opening", balance_opening),
                ("balance_change", balance_change),
                ("balance_before_exchange", balance_before_exchange),
                ("exchange_rate_effects", exchange_rate_effects),
                ("balance_closing", balance_closing),
            ]
            .into_iter()
            .collect(),
            non_cash_reclassifications,
        })
    }

    fn build_report_layout(&self, reports: &[PeriodReport]) -> ReportLayout {
        let max_amount_width = reports
            .iter()
            .flat_map(|report| PLACEHOLDER_KEYS.iter().map(move |key| (report, key)))
            .map(|(report, key)| {
                let value = report
                    .amounts
                    .get(key)
                    .expect("placeholder key missing in report");
                format_amount(*value, self.currency, false).chars().count()
            })
            .max()
            .unwrap_or(0);
        let max_period_width = reports
            .iter()
            .map(|report| report.period.chars().count())
            .max()
            .unwrap_or(0);

        ReportLayout {
            period_count: reports.len(),
            column_width: max_amount_width.max(max_period_width),
        }
    }

    fn build_placeholder_map(
        &self,
        reports: &[PeriodReport],
        layout: &ReportLayout,
    ) -> HashMap<String, String> {
        let mut placeholders = HashMap::new();
        let period_columns = reports
            .iter()
            .map(|report| report.period.clone())
            .collect::<Vec<String>>();
        placeholders.insert(
            "period".to_string(),
            self.format_columns(&period_columns, layout),
        );
        placeholders.insert(
            "hr_ext".to_string(),
            HR_CHAR.to_string().repeat(layout.rendered_columns_width()),
        );
        for key in PLACEHOLDER_KEYS {
            let values = reports
                .iter()
                .map(|report| {
                    *report
                        .amounts
                        .get(key)
                        .expect("placeholder key missing in report")
                })
                .collect::<Vec<f64>>();
            let rendered_values = values
                .iter()
                .map(|amount| format_amount(*amount, self.currency, false))
                .collect::<Vec<String>>();
            placeholders.insert(
                key.to_string(),
                self.format_columns(&rendered_values, layout),
            );
        }
        placeholders.insert(
            "non_cash_reclassifications".to_string(),
            reports
                .iter()
                // Only report non-cash reclassifications for the latest period
                // (i.e. the current reporting year).
                .max_by_key(|r| &r.period)
                .into_iter()
                .flat_map(|r| r.non_cash_reclassifications.iter())
                .map(|s| format!("• {}", s))
                .collect::<Vec<String>>()
                .join("\n"),
        );
        placeholders
    }

    fn format_columns(&self, values: &[String], layout: &ReportLayout) -> String {
        let mut rendered = String::new();
        for (idx, value) in values.iter().enumerate() {
            rendered.push_str(&" ".repeat(layout.column_left_padding(idx)));
            rendered.push_str(&format!("{:>width$}", value, width = layout.column_width));
            if idx + 1 < values.len() {
                rendered.push_str(&" ".repeat(layout.column_right_padding(idx)));
                rendered.push(COL_SEPARATOR_CHAR);
            }
        }
        rendered
    }

    fn column_border_scaffold(&self, layout: &ReportLayout) -> Vec<char> {
        let target_len = START_INDEX + layout.scaffold_end();
        let mut scaffold = vec![' '; target_len];
        for rel_pos in layout.border_positions() {
            let border_idx = START_INDEX + rel_pos;
            if border_idx < scaffold.len() {
                scaffold[border_idx] = COL_SEPARATOR_CHAR;
            }
        }
        scaffold
    }

    fn extend_column_separators_in_section(
        &self,
        rendered: String,
        layout: &ReportLayout,
    ) -> String {
        let mut lines = rendered
            .split('\n')
            .map(std::string::ToString::to_string)
            .collect::<Vec<String>>();
        let scaffold = self.column_border_scaffold(layout);
        let abs_border_positions = layout
            .border_positions()
            .into_iter()
            .map(|rel| START_INDEX + rel)
            .collect::<Vec<usize>>();
        let target_len = scaffold.len();

        for (start_line_idx, end_line_idx) in TABLE_SCAFFOLD_RANGES {
            for line in lines
                .iter_mut()
                .skip(*start_line_idx)
                .take(end_line_idx.saturating_sub(*start_line_idx) + 1)
            {
                let current_len = line.chars().count();
                if current_len < target_len {
                    let tail = scaffold[current_len..target_len].iter().collect::<String>();
                    line.push_str(&tail);
                }

                let mut chars = line.chars().collect::<Vec<char>>();
                for pos in &abs_border_positions {
                    if *pos < chars.len() && chars[*pos] == HR_CHAR {
                        chars[*pos] = HR_INTERSECTION_CHAR;
                    }
                }
                *line = chars.into_iter().collect();
            }
        }

        lines.join("\n")
    }

    fn net_income(&self, period: &str) -> Result<f64, ServerError> {
        hledger(
            &self.ledger_path,
            period,
            Query::IncomeStatement,
            true,
            None,
            Return::Total,
        )
    }

    fn expenses_paid_with_non_cash_payment(&self, period: &str) -> Result<f64, ServerError> {
        hledger(
            &self.ledger_path,
            period,
            Query::ChangeInAccountReverse {
                account: "Expenses".to_string(),
            },
            true,
            Some(CashflowTracingTag::key()),
            Return::SearchRowOrZero(CashflowTracingTag::NonCashPayment.value()),
        )
    }

    fn expense_by_tag(&self, period: &str, tag: CashflowTracingTag) -> Result<f64, ServerError> {
        hledger(
            &self.ledger_path,
            period,
            Query::ChangeInAccount {
                account: tag.value(),
            },
            true,
            Some(CashflowTracingTag::key()),
            Return::Total,
        )
    }

    fn income_by_tag(&self, period: &str, tag: CashflowTracingTag) -> Result<f64, ServerError> {
        Ok(-self.expense_by_tag(period, tag)?)
    }

    fn change_in_asset(
        &self,
        period: &str,
        classification: AssetClassification,
    ) -> Result<f64, ServerError> {
        hledger(
            &self.ledger_path,
            period,
            Query::ChangeInAccount {
                account: Into::<Account>::into(asset_tl(classification)).ledger(),
            },
            true,
            None,
            Return::Total,
        )
    }

    fn change_in_liability(
        &self,
        period: &str,
        classification: LiabilityClassification,
    ) -> Result<f64, ServerError> {
        Ok(-hledger(
            &self.ledger_path,
            period,
            Query::ChangeInAccount {
                account: Into::<Account>::into(liability_tl(classification)).ledger(),
            },
            true,
            None,
            Return::Total,
        )?)
    }

    fn cash_outflow_by_tag(
        &self,
        period: &str,
        tag: CashflowTracingTag,
    ) -> Result<f64, ServerError> {
        let cash_backed = hledger(
            &self.ledger_path,
            period,
            Query::ChangeInAccountReverse {
                account: Into::<Account>::into(asset_tl(
                    AssetClassification::CashAndCashEquivalents,
                ))
                .ledger(),
            },
            true,
            Some(CashflowTracingTag::key()),
            Return::SearchRowOrZero(tag.value()),
        )?;
        let non_cash_reclassifications = hledger(
            &self.ledger_path,
            period,
            Query::ChangeByTag {
                key: "s",
                value: "non_cash_reclassification",
            },
            true,
            Some(CashflowTracingTag::key()),
            Return::SearchRowOrZero(tag.value()),
        )?;
        Ok(cash_backed + non_cash_reclassifications)
    }

    fn cash_inflow_by_tag(
        &self,
        period: &str,
        tag: CashflowTracingTag,
    ) -> Result<f64, ServerError> {
        Ok(-self.cash_outflow_by_tag(period, tag)?)
    }

    fn period_start_balance(&self, period: &str) -> Result<f64, ServerError> {
        let period_end_balance = hledger(
            &self.ledger_path,
            period,
            Query::CumulativeBalance {
                account: Into::<Account>::into(asset_tl(
                    AssetClassification::CashAndCashEquivalents,
                ))
                .ledger(),
            },
            true,
            None,
            Return::Total,
        )?;
        let period_change = hledger(
            &self.ledger_path,
            period,
            Query::ChangeInAccount {
                account: Into::<Account>::into(asset_tl(
                    AssetClassification::CashAndCashEquivalents,
                ))
                .ledger(),
            },
            true,
            None,
            Return::Total,
        )?;
        Ok(period_end_balance - period_change)
    }

    fn non_cash_reclassifications(&self, period: &str) -> Result<Vec<String>, ServerError> {
        let source = hledger_register(
            &self.ledger_path,
            period,
            RegisterQuery::TagReverse {
                key: "s",
                value: "non_cash_reclassification",
            },
            true,
            None,
            RegisterOutput::Raw { width: 200 },
        )?;
        let dest = hledger_register(
            &self.ledger_path,
            period,
            RegisterQuery::Tag {
                key: "s",
                value: "non_cash_reclassification",
            },
            true,
            None,
            RegisterOutput::Raw { width: 200 },
        )?;
        zip(source.into_iter(), dest.into_iter())
            .map(|(s, d)| {
                let s_sec = split_sections(&s);
                let d_sec = split_sections(&d);
                Ok(format!(
                    "{}\n    {:57} {:>17}\n    {:57} {:>17}",
                    s_sec.get(0).map_or("", |s| s),
                    s_sec.get(1).map_or("", |s| s),
                    s_sec.get(2).map_or("", |s| s),
                    d_sec.get(1).map_or("", |s| s),
                    d_sec.get(2).map_or("", |s| s)
                ))
            })
            .collect()
    }
}
