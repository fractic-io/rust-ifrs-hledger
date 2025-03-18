use std::{iter::once, str::FromStr};

use async_trait::async_trait;
use chrono::NaiveDate;
use fractic_server_error::ServerError;

use crate::{
    data::models::iso_date_model::ISODateModel,
    entities::{
        Annotation, CommodityHandler as _, DecoratedTransactionSpec, DecoratorLogic, Handlers,
        Transaction, TransactionPosting,
    },
    ext::standard_accounts::{VAT_PAYABLE, VAT_PENDING_RECEIPT, VAT_RECEIVABLE},
};

#[derive(Debug)]
enum LogicType {
    AwaitingInvoice,
    Recoverable { invoice_date: NaiveDate },
    Unrecoverable,
    ReverseChargeExempt,
}

#[derive(Debug)]
pub struct StandardDecoratorVatKorea {
    logic: LogicType,
}

impl StandardDecoratorVatKorea {
    pub fn awaiting_invoice() -> Result<Self, ServerError> {
        Ok(Self {
            logic: LogicType::AwaitingInvoice,
        })
    }

    pub fn recoverable(invoice_date: &String) -> Result<Self, ServerError> {
        Ok(Self {
            logic: LogicType::Recoverable {
                invoice_date: ISODateModel::from_str(invoice_date)?.into(),
            },
        })
    }

    pub fn unrecoverable() -> Result<Self, ServerError> {
        Ok(Self {
            logic: LogicType::Unrecoverable,
        })
    }

    pub fn reverse_charge_exempt() -> Result<Self, ServerError> {
        Ok(Self {
            logic: LogicType::ReverseChargeExempt,
        })
    }

    // --

    fn apply_awaiting_invoice<H: Handlers>(
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
            amount: amount_total,
            commodity,
            backing_account,
            annotations,
            ext_transactions,
            ext_assertions,
        } = tx;

        // IMPORTANT NOTE:
        //   If this transaction is an expense, the amount is negative.

        let amount_core = amount_total / 1.1;
        let amount_vat = amount_total - amount_core;

        let vat_transactions = vec![Transaction {
            spec_id: id.clone(),
            date: payment_date,
            postings: vec![
                TransactionPosting {
                    account: backing_account.account().into(),
                    amount: amount_vat,
                    currency: commodity.iso_symbol(),
                },
                TransactionPosting {
                    account: VAT_PENDING_RECEIPT.clone().into(),
                    amount: -amount_vat,
                    currency: commodity.iso_symbol(),
                },
            ],
            comment: Some("VAT awaiting invoice".to_string()),
        }];

        // Tag this transaction, since the accounting logic deserves a note in
        // the financial records.
        let note = Annotation::VatKorea;

        Ok(DecoratedTransactionSpec {
            id,
            accrual_start,
            accrual_end,
            payment_date,
            accounting_logic,
            payee,
            description,
            amount: amount_core,
            commodity,
            backing_account,
            annotations: annotations.into_iter().chain(once(note)).collect(),
            ext_transactions: ext_transactions
                .into_iter()
                .chain(vat_transactions)
                .collect(),
            ext_assertions,
        })
    }

    fn apply_recoverable<H: Handlers>(
        &self,
        tx: DecoratedTransactionSpec<H>,
        invoice_date: NaiveDate,
    ) -> Result<DecoratedTransactionSpec<H>, ServerError> {
        let DecoratedTransactionSpec {
            id,
            accrual_start,
            accrual_end,
            payment_date,
            accounting_logic,
            payee,
            description,
            amount: amount_total,
            commodity,
            backing_account,
            annotations,
            ext_transactions,
            ext_assertions,
        } = tx;

        // IMPORTANT NOTE:
        //   If this transaction is an expense, the amount is negative.

        let amount_core = amount_total / 1.1;
        let amount_vat = amount_total - amount_core;

        let vat_transactions = vec![
            Transaction {
                spec_id: id.clone(),
                date: payment_date,
                postings: vec![
                    TransactionPosting {
                        account: backing_account.account().into(),
                        amount: amount_vat,
                        currency: commodity.iso_symbol(),
                    },
                    TransactionPosting {
                        account: VAT_PENDING_RECEIPT.clone().into(),
                        amount: -amount_vat,
                        currency: commodity.iso_symbol(),
                    },
                ],
                comment: Some("VAT awaiting invoice".to_string()),
            },
            Transaction {
                spec_id: id.clone(),
                date: invoice_date,
                postings: vec![
                    TransactionPosting {
                        account: VAT_PENDING_RECEIPT.clone().into(),
                        amount: amount_vat,
                        currency: commodity.iso_symbol(),
                    },
                    TransactionPosting {
                        account: if amount_vat > 0.0 {
                            VAT_RECEIVABLE.clone().into()
                        } else {
                            VAT_PAYABLE.clone().into()
                        },
                        amount: -amount_vat,
                        currency: commodity.iso_symbol(),
                    },
                ],
                comment: Some("VAT invoice received".to_string()),
            },
        ];

        // Tag this transaction, since the accounting logic deserves a note in
        // the financial records.
        let note = Annotation::VatKorea;

        Ok(DecoratedTransactionSpec {
            id,
            accrual_start,
            accrual_end,
            payment_date,
            accounting_logic,
            payee,
            description,
            amount: amount_core,
            commodity,
            backing_account,
            annotations: annotations.into_iter().chain(once(note)).collect(),
            ext_transactions: ext_transactions
                .into_iter()
                .chain(vat_transactions)
                .collect(),
            ext_assertions,
        })
    }

    fn apply_unrecoverable<H: Handlers>(
        &self,
        mut tx: DecoratedTransactionSpec<H>,
    ) -> Result<DecoratedTransactionSpec<H>, ServerError> {
        // Tag this transaction, since the accounting logic deserves a note in
        // the financial records.
        tx.annotations.push(Annotation::VatKoreaUnrecoverable);
        Ok(tx)
    }

    fn apply_reverse_charge_exempt<H: Handlers>(
        &self,
        mut tx: DecoratedTransactionSpec<H>,
    ) -> Result<DecoratedTransactionSpec<H>, ServerError> {
        // Tag this transaction, since the accounting logic deserves a note in
        // the financial records.
        tx.annotations.push(Annotation::VatKoreaReverseChargeExempt);
        Ok(tx)
    }
}

#[async_trait]
impl<H: Handlers> DecoratorLogic<H> for StandardDecoratorVatKorea {
    async fn apply(
        &self,
        tx: DecoratedTransactionSpec<H>,
    ) -> Result<DecoratedTransactionSpec<H>, ServerError> {
        match &self.logic {
            LogicType::AwaitingInvoice => self.apply_awaiting_invoice(tx),
            LogicType::Recoverable { invoice_date } => self.apply_recoverable(tx, *invoice_date),
            LogicType::Unrecoverable => self.apply_unrecoverable(tx),
            LogicType::ReverseChargeExempt => self.apply_reverse_charge_exempt(tx),
        }
    }
}
