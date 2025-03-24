use std::collections::HashMap;
use std::iter::zip;
use std::path::{Path, PathBuf};

use fractic_server_error::ServerError;
use iso_currency::Currency;

use crate::entities::{
    asset_tl, liability_tl, Account, AssetClassification, CashflowTracingTag,
    LiabilityClassification,
};
use crate::errors::{HledgerInvalidPath, InvalidIsoCurrencyCode};
use crate::presentation::utils::format_amount;

use super::utils::{
    hledger, hledger_register, replace_all_placeholders_in_string, split_sections, Query,
    RegisterOutput, RegisterQuery, Return,
};

pub struct CashFlowStatementGenerator {
    ledger_path: PathBuf,
    period: String,
    currency: Currency,
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
    pub fn new<P: AsRef<Path>>(
        ledger_path: P,
        period: &str,
        currency: impl IntoCurrency,
    ) -> Result<Self, ServerError> {
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
            currency: currency.try_into()?,
        })
    }

    pub fn generate(self) -> Result<String, ServerError> {
        // -------------------------------------
        // OPERATING ACTIVITIES
        // -------------------------------------

        let net_income = self.net_income()?;

        // Adjustments for non-cash items.
        //
        let nce_depreciation =
            self.expense_by_tag(CashflowTracingTag::NonCashExpenseDepreciation)?;
        let nce_amortization =
            self.expense_by_tag(CashflowTracingTag::NonCashExpenseAmortization)?;
        let nce_other = self.expense_by_tag(CashflowTracingTag::NonCashExpenseOther)?;

        // Changes in working capital.
        //
        let diff_accounts_receivable =
            self.change_in_asset(AssetClassification::AccountsReceivable)?;
        let diff_inventory = self.change_in_asset(AssetClassification::Inventory)?;
        let diff_prepaid_expenses = self.change_in_asset(AssetClassification::PrepaidExpenses)?;
        let diff_other_current_assets = self
            .change_in_asset(AssetClassification::ShortTermInvestments)?
            + self.change_in_asset(AssetClassification::ShortTermDeposits)?
            + self.change_in_asset(AssetClassification::OtherCurrentAssets)?;
        //
        let diff_accounts_payable =
            self.change_in_liability(LiabilityClassification::AccountsPayable)?;
        let diff_accrued_expenses =
            self.change_in_liability(LiabilityClassification::AccruedExpenses)?;
        let diff_deferred_revenue =
            self.change_in_liability(LiabilityClassification::DeferredRevenue)?;
        let diff_other_current_liabilities = self
            .change_in_liability(LiabilityClassification::ShortTermDebt)?
            + self.change_in_liability(LiabilityClassification::OtherCurrentLiabilities)?;

        // Cash flows included in investing or financing activities.
        //
        let gain_loss_sale_assets =
            self.income_by_tag(CashflowTracingTag::ReclassifyGainLossOnSaleOfAssets)?;

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

        let out_ppe = self.cash_outflow_by_tag(CashflowTracingTag::CashOutflowPpe)?;
        let out_intangible_assets =
            self.cash_outflow_by_tag(CashflowTracingTag::CashOutflowIntangibleAssets)?;
        let out_investment_securities =
            self.cash_outflow_by_tag(CashflowTracingTag::CashOutflowInvestmentSecurities)?;
        let out_long_term_deposits =
            self.cash_outflow_by_tag(CashflowTracingTag::CashOutflowLongTermDeposits)?;
        let out_other_investing =
            self.cash_outflow_by_tag(CashflowTracingTag::CashOutflowOtherInvesting)?;

        let in_ppe = self.cash_inflow_by_tag(CashflowTracingTag::CashInflowPpe)?;
        let in_intangible_assets =
            self.cash_inflow_by_tag(CashflowTracingTag::CashInflowIntangibleAssets)?;
        let in_investment_securities =
            self.cash_inflow_by_tag(CashflowTracingTag::CashInflowInvestmentSecurities)?;
        let in_long_term_deposits =
            self.cash_inflow_by_tag(CashflowTracingTag::CashInflowLongTermDeposits)?;
        let in_other_investing =
            self.cash_inflow_by_tag(CashflowTracingTag::CashInflowOtherInvesting)?;

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
        let in_borrowings = self.cash_inflow_by_tag(CashflowTracingTag::CashInflowBorrowings)?;
        let out_borrowings = self.cash_outflow_by_tag(CashflowTracingTag::CashOutflowBorrowings)?;

        // Equity-related cash flows.
        //
        let in_issuance_shares =
            self.cash_inflow_by_tag(CashflowTracingTag::CashInflowIssuanceShares)?;
        let out_share_buybacks =
            self.cash_outflow_by_tag(CashflowTracingTag::CashOutflowShareBuybacks)?;
        let out_dividends = self.cash_outflow_by_tag(CashflowTracingTag::CashOutflowDividends)?;

        // Other financing activities.
        //
        let in_out_other_financing =
            self.cash_inflow_by_tag(CashflowTracingTag::CashInOutflowOtherFinancing)?;

        let net_financing = in_borrowings - out_borrowings + in_issuance_shares
            - out_share_buybacks
            - out_dividends
            + in_out_other_financing;

        // -------------------------------------
        // RECONCILIATION
        // -------------------------------------

        let balance_opening = self.period_start_balance()?;
        let balance_change = net_operating + net_investing + net_financing;
        let balance_before_exchange = balance_opening + balance_change;
        let exchange_rate_effects = 0.0;
        let balance_closing = balance_before_exchange + exchange_rate_effects;

        // -------------------------------------
        // ADDITIONAL DISCLOSURES
        // -------------------------------------

        let non_cash_reclassifications = self
            .non_cash_reclassifications()?
            .into_iter()
            .map(|s| format!("• {}", s))
            .collect::<Vec<String>>()
            .join("\n");

        // -------------------------------------
        // FILL TEMPLATE
        // -------------------------------------

        let placeholders: HashMap<String, String> = vec![
            ("period", self.period.clone()),
            ("net_income", self.fmt(net_income)),
            ("nce_depreciation", self.fmt(nce_depreciation)),
            ("nce_amortization", self.fmt(nce_amortization)),
            ("nce_other", self.fmt(nce_other)),
            (
                "diff_accounts_receivable",
                self.fmt(diff_accounts_receivable),
            ),
            ("diff_inventory", self.fmt(diff_inventory)),
            ("diff_prepaid_expenses", self.fmt(diff_prepaid_expenses)),
            (
                "diff_other_current_assets",
                self.fmt(diff_other_current_assets),
            ),
            ("diff_accounts_payable", self.fmt(diff_accounts_payable)),
            ("diff_accrued_expenses", self.fmt(diff_accrued_expenses)),
            ("diff_deferred_revenue", self.fmt(diff_deferred_revenue)),
            (
                "diff_other_current_liabilities",
                self.fmt(diff_other_current_liabilities),
            ),
            ("gain_loss_sale_assets", self.fmt(gain_loss_sale_assets)),
            ("net_operating", self.fmt(net_operating)),
            ("out_ppe", self.fmt(out_ppe)),
            ("out_intangible_assets", self.fmt(out_intangible_assets)),
            (
                "out_investment_securities",
                self.fmt(out_investment_securities),
            ),
            ("out_long_term_deposits", self.fmt(out_long_term_deposits)),
            ("out_other_investing", self.fmt(out_other_investing)),
            ("in_ppe", self.fmt(in_ppe)),
            ("in_intangible_assets", self.fmt(in_intangible_assets)),
            (
                "in_investment_securities",
                self.fmt(in_investment_securities),
            ),
            ("in_long_term_deposits", self.fmt(in_long_term_deposits)),
            ("in_other_investing", self.fmt(in_other_investing)),
            ("net_investing", self.fmt(net_investing)),
            ("in_borrowings", self.fmt(in_borrowings)),
            ("out_borrowings", self.fmt(out_borrowings)),
            ("in_issuance_shares", self.fmt(in_issuance_shares)),
            ("out_share_buybacks", self.fmt(out_share_buybacks)),
            ("out_dividends", self.fmt(out_dividends)),
            ("in_out_other_financing", self.fmt(in_out_other_financing)),
            ("net_financing", self.fmt(net_financing)),
            ("balance_opening", self.fmt(balance_opening)),
            ("balance_change", self.fmt(balance_change)),
            ("balance_before_exchange", self.fmt(balance_before_exchange)),
            ("exchange_rate_effects", self.fmt(exchange_rate_effects)),
            ("balance_closing", self.fmt(balance_closing)),
            (
                "non_cash_reclassifications",
                non_cash_reclassifications.to_string(),
            ),
        ]
        .into_iter()
        .map(|(k, v)| (k.to_string(), v))
        .collect();

        let bytes = include_bytes!("../../../res/cash_flow_statement_template.txt");
        let template = String::from_utf8_lossy(bytes).to_string();
        replace_all_placeholders_in_string(template, &placeholders, true)
    }

    fn fmt(&self, amount: f64) -> String {
        format!("{:>22}", format_amount(amount, self.currency, false))
    }

    fn net_income(&self) -> Result<f64, ServerError> {
        hledger(
            &self.ledger_path,
            &self.period,
            Query::IncomeStatement,
            None,
            Return::Total,
        )
    }

    fn expense_by_tag(&self, tag: CashflowTracingTag) -> Result<f64, ServerError> {
        hledger(
            &self.ledger_path,
            &self.period,
            Query::ChangeInAccount {
                account: tag.value(),
            },
            Some(CashflowTracingTag::key()),
            Return::Total,
        )
    }

    fn income_by_tag(&self, tag: CashflowTracingTag) -> Result<f64, ServerError> {
        Ok(-self.expense_by_tag(tag)?)
    }

    fn change_in_asset(&self, classification: AssetClassification) -> Result<f64, ServerError> {
        hledger(
            &self.ledger_path,
            &self.period,
            Query::ChangeInAccount {
                account: Into::<Account>::into(asset_tl(classification)).ledger(),
            },
            None,
            Return::Total,
        )
    }

    fn change_in_liability(
        &self,
        classification: LiabilityClassification,
    ) -> Result<f64, ServerError> {
        Ok(-hledger(
            &self.ledger_path,
            &self.period,
            Query::ChangeInAccount {
                account: Into::<Account>::into(liability_tl(classification)).ledger(),
            },
            None,
            Return::Total,
        )?)
    }

    fn cash_outflow_by_tag(&self, tag: CashflowTracingTag) -> Result<f64, ServerError> {
        let cash_backed = hledger(
            &self.ledger_path,
            &self.period,
            Query::ChangeInAccountReverse {
                account: Into::<Account>::into(asset_tl(
                    AssetClassification::CashAndCashEquivalents,
                ))
                .ledger(),
            },
            Some(CashflowTracingTag::key()),
            Return::SearchRowOrZero(tag.value()),
        )?;
        let non_cash_reclassifications = hledger(
            &self.ledger_path,
            &self.period,
            Query::ChangeByTag {
                key: "s",
                value: "non_cash_reclassification",
            },
            Some(CashflowTracingTag::key()),
            Return::SearchRowOrZero(tag.value()),
        )?;
        Ok(cash_backed + non_cash_reclassifications)
    }

    fn cash_inflow_by_tag(&self, tag: CashflowTracingTag) -> Result<f64, ServerError> {
        Ok(-self.cash_outflow_by_tag(tag)?)
    }

    fn period_start_balance(&self) -> Result<f64, ServerError> {
        let period_end_balance = hledger(
            &self.ledger_path,
            &self.period,
            Query::CumulativeBalance {
                account: Into::<Account>::into(asset_tl(
                    AssetClassification::CashAndCashEquivalents,
                ))
                .ledger(),
            },
            None,
            Return::Total,
        )?;
        let period_change = hledger(
            &self.ledger_path,
            &self.period,
            Query::ChangeInAccount {
                account: Into::<Account>::into(asset_tl(
                    AssetClassification::CashAndCashEquivalents,
                ))
                .ledger(),
            },
            None,
            Return::Total,
        )?;
        Ok(period_end_balance - period_change)
    }

    fn non_cash_reclassifications(&self) -> Result<Vec<String>, ServerError> {
        let source = hledger_register(
            &self.ledger_path,
            &self.period,
            RegisterQuery::TagReverse {
                key: "s",
                value: "non_cash_reclassification",
            },
            None,
            RegisterOutput::Raw { width: 200 },
        )?;
        let dest = hledger_register(
            &self.ledger_path,
            &self.period,
            RegisterQuery::Tag {
                key: "s",
                value: "non_cash_reclassification",
            },
            None,
            RegisterOutput::Raw { width: 200 },
        )?;
        zip(source.into_iter(), dest.into_iter())
            .map(|(s, d)| {
                let s_sec = split_sections(&s);
                let d_sec = split_sections(&d);
                Ok(format!(
                    "{}\n    {:78} {:>17}\n    {:78} {:>17}",
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
