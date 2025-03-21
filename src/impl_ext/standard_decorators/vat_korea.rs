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
    ext::standard_accounts::{
        VAT_ADJUSTMENT_EXPENSE, VAT_ADJUSTMENT_INCOME, VAT_PAYABLE, VAT_PENDING_RECEIPT,
        VAT_RECEIVABLE,
    },
};

#[derive(Debug)]
enum LogicType {
    AwaitingInvoice,
    Recoverable { invoice_date: NaiveDate },
    Unrecoverable,
    ReverseChargeExempt,
    RefundAdjustment { core_amount: f64 },
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

    /// Records an adjustment record to correct discrepancy between the VAT
    /// refund expected during the accounting peroid and the actual refund.
    ///
    /// core_amount: The amount that had previously been set aside in accounting
    /// records for a refund.
    pub fn refund_adjustment(core_amount: f64) -> Result<Self, ServerError> {
        Ok(Self {
            logic: LogicType::RefundAdjustment { core_amount },
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
                    currency: commodity.currency()?,
                },
                TransactionPosting {
                    account: VAT_PENDING_RECEIPT.clone().into(),
                    amount: -amount_vat,
                    currency: commodity.currency()?,
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
                        currency: commodity.currency()?,
                    },
                    TransactionPosting {
                        account: VAT_PENDING_RECEIPT.clone().into(),
                        amount: -amount_vat,
                        currency: commodity.currency()?,
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
                        currency: commodity.currency()?,
                    },
                    TransactionPosting {
                        account: if amount_vat > 0.0 {
                            VAT_PAYABLE.clone().into()
                        } else {
                            VAT_RECEIVABLE.clone().into()
                        },
                        amount: -amount_vat,
                        currency: commodity.currency()?,
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

    fn apply_refund_adjustment<H: Handlers>(
        &self,
        tx: DecoratedTransactionSpec<H>,
        core_amount: f64,
    ) -> Result<DecoratedTransactionSpec<H>, ServerError> {
        let DecoratedTransactionSpec {
            id,
            accrual_start,
            accrual_end,
            payment_date,
            accounting_logic,
            payee,
            description,
            amount: cleared_amount,
            commodity,
            backing_account,
            annotations,
            ext_transactions,
            ext_assertions,
        } = tx;

        // Infer the sign of core_amount based on the sign of cleared_amount.
        // This lets the accounting be written more easily and avoids
        // accidentally double-inverting the amount.
        let core_amount = if cleared_amount > 0.0 {
            core_amount.abs()
        } else {
            -core_amount.abs()
        };

        let discrepancy = cleared_amount - core_amount;
        let vat_transactions = if discrepancy.abs() > 0.01 {
            vec![Transaction {
                spec_id: id.clone(),
                date: payment_date,
                postings: vec![
                    TransactionPosting {
                        account: if discrepancy > 0.0 {
                            VAT_ADJUSTMENT_INCOME.clone().into()
                        } else {
                            VAT_ADJUSTMENT_EXPENSE.clone().into()
                        },
                        amount: -discrepancy,
                        currency: commodity.currency()?,
                    },
                    TransactionPosting {
                        account: backing_account.account().into(),
                        amount: discrepancy,
                        currency: commodity.currency()?,
                    },
                ],
                comment: Some("VAT refund adjustment".to_string()),
            }]
        } else {
            vec![]
        };

        Ok(DecoratedTransactionSpec {
            id,
            accrual_start,
            accrual_end,
            payment_date,
            accounting_logic,
            payee,
            description,
            amount: core_amount,
            commodity,
            backing_account,
            annotations,
            ext_transactions: ext_transactions
                .into_iter()
                .chain(vat_transactions)
                .collect(),
            ext_assertions,
        })
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
            LogicType::RefundAdjustment { core_amount } => {
                self.apply_refund_adjustment(tx, *core_amount)
            }
        }
    }
}
