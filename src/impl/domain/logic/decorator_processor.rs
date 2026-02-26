use fractic_server_error::ServerError;
use futures::{
    stream::{self, StreamExt},
    TryStreamExt,
};

use crate::entities::{
    DecoratedTransactionSpec, DecoratorHandler, FinancialRecordSpecs,
    FinancialRecords_Intermediate1, Handlers,
};

pub(crate) struct DecoratorProcessor<H: Handlers> {
    specs: FinancialRecordSpecs<H>,
}

impl<H: Handlers> DecoratorProcessor<H> {
    pub(crate) fn new(specs: FinancialRecordSpecs<H>) -> Self {
        Self { specs }
    }

    pub(crate) async fn process(self) -> Result<FinancialRecords_Intermediate1<H>, ServerError> {
        let FinancialRecordSpecs {
            transaction_specs,
            commands,
            assertion_specs,
        } = self.specs;

        // let decorated_transaction_specs = transaction_specs
        //     .into_iter()
        //     .map(|tx| {
        //         tx.decorators.into_iter().try_fold(
        //             DecoratedTransactionSpec {
        //                 id: tx.id,
        //                 accrual_start: tx.accrual_start,
        //                 accrual_end: tx.accrual_end,
        //                 payment_date: tx.payment_date,
        //                 accounting_logic: tx.accounting_logic,
        //                 payee: tx.payee,
        //                 description: tx.description,
        //                 amount: tx.amount,
        //                 commodity: tx.commodity,
        //                 backing_account: tx.backing_account,
        //                 annotations: tx.annotations,
        //                 ext_transactions: Default::default(),
        //                 ext_assertions: Default::default(),
        //             },
        //             |acc, dec| dec.logic()?.apply(acc).await,
        //         )
        //     })
        //     .collect::<Result<Vec<_>, ServerError>>()?;

        let decorated_transaction_specs = stream::iter(transaction_specs)
            .then(|tx| async move {
                let initial = DecoratedTransactionSpec {
                    id: tx.id,
                    accrual_start: tx.accrual_start,
                    accrual_end: tx.accrual_end,
                    payment_date: tx.payment_date,
                    accounting_logic: tx.accounting_logic,
                    payee: tx.payee,
                    description: tx.description,
                    amount: tx.amount,
                    commodity: tx.commodity,
                    backing_account: tx.backing_account,
                    annotations: tx.annotations,
                    ext_transactions: Default::default(),
                    ext_assertions: Default::default(),
                    ext_raw: Default::default(),
                };

                stream::iter(tx.decorators.into_iter().map(Ok))
                    .try_fold(initial, |acc, dec| async move {
                        let l = dec.logic()?;
                        l.apply(acc).await
                    })
                    .await
            })
            .try_collect::<Vec<_>>()
            .await?;

        Ok(FinancialRecords_Intermediate1 {
            transaction_specs: decorated_transaction_specs,
            commands,
            assertion_specs,
        })
    }
}
