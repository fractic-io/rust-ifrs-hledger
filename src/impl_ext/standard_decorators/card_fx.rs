use std::{iter::once, path::PathBuf, str::FromStr as _, vec};

use async_trait::async_trait;
use chrono::NaiveDate;
use fractic_currency_conversion::util::FxUtil;
use fractic_server_error::ServerError;

use crate::{
    data::models::iso_date_model::ISODateModel,
    entities::{
        Annotation, CommodityHandler, DecoratedTransactionSpec, DecoratorLogic, Handlers,
        Transaction, TransactionPosting,
    },
    ext::standard_accounts::{FOREIGN_TRANSACTION_FEE, REALIZED_FX_GAIN, REALIZED_FX_LOSS},
};

#[derive(Debug)]
enum LogicType {
    DelayedSettleUnknownFee {
        settle_date: NaiveDate,
        settle_amount: f64,
    },
    ImmediateWithFee {
        charged: f64,
        fee: f64,
    },
}

#[derive(Debug)]
pub struct StandardDecoratorCardFx {
    logic: LogicType,
    currency_conversion_cache_dir: PathBuf,
    currency_conversion_api_key: String,
}

//
// IMPORTANT NOTE:
//   I think it's important that all these logic types start by recording the
//   initial transaction using the EOD rate from the fractic_currency_conversion
//   library (even if some other information, like the bank exchange rate used,
//   is directly available). This way we maintain comparability between FX
//   transactions, even if the card fee and settlement logic differs.
//
impl StandardDecoratorCardFx {
    /// Use for foreign currency transactions where:
    /// - The transaction is initially authorized, but then settles on a later
    ///   date, at a potentially different final amount.
    /// - The fee charged by the card issuer is either zero or not transparently
    ///   known (i.e. likely baked into their conversion rate).
    ///
    /// On payment date, the amount will be recorded based on the EOD exchange
    /// rate (irrespective of the authorization amount). On settlement date, a
    /// correction transaction will be recorded to account for the discrepancy
    /// between our initial calculation and the actual amount deducted from the
    /// bank account.
    ///
    /// Since the card fees are not transparently known, we just consider them
    /// immaterial and fold them into the FX gain / loss experienced on the
    /// settlement date, which is generally fine under IFRS.
    pub fn delayed_settle_unknown_fee(
        settle_date: &String,
        settle_amount: f64,
        currency_conversion_cache_dir: impl Into<PathBuf>,
        currency_conversion_api_key: impl Into<String>,
    ) -> Result<Self, ServerError> {
        Ok(Self {
            logic: LogicType::DelayedSettleUnknownFee {
                settle_date: ISODateModel::from_str(settle_date)?.into(),
                settle_amount,
            },
            currency_conversion_cache_dir: currency_conversion_cache_dir.into(),
            currency_conversion_api_key: currency_conversion_api_key.into(),
        })
    }

    /// Use for foreign currency transactions where:
    /// - The fee charged by the card issuer is transparent.
    /// - The amount is directly deducted from the bank account (no separate
    ///   settlement date).
    ///
    /// In this case, the fee is recorded as a general administrative expense,
    /// and any remaining discrepancy as FX gain / loss.
    pub fn immediate_with_fee(
        charged: f64,
        fee: f64,
        currency_conversion_cache_dir: impl Into<PathBuf>,
        currency_conversion_api_key: impl Into<String>,
    ) -> Result<Self, ServerError> {
        Ok(Self {
            logic: LogicType::ImmediateWithFee { charged, fee },
            currency_conversion_cache_dir: currency_conversion_cache_dir.into(),
            currency_conversion_api_key: currency_conversion_api_key.into(),
        })
    }

    // --

    async fn apply_delayed_settle_unknown_fee<H: Handlers>(
        &self,
        tx: DecoratedTransactionSpec<H>,
        settle_date: NaiveDate,
        settle_amount: f64,
    ) -> Result<DecoratedTransactionSpec<H>, ServerError> {
        let DecoratedTransactionSpec {
            id,
            accrual_start,
            accrual_end,
            payment_date,
            accounting_logic,
            payee,
            description,
            amount: source_amount,
            commodity: source_commodity,
            backing_account,
            annotations,
            ext_transactions,
            ext_assertions,
        } = tx;

        // IMPORTANT NOTE:
        //   If this transaction is an expense, the amount is negative.

        let main_commodity = H::M::default();

        // Calculate the amount to record on payment date (amount converted at
        // the rate on that day).
        let fx_util = FxUtil::using_open_exchange_rates_api(
            &self.currency_conversion_api_key,
            &self.currency_conversion_cache_dir,
        )?;
        let converted_amount = fx_util
            .convert(
                payment_date,
                source_commodity.iso_symbol(),
                main_commodity.iso_symbol(),
                source_amount,
            )
            .await?;

        // Record an additional correction transaction on the settlement date
        // (difference between our conversion and final settlement amount).
        let fx_discrepancy = if converted_amount > 0.0 {
            // For income, positive discrepancy is a gain.
            settle_amount.abs() - converted_amount.abs()
        } else {
            // For expense, positive discrepancy is a loss.
            converted_amount.abs() - settle_amount.abs()
        };
        let vat_transactions = if fx_discrepancy.abs() >= main_commodity.precision_cutoff()? {
            vec![Transaction {
                spec_id: id.clone(),
                date: settle_date,
                postings: vec![
                    TransactionPosting::new(
                        backing_account.account().into(),
                        fx_discrepancy,
                        main_commodity.currency()?,
                    ),
                    TransactionPosting::new(
                        if fx_discrepancy > 0.0 {
                            REALIZED_FX_GAIN.clone().into()
                        } else {
                            REALIZED_FX_LOSS.clone().into()
                        },
                        -fx_discrepancy,
                        main_commodity.currency()?,
                    ),
                ],
                comment: Some("Correct FX discrepancy".to_string()),
            }]
        } else {
            vec![]
        };

        // Tag this transaction, since the accounting logic deserves a note in
        // the financial records.
        let note = Annotation::CardFxBySettle;

        Ok(DecoratedTransactionSpec {
            id,
            accrual_start,
            accrual_end,
            payment_date,
            accounting_logic,
            payee,
            description,
            amount: converted_amount,
            commodity: main_commodity,
            backing_account,
            annotations: annotations.into_iter().chain(once(note)).collect(),
            ext_transactions: ext_transactions
                .into_iter()
                .chain(vat_transactions)
                .collect(),
            ext_assertions,
        })
    }

    async fn apply_immediate_with_fee<H: Handlers>(
        &self,
        tx: DecoratedTransactionSpec<H>,
        charged: f64,
        fee: f64,
    ) -> Result<DecoratedTransactionSpec<H>, ServerError> {
        let DecoratedTransactionSpec {
            id,
            accrual_start,
            accrual_end,
            payment_date,
            accounting_logic,
            payee,
            description,
            amount: source_amount,
            commodity: source_commodity,
            backing_account,
            annotations,
            ext_transactions,
            ext_assertions,
        } = tx;

        // IMPORTANT NOTE:
        //   If this transaction is an expense, the amount is negative.

        let main_commodity = H::M::default();

        // Calculate the amount to record on payment date (amount converted at
        // the rate on that day).
        let fx_util = FxUtil::using_open_exchange_rates_api(
            &self.currency_conversion_api_key,
            &self.currency_conversion_cache_dir,
        )?;
        let converted_amount = fx_util
            .convert(
                payment_date,
                source_commodity.iso_symbol(),
                main_commodity.iso_symbol(),
                source_amount,
            )
            .await?;

        let fee_transaction = if fee.abs() >= main_commodity.precision_cutoff()? {
            Some(Transaction {
                spec_id: id.clone(),
                date: payment_date,
                postings: vec![
                    TransactionPosting::new(
                        backing_account.account().into(),
                        -fee,
                        main_commodity.currency()?,
                    ),
                    TransactionPosting::new(
                        FOREIGN_TRANSACTION_FEE.clone().into(),
                        fee,
                        main_commodity.currency()?,
                    ),
                ],
                comment: Some("Foreign transaction fee".to_string()),
            })
        } else {
            None
        };

        let fx_discrepency = {
            let abs = charged.abs() - fee.abs() - converted_amount.abs();
            if source_amount > 0.0 {
                // For income, positive discrepancy is a gain.
                abs
            } else {
                // For expense, positive discrepancy is a loss.
                -abs
            }
        };
        let fx_discrepency_transaction =
            if fx_discrepency.abs() >= main_commodity.precision_cutoff()? {
                Some(Transaction {
                    spec_id: id.clone(),
                    date: payment_date,
                    postings: vec![
                        TransactionPosting::new(
                            backing_account.account().into(),
                            fx_discrepency,
                            main_commodity.currency()?,
                        ),
                        TransactionPosting::new(
                            if fx_discrepency > 0.0 {
                                REALIZED_FX_GAIN.clone().into()
                            } else {
                                REALIZED_FX_LOSS.clone().into()
                            },
                            -fx_discrepency,
                            main_commodity.currency()?,
                        ),
                    ],
                    comment: Some("Correct FX discrepancy".to_string()),
                })
            } else {
                None
            };

        // Tag this transaction, since the accounting logic deserves a note in
        // the financial records.
        let note = Annotation::CardFxByFee;

        Ok(DecoratedTransactionSpec {
            id,
            accrual_start,
            accrual_end,
            payment_date,
            accounting_logic,
            payee,
            description,
            amount: converted_amount,
            commodity: main_commodity,
            backing_account,
            annotations: annotations.into_iter().chain(once(note)).collect(),
            ext_transactions: ext_transactions
                .into_iter()
                .chain(
                    vec![fee_transaction, fx_discrepency_transaction]
                        .into_iter()
                        .flatten(),
                )
                .collect(),
            ext_assertions,
        })
    }
}

#[async_trait]
impl<H: Handlers> DecoratorLogic<H> for StandardDecoratorCardFx {
    async fn apply(
        &self,
        tx: DecoratedTransactionSpec<H>,
    ) -> Result<DecoratedTransactionSpec<H>, ServerError> {
        match &self.logic {
            LogicType::DelayedSettleUnknownFee {
                settle_date,
                settle_amount,
            } => {
                self.apply_delayed_settle_unknown_fee(tx, *settle_date, *settle_amount)
                    .await
            }
            LogicType::ImmediateWithFee { charged, fee } => {
                self.apply_immediate_with_fee(tx, *charged, *fee).await
            }
        }
    }
}
