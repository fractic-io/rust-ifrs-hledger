use fractic_server_error::ServerError;

use crate::entities::{
    DecoratedFinancialRecordSpecs, DecoratedTransactionSpec, DecoratorHandler,
    FinancialRecordSpecs, Handlers,
};

pub(crate) struct DecoratorProcessor<H: Handlers> {
    specs: FinancialRecordSpecs<H>,
}

impl<H: Handlers> DecoratorProcessor<H> {
    pub(crate) fn new(specs: FinancialRecordSpecs<H>) -> Self {
        Self { specs }
    }

    pub(crate) fn process(self) -> Result<DecoratedFinancialRecordSpecs<H>, ServerError> {
        let FinancialRecordSpecs {
            transaction_specs,
            assertion_specs,
        } = self.specs;

        let decorated_transaction_specs = transaction_specs
            .into_iter()
            .map(|tx| {
                tx.decorators.into_iter().try_fold(
                    DecoratedTransactionSpec {
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
                    },
                    |acc, dec| dec.logic().apply(acc),
                )
            })
            .collect::<Result<Vec<_>, ServerError>>()?;

        Ok(DecoratedFinancialRecordSpecs {
            transaction_specs: decorated_transaction_specs,
            assertion_specs,
        })
    }
}

// // ---------------------------------------------------------------------
// // Decorator processing.
// // This function takes the “core” transactions (assumed to be in txs[0])
// // and returns any extra side transactions.
// // ---------------------------------------------------------------------
// pub fn apply_decorators(
//     txs: &mut Vec<LedgerTransaction>,
//     decorators: &Vec<Decorator>,
//     _accrual_date: NaiveDate,
//     payment_date: NaiveDate,
//     commodity: &str,
//     description: &str,
// ) -> Result<Vec<LedgerTransaction>, ServerError> {
//     let mut side_txs = Vec::new();
//     if txs.is_empty() {
//         return Ok(side_txs);
//     }
//     let core_tx = &mut txs[0];
//     for dec in decorators {
//         match dec {
//             Decorator::VatAwaitingInvoice {} => {
//                 core_tx
//                     .notes
//                     .push(String::from("VAT charged but awaiting invoice"));
//             }
//             Decorator::VatRecoverable {
//                 invoice: ISODate(invoice_date),
//             } => {
//                 // Assume first posting is the main one.
//                 let orig_amount = core_tx.postings[0].amount;
//                 let core_amount = orig_amount * 100.0 / 110.0;
//                 let vat_amount = orig_amount - core_amount;
//                 core_tx.postings[0].amount = core_amount;
//                 // Adjust the backing posting proportionally.
//                 core_tx.postings[1].amount =
//                     -core_tx.postings[1].amount * (core_amount / orig_amount);
//                 let side_tx1 = LedgerTransaction {
//                     date: payment_date,
//                     description: format!(
//                         "VAT Recoverable: Record Vat Receipt Pending for {}",
//                         description
//                     ),
//                     postings: vec![
//                         Posting {
//                             account: LiabilityAccount("VatReceiptPending".into()).into(),
//                             amount: vat_amount,
//                             commodity: commodity.to_string(),
//                         },
//                         Posting {
//                             account: ExpenseAccount("VatSubtraction".into()).into(),
//                             amount: -vat_amount,
//                             commodity: commodity.to_string(),
//                         },
//                     ],
//                     notes: vec![],
//                 };
//                 let side_tx2 = LedgerTransaction {
//                     date: invoice_date.clone(),
//                     description: format!(
//                         "VAT Recoverable: Clear Vat Receipt Pending into Vat Receivable for {}",
//                         description
//                     ),
//                     postings: vec![
//                         Posting {
//                             account: AssetAccount("VatReceivable".into()).into(),
//                             amount: vat_amount,
//                             commodity: commodity.to_string(),
//                         },
//                         Posting {
//                             account: LiabilityAccount("VatReceiptPending".into()).into(),
//                             amount: -vat_amount,
//                             commodity: commodity.to_string(),
//                         },
//                     ],
//                     notes: vec![],
//                 };
//                 side_txs.push(side_tx1);
//                 side_txs.push(side_tx2);
//             }
//             Decorator::VatUnrecoverable => {
//                 let orig_amount = core_tx.postings[0].amount;
//                 let core_amount = orig_amount * 100.0 / 110.0;
//                 let vat_amount = orig_amount - core_amount;
//                 core_tx.postings[0].amount = core_amount;
//                 core_tx.postings[1].amount =
//                     -core_tx.postings[1].amount * (core_amount / orig_amount);
//                 let side_tx = LedgerTransaction {
//                     date: payment_date,
//                     description: format!(
//                         "VAT Unrecoverable: Record unrecoverable VAT expense for {}",
//                         description
//                     ),
//                     postings: vec![
//                         Posting {
//                             account: ExpenseAccount("VatUnrecoverable".into()).into(),
//                             amount: vat_amount,
//                             commodity: commodity.to_string(),
//                         },
//                         Posting {
//                             account: ExpenseAccount("VatSubtraction".into()).into(),
//                             amount: -vat_amount,
//                             commodity: commodity.to_string(),
//                         },
//                     ],
//                     notes: vec![],
//                 };
//                 side_txs.push(side_tx);
//             }
//             Decorator::VatReverseChargeExempt => {
//                 core_tx
//                     .notes
//                     .push(String::from("VAT charged on a reverse-charge basis"));
//             }
//             Decorator::CardFx {
//                 to: target_currency,
//                 settle: ISODate(settle_date),
//                 at,
//             } => {
//                 // Simulate a mid-market spot rate.
//                 let spot_rate = 1.05;
//                 for posting in &mut core_tx.postings {
//                     posting.amount *= spot_rate;
//                     posting.commodity = target_currency.clone();
//                 }
//                 let spot_conversion = core_tx.postings[0].amount;
//                 let diff = at - spot_conversion;
//                 let fx_account: Account = if diff >= 0.0 {
//                     IncomeAccount("FxGain".into()).into()
//                 } else {
//                     ExpenseAccount("FxLoss".into()).into()
//                 };
//                 let side_tx = LedgerTransaction {
//                     date: settle_date.clone(),
//                     description: format!("FX adjustment for {} via CardFx", description),
//                     postings: vec![
//                         Posting {
//                             account: fx_account,
//                             amount: diff,
//                             commodity: target_currency.clone(),
//                         },
//                         Posting {
//                             account: ExpenseAccount("FxSubtraction".into()).into(),
//                             amount: -diff,
//                             commodity: target_currency.clone(),
//                         },
//                     ],
//                     notes: vec![],
//                 };
//                 side_txs.push(side_tx);
//             }
//             Decorator::PaymentFee(fee) => {
//                 let side_tx = LedgerTransaction {
//                     date: payment_date,
//                     description: format!("Payment Fee: {} for {}", fee, description),
//                     postings: vec![
//                         Posting {
//                             account: ExpenseAccount("PaymentFees".into()).into(),
//                             amount: *fee,
//                             commodity: commodity.to_string(),
//                         },
//                         Posting {
//                             account: AssetAccount("Cash:Hana".into()).into(),
//                             amount: -fee,
//                             commodity: commodity.to_string(),
//                         },
//                     ],
//                     notes: vec![],
//                 };
//                 side_txs.push(side_tx);
//             }
//         }
//     }
//     Ok(side_txs)
// }
