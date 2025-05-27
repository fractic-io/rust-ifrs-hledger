use std::{
    collections::{HashMap, VecDeque},
    iter::once,
};

use chrono::{Duration, NaiveDate};
use fractic_server_error::ServerError;

use crate::{
    domain::logic::utils::{
        compute_daily_average, monthly_accrual_adjustments, round_to_currency_precision,
        track_unreimbursed_entries, MonthlyAccrualAdjustment,
    },
    entities::{
        Account, AccountingLogic, Annotation, Assertion, AssetHandler, BackingAccount, CashHandler,
        CommodityHandler, CommonStockWhileUnpaid, DecoratedFinancialRecordSpecs,
        DecoratedTransactionSpec, ExpenseAccount, ExpenseHandler, FinancialRecords, Handlers,
        IncomeHandler, LiabilityAccount, PayeeHandler, ReimbursableEntityHandler,
        ShareholderHandler, Transaction, TransactionLabel, TransactionPosting, TransactionSpecId,
    },
    errors::{
        CommonStockCannotBePrepaid, InvalidArgumentsForAccountingLogic, NoTransactionsToReimburse,
        NonAmortizableAsset, UnexpectedNegativeValue, UnexpectedPartialReimbursement,
        UnexpectedPositiveValue, VariableExpenseDoubleInit, VariableExpenseInvalidPaymentDate,
        VariableExpenseNoInit, VariableExpenseNotEnoughHistoricalData,
    },
    ext::standard_accounts::{
        PREPAID_SHARE_ISSUANCE_COSTS, SHARE_ISSUANCE_COSTS, SHARE_ISSUANCE_COSTS_PAYABLE,
        UNPAID_SHARE_CAPITAL_AS_ASSET, UNPAID_SHARE_CAPITAL_AS_EQUITY,
    },
    impl_ext::standard_accounts::vat::{VAT_PAYABLE, VAT_RECEIVABLE},
};

use super::utils::PopByAmount;

pub(crate) struct SpecProcessor<H: Handlers> {
    specs: DecoratedFinancialRecordSpecs<H>,
}

/// Store historical information of variables expenses, to use for making
/// estimates.
#[derive(Debug, Clone, Default)]
pub(crate) struct ExpenseHistory {
    pub(crate) init_date: Option<NaiveDate>,
    pub(crate) price_records: Vec<ExpenseHistoryPriceRecord>,
}
#[derive(Debug, Clone)]
pub(crate) struct ExpenseHistoryPriceRecord {
    pub(crate) start: NaiveDate,
    pub(crate) end: NaiveDate,
    pub(crate) daily_rate: f64,
}
#[derive(Debug, Clone)]
pub(crate) struct ExpenseHistoryDelta {
    pub(crate) account: ExpenseAccount,
    pub(crate) price_record: ExpenseHistoryPriceRecord,
    pub(crate) is_init: bool,
}

/// Keep track of unreimbursed entries.
pub(crate) type ReimbursementState = HashMap<LiabilityAccount, VecDeque<UnreimbursedEntry>>;
#[derive(Debug, Clone)]
pub struct UnreimbursedEntry {
    pub(crate) transaction_date: NaiveDate,
    pub(crate) total_amount: f64,
    pub(crate) credit_postings: Vec<TransactionPosting>,
}
#[derive(Debug)]
pub(crate) enum ReimbursementStateDelta {
    Pop {
        date: NaiveDate,
        account: LiabilityAccount,
        amount: f64,
    },
    Push {
        account: LiabilityAccount,
        entries: Vec<UnreimbursedEntry>,
    },
}

struct Transformation {
    spec_id: TransactionSpecId,
    label: TransactionLabel,
    transactions: Vec<Transaction>,
    ext_transactions: Vec<Transaction>,
    ext_assertions: Vec<Assertion>,
    expense_history_delta: Option<ExpenseHistoryDelta>,
    reimbursement_state_delta: Option<ReimbursementStateDelta>,
    annotations: Vec<Annotation>,
}

struct FoldState {
    transactions: Vec<Transaction>,
    assertions: Vec<Assertion>,
    expense_history_lookup: HashMap<ExpenseAccount, ExpenseHistory>,
    label_lookup: HashMap<TransactionSpecId, TransactionLabel>,
    annotations_lookup: HashMap<TransactionSpecId, Vec<Annotation>>,
    reimbursement_state: ReimbursementState,
}

impl FoldState {
    fn new() -> Self {
        Self {
            transactions: Vec::new(),
            assertions: Vec::new(),
            expense_history_lookup: HashMap::new(),
            label_lookup: HashMap::new(),
            annotations_lookup: HashMap::new(),
            reimbursement_state: HashMap::new(),
        }
    }

    /// Update current state with the given transformation.
    fn step(self, t: Transformation) -> Result<Self, ServerError> {
        let FoldState {
            mut transactions,
            mut assertions,
            mut expense_history_lookup,
            mut label_lookup,
            mut annotations_lookup,
            mut reimbursement_state,
        } = self;

        match t.reimbursement_state_delta {
            Some(ReimbursementStateDelta::Pop {
                date,
                account,
                amount,
            }) => {
                reimbursement_state
                    .entry(account)
                    .or_default()
                    .pop_until_exactly(amount, date)?;
            }
            Some(ReimbursementStateDelta::Push { account, entries }) => {
                reimbursement_state
                    .entry(account)
                    .or_default()
                    .extend(entries);
            }
            None => {}
        }

        transactions.extend(t.transactions);
        transactions.extend(t.ext_transactions);
        assertions.extend(t.ext_assertions);

        if let Some(delta) = t.expense_history_delta {
            let expense_history = expense_history_lookup.entry(delta.account).or_default();
            if delta.is_init {
                if expense_history.init_date.is_some() {
                    return Err(VariableExpenseDoubleInit::new(&t.label.description));
                }
                expense_history.init_date = Some(delta.price_record.start);
            }
            expense_history.price_records.push(delta.price_record);
        }

        label_lookup.insert(t.spec_id, t.label);
        annotations_lookup
            .entry(t.spec_id)
            .or_default()
            .extend(t.annotations);

        Ok(Self {
            transactions,
            assertions,
            expense_history_lookup,
            label_lookup,
            annotations_lookup,
            reimbursement_state,
        })
    }
}

macro_rules! amount_should_be_negative {
    ($amount:expr, $logic:expr, $id:expr) => {
        if $amount >= 0.0 {
            return Err(UnexpectedPositiveValue::new($amount, $logic, $id));
        }
    };
}

macro_rules! amount_should_be_positive {
    ($amount:expr, $logic:expr, $id:expr) => {
        if $amount <= 0.0 {
            return Err(UnexpectedNegativeValue::new($amount, $logic, $id));
        }
    };
}

impl<H: Handlers> SpecProcessor<H> {
    pub(crate) fn new(specs: DecoratedFinancialRecordSpecs<H>) -> Self {
        Self { specs }
    }

    pub(crate) fn process(self) -> Result<FinancialRecords, ServerError> {
        let DecoratedFinancialRecordSpecs {
            mut transaction_specs,
            assertion_specs,
        } = self.specs;

        // Important for reimbursement tracking.
        transaction_specs.sort_by_key(|s| s.payment_date);

        let transactions_fold_result =
            transaction_specs
                .into_iter()
                .try_fold(FoldState::new(), |state, spec| {
                    let transformation = match &spec.accounting_logic {
                        AccountingLogic::CommonStock { .. } => Self::process_common_stock(spec)?,
                        AccountingLogic::CostOfEquity => Self::process_cost_of_equity(spec)?,
                        AccountingLogic::SimpleExpense(..) => Self::process_simple_expense(spec)?,
                        AccountingLogic::Capitalize(..) => Self::process_capitalize(spec)?,
                        AccountingLogic::Amortize(..) => Self::process_amortize(spec)?,
                        AccountingLogic::FixedExpense(..) => Self::process_fixed_expense(spec)?,
                        AccountingLogic::VariableExpenseInit { .. } => {
                            Self::process_variable_expense_init(spec)?
                        }
                        AccountingLogic::VariableExpense(..) => {
                            Self::process_variable_expense(spec, &state.expense_history_lookup)?
                        }
                        AccountingLogic::ImmaterialIncome(..) => {
                            Self::process_immaterial_income(spec)?
                        }
                        AccountingLogic::ImmaterialExpense(..) => {
                            Self::process_immaterial_expense(spec)?
                        }
                        AccountingLogic::Reimburse(..) => {
                            Self::process_reimburse(spec, &state.reimbursement_state)?
                        }
                        AccountingLogic::ReimbursePartial { .. } => {
                            Self::process_reimburse_partial(spec, &state.reimbursement_state)?
                        }
                        AccountingLogic::ClearVat { .. } => Self::process_clear_vat(spec)?,
                    };
                    state.step(transformation)
                })?;

        let assertions = assertion_specs
            .into_iter()
            .map(|spec| {
                Ok(Assertion {
                    date: spec.date,
                    account: spec.cash_handler.account().into(),
                    balance: spec.balance,
                    currency: spec.commodity.currency()?,
                })
            })
            .collect::<Result<Vec<Assertion>, ServerError>>()?
            .into_iter()
            .chain(transactions_fold_result.assertions.into_iter())
            .collect();

        Ok(FinancialRecords {
            transactions: transactions_fold_result.transactions,
            assertions,
            label_lookup: transactions_fold_result.label_lookup,
            annotations_lookup: transactions_fold_result.annotations_lookup,
            unreimbursed_entries: transactions_fold_result
                .reimbursement_state
                .into_iter()
                .flat_map(|(account, entries)| {
                    entries
                        .into_iter()
                        .map(move |entry| (account.clone(), entry))
                })
                .collect(),
        })
    }

    fn process_common_stock(
        spec: DecoratedTransactionSpec<H>,
    ) -> Result<Transformation, ServerError> {
        let DecoratedTransactionSpec {
            id,
            accrual_start: accrual_date,
            accrual_end: None,
            payment_date,
            accounting_logic:
                AccountingLogic::CommonStock {
                    subscriber: s_handler,
                    while_unpaid,
                },
            payee,
            description,
            amount,
            commodity,
            backing_account,
            annotations,
            ext_transactions,
            ext_assertions,
        } = spec
        else {
            return Err(InvalidArgumentsForAccountingLogic::with_debug(&spec));
        };
        amount_should_be_positive!(amount, "CommonStock", &id);

        let transactions = if payment_date == accrual_date {
            // Capital contribution was made on the same day, so record in a
            // single journal entry.
            vec![Transaction {
                spec_id: id,
                date: payment_date,
                comment: None,
                postings: vec![
                    TransactionPosting::new(
                        s_handler.account().into(),
                        -amount.abs(),
                        commodity.currency()?,
                    ),
                    TransactionPosting::new(
                        backing_account.account(),
                        amount.abs(),
                        commodity.currency()?,
                    ),
                ],
            }]
        } else if payment_date > accrual_date {
            // Capital contribution was made later, so record unpaid share
            // capital as a temporary asset.
            let account: Account = match while_unpaid {
                CommonStockWhileUnpaid::Asset => UNPAID_SHARE_CAPITAL_AS_ASSET.clone().into(),
                CommonStockWhileUnpaid::NegativeEquity => {
                    UNPAID_SHARE_CAPITAL_AS_EQUITY.clone().into()
                }
            };
            vec![
                Transaction {
                    spec_id: id,
                    date: accrual_date,
                    comment: Some("Unpaid share capital".into()),
                    postings: vec![
                        TransactionPosting::new(
                            s_handler.account().into(),
                            -amount.abs(),
                            commodity.currency()?,
                        ),
                        TransactionPosting::new(
                            account.clone(),
                            amount.abs(),
                            commodity.currency()?,
                        ),
                    ],
                },
                Transaction {
                    spec_id: id,
                    date: payment_date,
                    comment: Some("Share capital contribution".into()),
                    postings: vec![
                        TransactionPosting::linked(
                            account,
                            s_handler.account().into(),
                            -amount.abs(),
                            commodity.currency()?,
                        ),
                        TransactionPosting::new(
                            backing_account.account(),
                            amount.abs(),
                            commodity.currency()?,
                        ),
                    ],
                },
            ]
        } else {
            return Err(CommonStockCannotBePrepaid::new(&description));
        };

        Ok(Transformation {
            spec_id: id,
            label: TransactionLabel {
                payee: payee.name(),
                description,
            },
            expense_history_delta: None,
            reimbursement_state_delta: track_unreimbursed_entries(
                &backing_account,
                &transactions,
                &ext_transactions,
            )?,
            transactions,
            ext_transactions,
            ext_assertions,
            annotations,
        })
    }

    fn process_cost_of_equity(
        spec: DecoratedTransactionSpec<H>,
    ) -> Result<Transformation, ServerError> {
        let DecoratedTransactionSpec {
            id,
            accrual_start: accrual_date,
            accrual_end: None,
            payment_date,
            accounting_logic: AccountingLogic::CostOfEquity,
            payee,
            description,
            amount,
            commodity,
            backing_account,
            annotations,
            ext_transactions,
            ext_assertions,
        } = spec
        else {
            return Err(InvalidArgumentsForAccountingLogic::with_debug(&spec));
        };
        amount_should_be_negative!(amount, "CostOfEquity", &id);

        let transactions = if payment_date == accrual_date {
            // Record a single journal entry on the day of payment, since
            // accrual is immediate.
            vec![Transaction {
                spec_id: id,
                date: payment_date,
                comment: None,
                postings: vec![
                    TransactionPosting::new(
                        backing_account.account(),
                        -amount.abs(),
                        commodity.currency()?,
                    ),
                    TransactionPosting::new(
                        SHARE_ISSUANCE_COSTS.clone().into(),
                        amount.abs(),
                        commodity.currency()?,
                    ),
                ],
            }]
        } else if payment_date < accrual_date {
            // Record as prepaid, then clear on accrual.
            vec![
                Transaction {
                    spec_id: id,
                    date: payment_date,
                    comment: Some("Pre-paid share issuance costs".into()),
                    postings: vec![
                        TransactionPosting::new(
                            backing_account.account(),
                            -amount.abs(),
                            commodity.currency()?,
                        ),
                        TransactionPosting::linked(
                            PREPAID_SHARE_ISSUANCE_COSTS.clone().into(),
                            SHARE_ISSUANCE_COSTS.clone().into(),
                            amount.abs(),
                            commodity.currency()?,
                        ),
                    ],
                },
                Transaction {
                    spec_id: id,
                    date: accrual_date,
                    comment: Some("Accrue pre-paid share issuance costs".into()),
                    postings: vec![
                        TransactionPosting::new(
                            PREPAID_SHARE_ISSUANCE_COSTS.clone().into(),
                            -amount.abs(),
                            commodity.currency()?,
                        ),
                        TransactionPosting::new(
                            SHARE_ISSUANCE_COSTS.clone().into(),
                            amount.abs(),
                            commodity.currency()?,
                        ),
                    ],
                },
            ]
        } else {
            // Accrue as payable, then clear on payment.
            vec![
                Transaction {
                    spec_id: id,
                    date: accrual_date,
                    comment: Some("Accrue payable share issuance costs".into()),
                    postings: vec![
                        TransactionPosting::new(
                            SHARE_ISSUANCE_COSTS_PAYABLE.clone().into(),
                            -amount.abs(),
                            commodity.currency()?,
                        ),
                        TransactionPosting::new(
                            SHARE_ISSUANCE_COSTS.clone().into(),
                            amount.abs(),
                            commodity.currency()?,
                        ),
                    ],
                },
                Transaction {
                    spec_id: id,
                    date: payment_date,
                    comment: Some("Clear payable share issuance costs".into()),
                    postings: vec![
                        TransactionPosting::new(
                            backing_account.account(),
                            -amount.abs(),
                            commodity.currency()?,
                        ),
                        TransactionPosting::linked(
                            SHARE_ISSUANCE_COSTS_PAYABLE.clone().into(),
                            SHARE_ISSUANCE_COSTS.clone().into(),
                            amount.abs(),
                            commodity.currency()?,
                        ),
                    ],
                },
            ]
        };

        Ok(Transformation {
            spec_id: id,
            label: TransactionLabel {
                payee: payee.name(),
                description,
            },
            expense_history_delta: None,
            reimbursement_state_delta: track_unreimbursed_entries(
                &backing_account,
                &transactions,
                &ext_transactions,
            )?,
            transactions,
            ext_transactions,
            ext_assertions,
            annotations,
        })
    }

    fn process_simple_expense(
        spec: DecoratedTransactionSpec<H>,
    ) -> Result<Transformation, ServerError> {
        let DecoratedTransactionSpec {
            id,
            accrual_start: accrual_date,
            accrual_end: None,
            payment_date,
            accounting_logic: AccountingLogic::SimpleExpense(e_handler),
            payee,
            description,
            amount,
            commodity,
            backing_account,
            annotations,
            ext_transactions,
            ext_assertions,
        } = spec
        else {
            return Err(InvalidArgumentsForAccountingLogic::with_debug(&spec));
        };
        amount_should_be_negative!(amount, "SimpleExpense", &id);

        let transactions = if payment_date == accrual_date {
            // Record a single journal entry on the day of payment, since
            // accrual is immediate.
            vec![Transaction {
                spec_id: id,
                date: payment_date,
                comment: None,
                postings: vec![
                    TransactionPosting::new(
                        backing_account.account(),
                        -amount.abs(),
                        commodity.currency()?,
                    ),
                    TransactionPosting::new(
                        e_handler.account().into(),
                        amount.abs(),
                        commodity.currency()?,
                    ),
                ],
            }]
        } else if payment_date < accrual_date {
            // Record prepaid expense, then clear on accrual.
            vec![
                Transaction {
                    spec_id: id,
                    date: payment_date,
                    comment: Some("Pre-paid expense".into()),
                    postings: vec![
                        TransactionPosting::new(
                            backing_account.account(),
                            -amount.abs(),
                            commodity.currency()?,
                        ),
                        TransactionPosting::linked(
                            e_handler.while_prepaid().into(),
                            e_handler.account().into(),
                            amount.abs(),
                            commodity.currency()?,
                        ),
                    ],
                },
                Transaction {
                    spec_id: id,
                    date: accrual_date,
                    comment: Some("Accrue pre-paid expense".into()),
                    postings: vec![
                        TransactionPosting::new(
                            e_handler.while_prepaid().into(),
                            -amount.abs(),
                            commodity.currency()?,
                        ),
                        TransactionPosting::new(
                            e_handler.account().into(),
                            amount.abs(),
                            commodity.currency()?,
                        ),
                    ],
                },
            ]
        } else {
            // Accrue as payable, then clear on payment.
            vec![
                Transaction {
                    spec_id: id,
                    date: accrual_date,
                    comment: Some("Accrue payable expense".into()),
                    postings: vec![
                        TransactionPosting::new(
                            e_handler.while_payable().into(),
                            -amount.abs(),
                            commodity.currency()?,
                        ),
                        TransactionPosting::new(
                            e_handler.account().into(),
                            amount.abs(),
                            commodity.currency()?,
                        ),
                    ],
                },
                Transaction {
                    spec_id: id,
                    date: payment_date,
                    comment: Some("Clear payable expense".into()),
                    postings: vec![
                        TransactionPosting::new(
                            backing_account.account(),
                            -amount.abs(),
                            commodity.currency()?,
                        ),
                        TransactionPosting::linked(
                            e_handler.while_payable().into(),
                            e_handler.account().into(),
                            amount.abs(),
                            commodity.currency()?,
                        ),
                    ],
                },
            ]
        };

        Ok(Transformation {
            spec_id: id,
            label: TransactionLabel {
                payee: payee.name(),
                description,
            },
            expense_history_delta: None,
            reimbursement_state_delta: track_unreimbursed_entries(
                &backing_account,
                &transactions,
                &ext_transactions,
            )?,
            transactions,
            ext_transactions,
            ext_assertions,
            annotations,
        })
    }

    fn process_capitalize(
        spec: DecoratedTransactionSpec<H>,
    ) -> Result<Transformation, ServerError> {
        let DecoratedTransactionSpec {
            id,
            accrual_start: accrual_date,
            accrual_end: None,
            payment_date,
            accounting_logic: AccountingLogic::Capitalize(a_handler),
            payee,
            description,
            amount,
            commodity,
            backing_account,
            annotations,
            ext_transactions,
            ext_assertions,
        } = spec
        else {
            return Err(InvalidArgumentsForAccountingLogic::with_debug(&spec));
        };
        amount_should_be_negative!(amount, "Capitalize", &id);

        let transactions = if payment_date == accrual_date {
            // Record a single journal entry on the day of payment, since
            // accrual is immediate.
            vec![Transaction {
                spec_id: id,
                date: payment_date,
                comment: None,
                postings: vec![
                    TransactionPosting::new(
                        backing_account.account(),
                        -amount.abs(),
                        commodity.currency()?,
                    ),
                    TransactionPosting::new(
                        a_handler.account().into(),
                        amount.abs(),
                        commodity.currency()?,
                    ),
                ],
            }]
        } else if payment_date < accrual_date {
            // Record prepaid asset, then clear on accrual.
            let mut ts = vec![Transaction {
                spec_id: id,
                date: payment_date,
                comment: Some("Pre-paid asset".into()),
                postings: vec![
                    TransactionPosting::new(
                        backing_account.account(),
                        -amount.abs(),
                        commodity.currency()?,
                    ),
                    // At this point, it's not linked, since the cash
                    // transaction is to pre-pay, and can't yet be
                    // considered cash for acquisition of the final
                    // asset.
                    //
                    // As a result, regardless of asset type, this cash
                    // flow will appear in the operating activities on
                    // the cash flow statement (until reclassification).
                    TransactionPosting::new(
                        a_handler.while_prepaid().into(),
                        amount.abs(),
                        commodity.currency()?,
                    ),
                ],
            }];
            if a_handler.account() != a_handler.while_prepaid() {
                ts.push(Transaction {
                    spec_id: id,
                    date: accrual_date,
                    comment: Some("Reclassify pre-paid asset".into()),
                    postings: vec![
                        TransactionPosting::new(
                            a_handler.while_prepaid().into(),
                            -amount.abs(),
                            commodity.currency()?,
                        ),
                        TransactionPosting::non_cash_reclassification(
                            a_handler.account().into(),
                            amount.abs(),
                            commodity.currency()?,
                        ),
                    ],
                });
            }
            ts
        } else {
            // Accrue as payable, then clear on payment.
            vec![
                Transaction {
                    spec_id: id,
                    date: accrual_date,
                    comment: Some("Accrue payable asset".into()),
                    postings: vec![
                        TransactionPosting::new(
                            a_handler.while_payable().into(),
                            -amount.abs(),
                            commodity.currency()?,
                        ),
                        TransactionPosting::new(
                            a_handler.account().into(),
                            amount.abs(),
                            commodity.currency()?,
                        ),
                    ],
                },
                Transaction {
                    spec_id: id,
                    date: payment_date,
                    comment: Some("Clear payable asset".into()),
                    postings: vec![
                        TransactionPosting::new(
                            backing_account.account(),
                            -amount.abs(),
                            commodity.currency()?,
                        ),
                        TransactionPosting::linked(
                            a_handler.while_payable().into(),
                            a_handler.account().into(),
                            amount.abs(),
                            commodity.currency()?,
                        ),
                    ],
                },
            ]
        };

        Ok(Transformation {
            spec_id: id,
            label: TransactionLabel {
                payee: payee.name(),
                description,
            },
            expense_history_delta: None,
            reimbursement_state_delta: track_unreimbursed_entries(
                &backing_account,
                &transactions,
                &ext_transactions,
            )?,
            transactions,
            ext_transactions,
            ext_assertions,
            annotations,
        })
    }

    fn process_amortize(spec: DecoratedTransactionSpec<H>) -> Result<Transformation, ServerError> {
        let DecoratedTransactionSpec {
            id,
            accrual_start,
            accrual_end: Some(accrual_end),
            payment_date,
            accounting_logic: AccountingLogic::Amortize(a_handler),
            payee,
            description,
            amount,
            commodity,
            backing_account,
            annotations,
            ext_transactions,
            ext_assertions,
        } = spec
        else {
            return Err(InvalidArgumentsForAccountingLogic::with_debug(&spec));
        };
        amount_should_be_negative!(amount, "Amortize", &id);

        let mut transactions = Vec::new();

        // Record the capitalization.
        let cap_transformation = Self::process_capitalize(DecoratedTransactionSpec {
            id,
            accrual_start,
            accrual_end: None,
            payment_date,
            accounting_logic: AccountingLogic::Capitalize(a_handler.clone()),
            payee: payee.clone(),
            description: description.clone(),
            amount,
            commodity: commodity.clone(),
            backing_account: backing_account.clone(),
            annotations: annotations.clone(),
            ext_transactions: Default::default(),
            ext_assertions: Default::default(),
        })?;
        transactions.extend(cap_transformation.transactions);

        let accrual_account = a_handler
            .upon_accrual()
            .ok_or_else(|| NonAmortizableAsset::new(&description))?;

        // Record the monthly amortization adjustments.
        for MonthlyAccrualAdjustment {
            period_start,
            period_end,
            adjustment_amount: monthly_amort,
            adjustment_date,
        } in monthly_accrual_adjustments(
            accrual_start,
            accrual_end,
            amount.abs(),
            commodity.currency()?,
        )? {
            transactions.push(Transaction {
                spec_id: id,
                date: adjustment_date,
                comment: Some(format!(
                    "Amortization adjustment for {} - {}",
                    period_start, period_end
                )),
                postings: vec![
                    TransactionPosting::new(
                        a_handler.account().into(),
                        -monthly_amort,
                        commodity.currency()?,
                    ),
                    TransactionPosting::new(
                        accrual_account.clone().into(),
                        monthly_amort,
                        commodity.currency()?,
                    ),
                ],
            });
        }

        Ok(Transformation {
            spec_id: id,
            label: TransactionLabel {
                payee: payee.name(),
                description,
            },
            expense_history_delta: None,
            reimbursement_state_delta: track_unreimbursed_entries(
                &backing_account,
                &transactions,
                &ext_transactions,
            )?,
            transactions,
            ext_transactions,
            ext_assertions,
            annotations,
        })
    }

    fn process_fixed_expense(
        spec: DecoratedTransactionSpec<H>,
    ) -> Result<Transformation, ServerError> {
        let DecoratedTransactionSpec {
            id,
            accrual_start,
            accrual_end: Some(accrual_end),
            payment_date,
            accounting_logic: AccountingLogic::FixedExpense(e_handler),
            payee,
            description,
            amount,
            commodity,
            backing_account,
            annotations,
            ext_transactions,
            ext_assertions,
        } = spec
        else {
            return Err(InvalidArgumentsForAccountingLogic::with_debug(&spec));
        };
        amount_should_be_negative!(amount, "FixedExpense", &id);

        let mut transactions = Vec::new();
        let mut payable_sum = 0.0;
        let mut prepaid_sum = 0.0;
        for MonthlyAccrualAdjustment {
            period_start,
            period_end,
            adjustment_amount: period_expense,
            adjustment_date,
        } in monthly_accrual_adjustments(
            accrual_start,
            accrual_end,
            amount.abs(),
            commodity.currency()?,
        )? {
            if adjustment_date <= payment_date {
                // Record accrual adjustment as payable, since the clearing
                // transaction will occur in the future.
                transactions.push(Transaction {
                    spec_id: id,
                    date: adjustment_date,
                    comment: Some(format!(
                        "Accrue fixed expense for {} - {}",
                        period_start, period_end
                    )),
                    postings: vec![
                        TransactionPosting::new(
                            e_handler.while_payable().into(),
                            -period_expense,
                            commodity.currency()?,
                        ),
                        TransactionPosting::new(
                            e_handler.account().into(),
                            period_expense,
                            commodity.currency()?,
                        ),
                    ],
                });
                payable_sum += period_expense;
            } else {
                // Record accrual adjustment as prepaid, since the clearing
                // transaction will occur in the past.
                transactions.push(Transaction {
                    spec_id: id,
                    date: adjustment_date,
                    comment: Some(format!(
                        "Accrue fixed expense for {} - {}",
                        period_start, period_end
                    )),
                    postings: vec![
                        TransactionPosting::new(
                            e_handler.while_prepaid().into(),
                            -period_expense,
                            commodity.currency()?,
                        ),
                        TransactionPosting::new(
                            e_handler.account().into(),
                            period_expense,
                            commodity.currency()?,
                        ),
                    ],
                });
                prepaid_sum += period_expense;
            }
        }

        // Record prepaid / payable amounts on payment date.
        //
        // This is kind of confusing, but can think of this clearing transaction
        // as happening anywhere before, during or after the accrual period --
        // so this transaction may be before, in between, or after the
        // transactions we added above. On this clearing date, we need to clear
        // the payables already accrued up to that point, and pour the remainder
        // into prepaid account (to be accrued in the later transactions).
        transactions.push(Transaction {
            spec_id: id,
            date: payment_date,
            comment: Some("Clear / pre-pay fixed expense".into()),
            postings: vec![
                TransactionPosting::new(
                    backing_account.account(),
                    -amount.abs(),
                    commodity.currency()?,
                ),
                TransactionPosting::linked(
                    e_handler.while_prepaid().into(),
                    e_handler.account().into(),
                    prepaid_sum,
                    commodity.currency()?,
                ),
                TransactionPosting::linked(
                    e_handler.while_payable().into(),
                    e_handler.account().into(),
                    -payable_sum,
                    commodity.currency()?,
                ),
            ],
        });

        Ok(Transformation {
            spec_id: id,
            label: TransactionLabel {
                payee: payee.name(),
                description,
            },
            expense_history_delta: None,
            reimbursement_state_delta: track_unreimbursed_entries(
                &backing_account,
                &transactions,
                &ext_transactions,
            )?,
            transactions,
            ext_transactions,
            ext_assertions,
            annotations,
        })
    }

    /// Initiates variable expense calculations with a manual estimate.
    fn process_variable_expense_init(
        spec: DecoratedTransactionSpec<H>,
    ) -> Result<Transformation, ServerError> {
        let DecoratedTransactionSpec {
            accrual_start,
            accrual_end: Some(accrual_end),
            accounting_logic:
                AccountingLogic::VariableExpenseInit {
                    account: ref e_handler,
                    estimate,
                },
            ..
        } = spec
        else {
            return Err(InvalidArgumentsForAccountingLogic::with_debug(&spec));
        };

        let accrual_days = (accrual_end - accrual_start).num_days() + 1;
        let init_daily = (estimate.abs() as f64) / (accrual_days as f64);

        let e_handler = e_handler.clone();
        Self::process_variable_expense_helper(spec, e_handler, init_daily, true)
    }

    /// Uses the past 90 days of historical data (prior to accrual date) to
    /// compute the daily average. If VariableExpenseInit is less than 90 days
    /// prior, the window instead starts at the init date. If there is not
    /// enough historical data, it returns an error.
    ///
    /// NOTE: May go without saying, but the order of transaction specs
    /// therefore matters.
    fn process_variable_expense(
        spec: DecoratedTransactionSpec<H>,
        history_lookup: &HashMap<ExpenseAccount, ExpenseHistory>,
    ) -> Result<Transformation, ServerError> {
        let DecoratedTransactionSpec {
            accrual_start,
            accounting_logic: AccountingLogic::VariableExpense(ref e_handler),
            ref description,
            ..
        } = spec
        else {
            return Err(InvalidArgumentsForAccountingLogic::with_debug(&spec));
        };

        // Use the last 90 days of history before the accrual_date.
        let expense_account = e_handler.account();
        let records = history_lookup
            .get(&expense_account)
            .map(|v| v.price_records.as_slice())
            .unwrap_or(&[]);
        let Some(history_init_date) = history_lookup
            .get(&expense_account)
            .and_then(|v| v.init_date)
        else {
            return Err(VariableExpenseNoInit::new(description));
        };
        let history_window_start =
            std::cmp::max(history_init_date, accrual_start - Duration::days(90));
        let history_window_end = accrual_start - Duration::days(1);

        // Compute the average daily accrual rate over the 90-day window.
        let daily_rate = compute_daily_average(records, history_window_start, history_window_end)
            .ok_or_else(|| VariableExpenseNotEnoughHistoricalData::new(description))?;

        let e_handler = e_handler.clone();
        Self::process_variable_expense_helper(spec, e_handler, daily_rate, false)
    }

    fn process_variable_expense_helper(
        spec: DecoratedTransactionSpec<H>,
        e_handler: H::E,
        estimated_daily_rate: f64,
        is_init: bool,
    ) -> Result<Transformation, ServerError> {
        let DecoratedTransactionSpec {
            id,
            accrual_start,
            accrual_end: Some(accrual_end),
            payment_date,
            accounting_logic: _,
            payee,
            description,
            amount,
            commodity,
            backing_account,
            annotations,
            ext_transactions,
            ext_assertions,
        } = spec
        else {
            return Err(InvalidArgumentsForAccountingLogic::with_debug(&spec));
        };
        amount_should_be_negative!(amount, "VariableExpense", &id);

        if payment_date <= accrual_end {
            return Err(VariableExpenseInvalidPaymentDate::new(
                &description,
                &payment_date,
                &accrual_end,
            ));
        }

        let currency = commodity.currency()?;
        let accrual_days = (accrual_end - accrual_start).num_days() + 1;
        let estimated_total = estimated_daily_rate * (accrual_days as f64);
        let actual_daily_rate = amount.abs() / (accrual_days as f64);

        // Break into monthly accrual periods.
        let mut transactions = Vec::new();
        for MonthlyAccrualAdjustment {
            period_start,
            period_end,
            adjustment_amount: period_estimate,
            adjustment_date,
        } in monthly_accrual_adjustments(accrual_start, accrual_end, estimated_total, currency)?
        {
            transactions.push(Transaction {
                spec_id: id,
                date: adjustment_date,
                comment: Some(format!(
                    "Estimated expense accrual for {} - {}",
                    period_start, period_end
                )),
                postings: vec![
                    TransactionPosting::new(
                        e_handler.while_payable().into(),
                        -period_estimate,
                        currency,
                    ),
                    TransactionPosting::new(e_handler.account().into(), period_estimate, currency),
                ],
            });
        }

        // Record any estimation discrepancies.
        //
        // Note, to ensure we don't have lingering pennies, the discrepancy must
        // be calculated at the precision level of the currency.
        let discrepancy = round_to_currency_precision(amount.abs(), &currency)?
            - round_to_currency_precision(estimated_total, &currency)?;
        if discrepancy.abs() >= commodity.precision_cutoff()? {
            transactions.push(Transaction {
                spec_id: id,
                date: payment_date,
                comment: Some("Correct estimate discrepancy".into()),
                postings: vec![
                    TransactionPosting::new(
                        e_handler.while_payable().into(),
                        -discrepancy,
                        currency,
                    ),
                    TransactionPosting::new(e_handler.account().into(), discrepancy, currency),
                ],
            });
        }

        // Record the clearing transaction.
        transactions.push(Transaction {
            spec_id: id,
            date: payment_date,
            comment: Some("Clear payable expense".into()),
            postings: vec![
                TransactionPosting::new(backing_account.account(), -amount.abs(), currency),
                TransactionPosting::linked(
                    e_handler.while_payable().into(),
                    e_handler.account().into(),
                    amount.abs(),
                    currency,
                ),
            ],
        });

        // Record this variable expense’s daily rate for future history.
        let expense_history_delta = ExpenseHistoryDelta {
            account: e_handler.account(),
            price_record: ExpenseHistoryPriceRecord {
                start: accrual_start,
                end: accrual_end,
                daily_rate: actual_daily_rate,
            },
            is_init,
        };

        // Tag this transaction, since the accounting logic deserves a note in
        // the financial records.
        let note = Annotation::VariableExpense;

        Ok(Transformation {
            spec_id: id,
            label: TransactionLabel {
                payee: payee.name(),
                description,
            },
            expense_history_delta: Some(expense_history_delta),
            reimbursement_state_delta: track_unreimbursed_entries(
                &backing_account,
                &transactions,
                &ext_transactions,
            )?,
            transactions,
            ext_transactions,
            ext_assertions,
            annotations: annotations.into_iter().chain(once(note)).collect(),
        })
    }

    fn process_immaterial_income(
        spec: DecoratedTransactionSpec<H>,
    ) -> Result<Transformation, ServerError> {
        let DecoratedTransactionSpec {
            id,
            accrual_start: _, // Ignored.
            accrual_end: _,   // Ignored.
            payment_date,
            accounting_logic: AccountingLogic::ImmaterialIncome(i_handler),
            payee,
            description,
            amount,
            commodity,
            backing_account,
            annotations,
            ext_transactions,
            ext_assertions,
        } = spec
        else {
            return Err(InvalidArgumentsForAccountingLogic::with_debug(&spec));
        };
        amount_should_be_positive!(amount, "ImmaterialIncome", &id);

        let tx = Transaction {
            spec_id: id,
            date: payment_date,
            comment: None,
            postings: vec![
                TransactionPosting::new(
                    i_handler.account().into(),
                    -amount.abs(),
                    commodity.currency()?,
                ),
                TransactionPosting::new(
                    backing_account.account(),
                    amount.abs(),
                    commodity.currency()?,
                ),
            ],
        };

        // Tag this transaction, since the accounting logic deserves a note in
        // the financial records.
        let note = Annotation::ImmaterialIncome;

        Ok(Transformation {
            spec_id: id,
            label: TransactionLabel {
                payee: payee.name(),
                description,
            },
            transactions: vec![tx],
            ext_transactions,
            ext_assertions,
            expense_history_delta: None,
            reimbursement_state_delta: None,
            annotations: annotations.into_iter().chain(once(note)).collect(),
        })
    }

    fn process_immaterial_expense(
        spec: DecoratedTransactionSpec<H>,
    ) -> Result<Transformation, ServerError> {
        let DecoratedTransactionSpec {
            id,
            accrual_start: _, // Ignored.
            accrual_end: _,   // Ignored.
            payment_date,
            accounting_logic: AccountingLogic::ImmaterialExpense(e_handler),
            payee,
            description,
            amount,
            commodity,
            backing_account,
            annotations,
            ext_transactions,
            ext_assertions,
        } = spec
        else {
            return Err(InvalidArgumentsForAccountingLogic::with_debug(&spec));
        };
        amount_should_be_negative!(amount, "ImmaterialExpense", &id);

        let tx = Transaction {
            spec_id: id,
            date: payment_date,
            comment: None,
            postings: vec![
                TransactionPosting::new(
                    backing_account.account(),
                    -amount.abs(),
                    commodity.currency()?,
                ),
                TransactionPosting::new(
                    e_handler.account().into(),
                    amount.abs(),
                    commodity.currency()?,
                ),
            ],
        };

        // Tag this transaction, since the accounting logic deserves a note in
        // the financial records.
        let note = Annotation::ImmaterialExpense;

        let transactions = vec![tx];
        Ok(Transformation {
            spec_id: id,
            label: TransactionLabel {
                payee: payee.name(),
                description,
            },
            expense_history_delta: None,
            reimbursement_state_delta: track_unreimbursed_entries(
                &backing_account,
                &transactions,
                &ext_transactions,
            )?,
            transactions,
            ext_transactions,
            ext_assertions,
            annotations: annotations.into_iter().chain(once(note)).collect(),
        })
    }

    fn process_reimburse(
        spec: DecoratedTransactionSpec<H>,
        reimbursement_state: &ReimbursementState,
    ) -> Result<Transformation, ServerError> {
        let AccountingLogic::Reimburse(ref r_handler) = spec.accounting_logic else {
            return Err(InvalidArgumentsForAccountingLogic::with_debug(&spec));
        };
        let r_account = r_handler.account();
        Self::process_reimburse_helper(spec, r_account, false, reimbursement_state)
    }

    fn process_reimburse_partial(
        spec: DecoratedTransactionSpec<H>,
        reimbursement_state: &ReimbursementState,
    ) -> Result<Transformation, ServerError> {
        let AccountingLogic::ReimbursePartial(ref r_handler) = spec.accounting_logic else {
            return Err(InvalidArgumentsForAccountingLogic::with_debug(&spec));
        };
        let r_account = r_handler.account();
        Self::process_reimburse_helper(spec, r_account, true, reimbursement_state)
    }

    fn process_reimburse_helper(
        spec: DecoratedTransactionSpec<H>,
        r_account: LiabilityAccount,
        expect_remaining: bool,
        reimbursement_state: &ReimbursementState,
    ) -> Result<Transformation, ServerError> {
        let DecoratedTransactionSpec {
            id,
            accrual_start: _,
            accrual_end: None,
            payment_date,
            accounting_logic: _,
            payee,
            description,
            amount,
            commodity,
            backing_account: BackingAccount::Cash(c_handler),
            annotations,
            ext_transactions,
            ext_assertions,
        } = spec
        else {
            return Err(InvalidArgumentsForAccountingLogic::with_debug(&spec));
        };
        amount_should_be_negative!(amount, "Reimburse", &id);

        let (reimbursement_entries, unreimbursed_remaining) = reimbursement_state
            .get(&r_account)
            .map(|v| v.peak_until_exactly(amount.abs(), payment_date))
            .transpose()?
            .ok_or_else(|| NoTransactionsToReimburse::new(&id, &r_account))?;

        if !expect_remaining && unreimbursed_remaining.abs() >= commodity.precision_cutoff()? {
            return Err(UnexpectedPartialReimbursement::new(
                &id,
                &r_account,
                unreimbursed_remaining,
            ));
        }

        let linked_postings = reimbursement_entries
            .into_iter()
            .flat_map(|e| e.credit_postings.iter())
            .map(|p| {
                TransactionPosting::linked(
                    r_account.clone().into(),
                    p.source_account.clone().unwrap_or(p.account.clone()),
                    p.amount.abs(),
                    p.currency,
                )
            })
            .collect::<Vec<_>>();

        let tx = Transaction {
            spec_id: id,
            date: payment_date,
            comment: None,
            postings: once(TransactionPosting::new(
                c_handler.account().into(),
                -amount.abs(),
                commodity.currency()?,
            ))
            .chain(linked_postings)
            .collect(),
        };
        let assrt = Assertion {
            date: payment_date,
            account: r_account.clone().into(),
            balance: if expect_remaining {
                -unreimbursed_remaining.abs()
            } else {
                0.0
            },
            currency: commodity.currency()?,
        };

        Ok(Transformation {
            spec_id: id,
            label: TransactionLabel {
                payee: payee.name(),
                description,
            },
            transactions: vec![tx],
            ext_transactions,
            ext_assertions: ext_assertions.into_iter().chain(once(assrt)).collect(),
            expense_history_delta: None,
            reimbursement_state_delta: Some(ReimbursementStateDelta::Pop {
                date: payment_date,
                account: r_account,
                amount: amount.abs(),
            }),
            annotations,
        })
    }

    fn process_clear_vat(spec: DecoratedTransactionSpec<H>) -> Result<Transformation, ServerError> {
        let DecoratedTransactionSpec {
            id,
            accrual_start: accrual_date,
            accrual_end: None,
            payment_date: _,
            accounting_logic: AccountingLogic::ClearVat { from, to },
            payee,
            description,
            amount,
            commodity,
            backing_account: BackingAccount::Cash(cash),
            annotations,
            ext_transactions,
            ext_assertions,
        } = spec
        else {
            return Err(InvalidArgumentsForAccountingLogic::with_debug(&spec));
        };

        let tx = if amount > 0.0 {
            Transaction {
                spec_id: id,
                date: accrual_date,
                comment: Some(format!("Clear VAT receivable for {} - {}", from, to)),
                postings: vec![
                    TransactionPosting::new(
                        VAT_RECEIVABLE.clone().into(),
                        -amount.abs(),
                        commodity.currency()?,
                    ),
                    TransactionPosting::new(
                        cash.account().into(),
                        amount.abs(),
                        commodity.currency()?,
                    ),
                ],
            }
        } else {
            Transaction {
                spec_id: id,
                date: accrual_date,
                comment: Some(format!("Clear VAT payable for {} - {}", from, to)),
                postings: vec![
                    TransactionPosting::new(
                        cash.account().into(),
                        -amount.abs(),
                        commodity.currency()?,
                    ),
                    TransactionPosting::new(
                        VAT_PAYABLE.clone().into(),
                        amount.abs(),
                        commodity.currency()?,
                    ),
                ],
            }
        };
        let assrt = if amount > 0.0 {
            vec![
                Assertion {
                    date: to,
                    account: VAT_RECEIVABLE.clone().into(),
                    balance: amount.abs(),
                    currency: commodity.currency()?,
                },
                Assertion {
                    date: to,
                    account: VAT_PAYABLE.clone().into(),
                    balance: 0.0,
                    currency: commodity.currency()?,
                },
            ]
        } else {
            vec![
                Assertion {
                    date: to,
                    account: VAT_RECEIVABLE.clone().into(),
                    balance: 0.0,
                    currency: commodity.currency()?,
                },
                Assertion {
                    date: to,
                    account: VAT_PAYABLE.clone().into(),
                    balance: -amount.abs(),
                    currency: commodity.currency()?,
                },
            ]
        };

        Ok(Transformation {
            spec_id: id,
            label: TransactionLabel {
                payee: payee.name(),
                description,
            },
            transactions: vec![tx],
            ext_transactions,
            ext_assertions: ext_assertions.into_iter().chain(assrt).collect(),
            expense_history_delta: None,
            reimbursement_state_delta: None,
            annotations,
        })
    }
}
