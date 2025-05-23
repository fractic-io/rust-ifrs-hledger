use std::{iter::once, path::PathBuf};

use async_trait::async_trait;
use fractic_currency_conversion::util::FxUtil;
use fractic_server_error::ServerError;

use crate::{
    entities::{
        Annotation, CommodityHandler, DecoratedTransactionSpec, DecoratorLogic, Handlers,
        Transaction, TransactionPosting,
    },
    ext::standard_accounts::FOREIGN_TRANSACTION_FEE,
};

#[derive(Debug)]
pub struct StandardDecoratorCardFxByFee {
    fee_percent: f64,
    currency_conversion_cache_dir: PathBuf,
    currency_conversion_api_key: String,
}

impl StandardDecoratorCardFxByFee {
    pub fn new(
        fee_percent: f64,
        currency_conversion_cache_dir: impl Into<PathBuf>,
        currency_conversion_api_key: impl Into<String>,
    ) -> Result<Self, ServerError> {
        Ok(Self {
            fee_percent,
            currency_conversion_cache_dir: currency_conversion_cache_dir.into(),
            currency_conversion_api_key: currency_conversion_api_key.into(),
        })
    }
}

#[async_trait]
impl<H: Handlers> DecoratorLogic<H> for StandardDecoratorCardFxByFee {
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

        let foreign_transaction_fee = converted_amount.abs() * self.fee_percent / 100.0;
        let vat_transactions = if foreign_transaction_fee.abs() > 0.01 {
            vec![Transaction {
                spec_id: id.clone(),
                date: payment_date,
                postings: vec![
                    TransactionPosting::new(
                        backing_account.account().into(),
                        -foreign_transaction_fee,
                        main_commodity.currency()?,
                    ),
                    TransactionPosting::new(
                        FOREIGN_TRANSACTION_FEE.clone().into(),
                        foreign_transaction_fee,
                        main_commodity.currency()?,
                    ),
                ],
                comment: Some("Foreign transaction fee".to_string()),
            }]
        } else {
            vec![]
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
                .chain(vat_transactions)
                .collect(),
            ext_assertions,
        })
    }
}
