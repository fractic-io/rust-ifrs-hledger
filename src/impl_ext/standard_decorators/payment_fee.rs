use fractic_server_error::ServerError;

use crate::{
    entities::{
        CommodityHandler, DecoratedTransactionSpec, DecoratorLogic, Handlers, Transaction,
        TransactionPosting,
    },
    ext::standard_accounts::PAYMENT_FEES,
};

pub struct StandardDecoratorPaymentFee {
    fee: f64,
}

impl StandardDecoratorPaymentFee {
    pub fn new(fee: f64) -> Self {
        Self { fee }
    }
}

impl<H: Handlers> DecoratorLogic<H> for StandardDecoratorPaymentFee {
    fn apply(
        &self,
        mut tx: DecoratedTransactionSpec<H>,
    ) -> Result<DecoratedTransactionSpec<H>, ServerError> {
        tx.ext_transactions.push(Transaction {
            spec_id: tx.id.clone(),
            date: tx.payment_date,
            postings: vec![
                TransactionPosting {
                    account: tx.backing_account.account().into(),
                    amount: -self.fee,
                    currency: tx.commodity.iso_symbol(),
                },
                TransactionPosting {
                    account: PAYMENT_FEES.clone().into(),
                    amount: self.fee,
                    currency: tx.commodity.iso_symbol(),
                },
            ],
            comment: Some("Payment fee".to_string()),
        });
        Ok(tx)
    }
}
