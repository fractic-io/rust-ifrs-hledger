use std::iter::once;

use async_trait::async_trait;
use fractic_server_error::{DivisionByZeroError, ServerError};

use crate::{
    entities::{
        Annotation, CommodityHandler, DecoratedTransactionSpec, DecoratorLogic, Handlers,
        Transaction, TransactionPosting,
    },
    ext::standard_accounts::FOREIGN_WITHHOLDING_TAX,
};

#[derive(Debug)]
enum LogicType {
    UnrecoverableForeign { percent: f64 },
}

#[derive(Debug)]
pub struct StandardDecoratorWithholdingTax {
    logic: LogicType,
}

impl StandardDecoratorWithholdingTax {
    pub fn unrecoverable_foreign(percent: f64) -> Result<Self, ServerError> {
        Ok(Self {
            logic: LogicType::UnrecoverableForeign { percent },
        })
    }

    // --

    fn apply_unrecoverable_foreign<H: Handlers>(
        &self,
        tx: DecoratedTransactionSpec<H>,
        percent: f64,
    ) -> Result<DecoratedTransactionSpec<H>, ServerError> {
        let DecoratedTransactionSpec {
            id,
            accrual_start,
            accrual_end,
            payment_date,
            accounting_logic,
            payee,
            description,
            amount: amount_post_withholding,
            commodity,
            backing_account,
            annotations,
            ext_transactions,
            ext_assertions,
        } = tx;

        // IMPORTANT NOTE:
        //   If this transaction is an expense, the amount is negative.

        let amount_pre_withholding = amount_post_withholding / (1.0 - percent / 100.0);
        if amount_pre_withholding.is_nan() || amount_pre_withholding.is_infinite() {
            return Err(DivisionByZeroError::new());
        }
        let withholding_amount = amount_pre_withholding - amount_post_withholding;

        let withholding_transaction = Transaction {
            spec_id: id.clone(),
            date: payment_date,
            postings: vec![
                TransactionPosting::new(
                    backing_account.account().into(),
                    -withholding_amount,
                    commodity.currency()?,
                ),
                TransactionPosting::new(
                    FOREIGN_WITHHOLDING_TAX.clone().into(),
                    withholding_amount,
                    commodity.currency()?,
                ),
            ],
            comment: Some("Foreign withholding tax".to_string()),
        };

        // Tag this transaction, since the accounting logic deserves a note in
        // the financial records.
        let note = Annotation::ForeignWithholdingTax(percent as i32);

        Ok(DecoratedTransactionSpec {
            id,
            accrual_start,
            accrual_end,
            payment_date,
            accounting_logic,
            payee,
            description,
            amount: amount_pre_withholding,
            commodity,
            backing_account,
            annotations: annotations.into_iter().chain(once(note)).collect(),
            ext_transactions: ext_transactions
                .into_iter()
                .chain(once(withholding_transaction))
                .collect(),
            ext_assertions,
        })
    }
}

#[async_trait]
impl<H: Handlers> DecoratorLogic<H> for StandardDecoratorWithholdingTax {
    async fn apply(
        &self,
        tx: DecoratedTransactionSpec<H>,
    ) -> Result<DecoratedTransactionSpec<H>, ServerError> {
        match &self.logic {
            LogicType::UnrecoverableForeign { percent } => {
                self.apply_unrecoverable_foreign(tx, *percent)
            }
        }
    }
}
