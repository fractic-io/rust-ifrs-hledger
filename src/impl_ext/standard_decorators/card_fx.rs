use std::{iter::once, path::PathBuf, str::FromStr as _};

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
    ext::standard_accounts::{FX_GAIN, FX_LOSS},
};

#[derive(Debug)]
pub struct StandardDecoratorCardFx {
    settle_date: NaiveDate,
    settle_amount: f64,
    currency_conversion_cache_dir: PathBuf,
    currency_conversion_api_key: String,
}

impl StandardDecoratorCardFx {
    pub fn new(
        settle_date: &String,
        settle_amount: f64,
        currency_conversion_cache_dir: impl Into<PathBuf>,
        currency_conversion_api_key: impl Into<String>,
    ) -> Result<Self, ServerError> {
        Ok(Self {
            settle_date: ISODateModel::from_str(settle_date)?.into(),
            settle_amount,
            currency_conversion_cache_dir: currency_conversion_cache_dir.into(),
            currency_conversion_api_key: currency_conversion_api_key.into(),
        })
    }
}

#[async_trait]
impl<H: Handlers> DecoratorLogic<H> for StandardDecoratorCardFx {
    async fn apply(
        &self,
        tx: DecoratedTransactionSpec<H>,
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
            self.settle_amount.abs() - converted_amount.abs()
        } else {
            // For expense, positive discrepancy is a loss.
            converted_amount.abs() - self.settle_amount.abs()
        };
        let vat_transactions = if fx_discrepancy.abs() > 0.01 {
            vec![Transaction {
                spec_id: id.clone(),
                date: self.settle_date,
                postings: vec![
                    TransactionPosting {
                        account: backing_account.account().into(),
                        amount: fx_discrepancy,
                        currency: main_commodity.currency()?,
                    },
                    TransactionPosting {
                        account: if fx_discrepancy > 0.0 {
                            FX_GAIN.clone().into()
                        } else {
                            FX_LOSS.clone().into()
                        },
                        amount: -fx_discrepancy,
                        currency: main_commodity.currency()?,
                    },
                ],
                comment: Some("VAT awaiting invoice".to_string()),
            }]
        } else {
            vec![]
        };

        // Tag this transaction, since the accounting logic deserves a note in
        // the financial records.
        let note = Annotation::CardFx;

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
}
