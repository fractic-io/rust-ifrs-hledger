use std::collections::HashMap;

use chrono::{Duration, NaiveDate};
use fractic_server_error::ServerError;

use crate::{
    domain::logic::utils::{compute_daily_average, monthly_accrual_periods, MonthlyAccrualPeriod},
    entities::{
        Account, AccountingLogic, Annotation, Assertion, AssetHandler, BackingAccount, CashHandler,
        CommodityHandler, DecoratedFinancialRecordSpecs, DecoratedTransactionSpec, ExpenseAccount,
        ExpenseHandler, FinancialRecords, Handlers, IncomeHandler, PayeeHandler,
        ReimbursableEntityHandler, Transaction, TransactionLabel, TransactionPosting,
        TransactionSpecId,
    },
    errors::{
        InvalidArgumentsForAccountingLogic, NonAmortizableAsset, UnexpectedNegativeValue,
        UnexpectedPositiveValue, VariableExpenseInvalidPaymentDate,
        VariableExpenseNoHistoricalData,
    },
    impl_ext::standard_accounts::vat::{VAT_PAYABLE, VAT_RECEIVABLE},
};

pub(crate) struct SpecProcessor<H: Handlers> {
    specs: DecoratedFinancialRecordSpecs<H>,
}

/// A record that stores the daily accrual rate for a given period.
/// (Note: We use the accrual period [start, end] – the clearing date is handled in the transactions.)
#[derive(Clone, Debug)]
pub struct VariableExpenseRecord {
    pub start: NaiveDate,
    pub end: NaiveDate,
    pub daily_rate: f64,
}

struct Transformation {
    spec_id: TransactionSpecId,
    label: TransactionLabel,
    transactions: Vec<Transaction>,
    side_transactions: Vec<Transaction>,
    expense_history_delta: Option<(ExpenseAccount, VariableExpenseRecord)>,
    annotations: Vec<Annotation>,
}

struct FoldState {
    accounts: Vec<Account>,
    transactions: Vec<Transaction>,
    expense_history: HashMap<ExpenseAccount, Vec<VariableExpenseRecord>>,
    label_lookup: HashMap<TransactionSpecId, TransactionLabel>,
    annotations_lookup: HashMap<TransactionSpecId, Vec<Annotation>>,
}

impl FoldState {
    fn new() -> Self {
        Self {
            accounts: Vec::new(),
            transactions: Vec::new(),
            expense_history: HashMap::new(),
            label_lookup: HashMap::new(),
            annotations_lookup: HashMap::new(),
        }
    }

    /// Update current state with the given transformation.
    fn step(self, t: Transformation) -> Self {
        let mut accounts = self.accounts;
        let mut transactions = self.transactions;
        let mut expense_history = self.expense_history;
        let mut label_lookup = self.label_lookup;
        let mut annotations_lookup = self.annotations_lookup;

        accounts.extend(
            t.transactions
                .iter()
                .flat_map(|t| t.postings.iter().map(|p| p.account.clone())),
        );
        transactions.extend(t.transactions);
        transactions.extend(t.side_transactions);
        if let Some((account, record)) = t.expense_history_delta {
            expense_history.entry(account).or_default().push(record);
        }
        label_lookup.insert(t.spec_id, t.label);
        annotations_lookup
            .entry(t.spec_id)
            .or_default()
            .extend(t.annotations);

        Self {
            accounts,
            transactions,
            expense_history,
            label_lookup,
            annotations_lookup,
        }
    }
}

impl<R, C> BackingAccount<R, C>
where
    R: ReimbursableEntityHandler,
    C: CashHandler,
{
    fn account(&self) -> Account {
        match self {
            BackingAccount::Reimburse(r) => r.account().into(),
            BackingAccount::Cash(c) => c.account().into(),
        }
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
            transaction_specs,
            assertion_specs,
        } = self.specs;

        let transactions_fold_result =
            transaction_specs
                .into_iter()
                .try_fold(FoldState::new(), |state, spec| {
                    let transformation = match &spec.accounting_logic {
                        AccountingLogic::SimpleExpense(..) => Self::process_simple_expense(spec)?,
                        AccountingLogic::Capitalize(..) => Self::process_capitalize(spec)?,
                        AccountingLogic::Amortize(..) => Self::process_amortize(spec)?,
                        AccountingLogic::FixedExpense(..) => Self::process_fixed_expense(spec)?,
                        AccountingLogic::VariableExpenseInit { .. } => {
                            Self::process_variable_expense_init(spec)?
                        }
                        AccountingLogic::VariableExpense(..) => {
                            Self::process_variable_expense(spec, &state.expense_history)?
                        }
                        AccountingLogic::ImmaterialIncome(..) => {
                            Self::process_immaterial_income(spec)?
                        }
                        AccountingLogic::ImmaterialExpense(..) => {
                            Self::process_immaterial_expense(spec)?
                        }
                        AccountingLogic::Reimburse(..) => Self::process_reimburse(spec)?,
                        AccountingLogic::ClearVat { .. } => Self::process_clear_vat(spec)?,
                    };
                    Ok(state.step(transformation))
                })?;

        let assertions = assertion_specs
            .into_iter()
            .map(|spec| Assertion {
                date: spec.date,
                account: spec.cash_handler.account().into(),
                balance: spec.balance,
                currency: spec.commodity.iso_symbol(),
            })
            .collect();

        Ok(FinancialRecords {
            accounts: transactions_fold_result.accounts,
            transactions: transactions_fold_result.transactions,
            assertions,
            label_lookup: transactions_fold_result.label_lookup,
            annotations_lookup: transactions_fold_result.annotations_lookup,
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
            side_transactions,
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
                    TransactionPosting {
                        account: backing_account.account(),
                        amount: -amount.abs(),
                        currency: commodity.iso_symbol(),
                    },
                    TransactionPosting {
                        account: e_handler.account().into(),
                        amount: amount.abs(),
                        currency: commodity.iso_symbol(),
                    },
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
                        TransactionPosting {
                            account: backing_account.account(),
                            amount: -amount.abs(),
                            currency: commodity.iso_symbol(),
                        },
                        TransactionPosting {
                            account: e_handler.while_prepaid().into(),
                            amount: amount.abs(),
                            currency: commodity.iso_symbol(),
                        },
                    ],
                },
                Transaction {
                    spec_id: id,
                    date: accrual_date,
                    comment: Some("Accrue pre-paid expense".into()),
                    postings: vec![
                        TransactionPosting {
                            account: e_handler.while_prepaid().into(),
                            amount: -amount.abs(),
                            currency: commodity.iso_symbol(),
                        },
                        TransactionPosting {
                            account: e_handler.account().into(),
                            amount: amount.abs(),
                            currency: commodity.iso_symbol(),
                        },
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
                        TransactionPosting {
                            account: e_handler.while_payable().into(),
                            amount: -amount.abs(),
                            currency: commodity.iso_symbol(),
                        },
                        TransactionPosting {
                            account: e_handler.account().into(),
                            amount: amount.abs(),
                            currency: commodity.iso_symbol(),
                        },
                    ],
                },
                Transaction {
                    spec_id: id,
                    date: payment_date,
                    comment: Some("Clear payable expense".into()),
                    postings: vec![
                        TransactionPosting {
                            account: backing_account.account(),
                            amount: -amount.abs(),
                            currency: commodity.iso_symbol(),
                        },
                        TransactionPosting {
                            account: e_handler.while_payable().into(),
                            amount: amount.abs(),
                            currency: commodity.iso_symbol(),
                        },
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
            transactions,
            side_transactions,
            expense_history_delta: None,
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
            side_transactions,
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
                    TransactionPosting {
                        account: backing_account.account(),
                        amount: -amount.abs(),
                        currency: commodity.iso_symbol(),
                    },
                    TransactionPosting {
                        account: a_handler.account().into(),
                        amount: amount.abs(),
                        currency: commodity.iso_symbol(),
                    },
                ],
            }]
        } else if payment_date < accrual_date {
            // Record prepaid asset, then clear on accrual.
            vec![
                Transaction {
                    spec_id: id,
                    date: payment_date,
                    comment: Some("Pre-paid asset".into()),
                    postings: vec![
                        TransactionPosting {
                            account: backing_account.account(),
                            amount: -amount.abs(),
                            currency: commodity.iso_symbol(),
                        },
                        TransactionPosting {
                            account: a_handler.while_prepaid().into(),
                            amount: amount.abs(),
                            currency: commodity.iso_symbol(),
                        },
                    ],
                },
                Transaction {
                    spec_id: id,
                    date: accrual_date,
                    comment: Some("Accrue pre-paid asset".into()),
                    postings: vec![
                        TransactionPosting {
                            account: a_handler.while_prepaid().into(),
                            amount: -amount.abs(),
                            currency: commodity.iso_symbol(),
                        },
                        TransactionPosting {
                            account: a_handler.account().into(),
                            amount: amount.abs(),
                            currency: commodity.iso_symbol(),
                        },
                    ],
                },
            ]
        } else {
            // Accrue as payable, then clear on payment.
            vec![
                Transaction {
                    spec_id: id,
                    date: accrual_date,
                    comment: Some("Accrue payable asset".into()),
                    postings: vec![
                        TransactionPosting {
                            account: a_handler.while_payable().into(),
                            amount: -amount.abs(),
                            currency: commodity.iso_symbol(),
                        },
                        TransactionPosting {
                            account: a_handler.account().into(),
                            amount: amount.abs(),
                            currency: commodity.iso_symbol(),
                        },
                    ],
                },
                Transaction {
                    spec_id: id,
                    date: payment_date,
                    comment: Some("Clear payable asset".into()),
                    postings: vec![
                        TransactionPosting {
                            account: backing_account.account(),
                            amount: -amount.abs(),
                            currency: commodity.iso_symbol(),
                        },
                        TransactionPosting {
                            account: a_handler.while_payable().into(),
                            amount: amount.abs(),
                            currency: commodity.iso_symbol(),
                        },
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
            transactions,
            side_transactions,
            expense_history_delta: None,
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
            side_transactions,
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
            backing_account,
            annotations: annotations.clone(),
            side_transactions: Default::default(),
        })?;
        transactions.extend(cap_transformation.transactions);

        let accrual_account = a_handler
            .upon_accrual()
            .ok_or_else(|| NonAmortizableAsset::new(&description))?;

        // Record the monthly amortization adjustments.
        let total_days = (accrual_end - accrual_start).num_days() + 1;
        let daily_amort = amount / (total_days as f64);
        for MonthlyAccrualPeriod {
            period_start,
            period_end,
            num_days,
            adjustment_date,
        } in monthly_accrual_periods(accrual_start, accrual_end)?
        {
            let monthly_amort = daily_amort * (num_days as f64);
            transactions.push(Transaction {
                spec_id: id,
                date: adjustment_date,
                comment: Some(format!(
                    "Amortization adjustment for {} - {}",
                    period_start, period_end
                )),
                postings: vec![
                    TransactionPosting {
                        account: a_handler.account().into(),
                        amount: -monthly_amort,
                        currency: commodity.iso_symbol(),
                    },
                    TransactionPosting {
                        account: accrual_account.clone().into(),
                        amount: monthly_amort,
                        currency: commodity.iso_symbol(),
                    },
                ],
            });
        }

        Ok(Transformation {
            spec_id: id,
            label: TransactionLabel {
                payee: payee.name(),
                description,
            },
            transactions,
            side_transactions,
            expense_history_delta: None,
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
            side_transactions,
        } = spec
        else {
            return Err(InvalidArgumentsForAccountingLogic::with_debug(&spec));
        };
        amount_should_be_negative!(amount, "FixedExpense", &id);

        let total_days = (accrual_end - accrual_start).num_days() + 1;
        let daily_expense = amount.abs() / (total_days as f64);
        let mut transactions = Vec::new();
        let mut payable_sum = 0.0;
        let mut prepaid_sum = 0.0;
        for MonthlyAccrualPeriod {
            period_start,
            period_end,
            num_days,
            adjustment_date,
        } in monthly_accrual_periods(accrual_start, accrual_end)?
        {
            let period_expense = daily_expense * (num_days as f64);
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
                        TransactionPosting {
                            account: e_handler.while_payable().into(),
                            amount: -period_expense,
                            currency: commodity.iso_symbol(),
                        },
                        TransactionPosting {
                            account: e_handler.account().into(),
                            amount: period_expense,
                            currency: commodity.iso_symbol(),
                        },
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
                        TransactionPosting {
                            account: e_handler.while_prepaid().into(),
                            amount: -period_expense,
                            currency: commodity.iso_symbol(),
                        },
                        TransactionPosting {
                            account: e_handler.account().into(),
                            amount: period_expense,
                            currency: commodity.iso_symbol(),
                        },
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
                TransactionPosting {
                    account: backing_account.account(),
                    amount: -amount.abs(),
                    currency: commodity.iso_symbol(),
                },
                TransactionPosting {
                    account: e_handler.while_prepaid().into(),
                    amount: prepaid_sum,
                    currency: commodity.iso_symbol(),
                },
                TransactionPosting {
                    account: e_handler.while_payable().into(),
                    amount: -payable_sum,
                    currency: commodity.iso_symbol(),
                },
            ],
        });

        Ok(Transformation {
            spec_id: id,
            label: TransactionLabel {
                payee: payee.name(),
                description,
            },
            transactions,
            side_transactions,
            expense_history_delta: None,
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
        let init_daily = (estimate as f64) / (accrual_days as f64);

        let e_handler = e_handler.clone();
        Self::process_variable_expense_helper(spec, e_handler, init_daily)
    }

    /// Uses the past 90 days of historical data (prior to accrual date) to
    /// compute the daily average. If less than 90 days of historical data, uses
    /// what is available. If no historical data, returns an error.
    fn process_variable_expense(
        spec: DecoratedTransactionSpec<H>,
        history: &HashMap<ExpenseAccount, Vec<VariableExpenseRecord>>,
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

        // Use the last 90 days of history before (and including) the accrual_date.
        let history_window_start = accrual_start - Duration::days(90);
        let history_window_end = accrual_start; // inclusive
        let expense_account = e_handler.account();
        let records = history
            .get(&expense_account)
            .map(|v| v.as_slice())
            .unwrap_or(&[]);

        // Compute the average daily accrual rate over the 90-day window.
        let daily_rate = compute_daily_average(records, history_window_start, history_window_end)
            .ok_or_else(|| VariableExpenseNoHistoricalData::new(description))?;

        let e_handler = e_handler.clone();
        Self::process_variable_expense_helper(spec, e_handler, daily_rate)
    }

    fn process_variable_expense_helper(
        spec: DecoratedTransactionSpec<H>,
        e_handler: H::E,
        daily_rate: f64,
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
            side_transactions,
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

        let accrual_days = (accrual_end - accrual_start).num_days() + 1;
        let estimated_total = daily_rate * (accrual_days as f64);

        // Break into monthly accrual periods.
        let mut transactions = Vec::new();
        for MonthlyAccrualPeriod {
            period_start,
            period_end,
            num_days,
            adjustment_date,
        } in monthly_accrual_periods(accrual_start, accrual_end)?
        {
            let period_estimate = daily_rate * (num_days as f64);
            transactions.push(Transaction {
                spec_id: id,
                date: adjustment_date,
                comment: Some(format!(
                    "Estimated expense accrual for {} - {}",
                    period_start, period_end
                )),
                postings: vec![
                    TransactionPosting {
                        account: e_handler.while_payable().into(),
                        amount: -period_estimate,
                        currency: commodity.iso_symbol(),
                    },
                    TransactionPosting {
                        account: e_handler.account().into(),
                        amount: period_estimate,
                        currency: commodity.iso_symbol(),
                    },
                ],
            });
        }

        // Record any estimation discrepancies.
        let discrepancy = amount.abs() - estimated_total;
        if discrepancy.abs() > 0.01 {
            transactions.push(Transaction {
                spec_id: id,
                date: payment_date,
                comment: Some("Correct estimatation discrepancy".into()),
                postings: vec![
                    TransactionPosting {
                        account: e_handler.while_payable().into(),
                        amount: -discrepancy,
                        currency: commodity.iso_symbol(),
                    },
                    TransactionPosting {
                        account: e_handler.account().into(),
                        amount: discrepancy,
                        currency: commodity.iso_symbol(),
                    },
                ],
            });
        }

        // Record the clearing transaction.
        transactions.push(Transaction {
            spec_id: id,
            date: payment_date,
            comment: Some("Clear payable expense".into()),
            postings: vec![
                TransactionPosting {
                    account: backing_account.account(),
                    amount: -amount.abs(),
                    currency: commodity.iso_symbol(),
                },
                TransactionPosting {
                    account: e_handler.while_payable().into(),
                    amount: amount.abs(),
                    currency: commodity.iso_symbol(),
                },
            ],
        });

        // Record this variable expense’s daily rate for future history.
        let new_record = VariableExpenseRecord {
            start: accrual_start,
            end: accrual_end,
            daily_rate,
        };

        Ok(Transformation {
            spec_id: id,
            label: TransactionLabel {
                payee: payee.name(),
                description,
            },
            transactions,
            side_transactions,
            expense_history_delta: Some((e_handler.account(), new_record)),
            annotations,
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
            side_transactions,
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
                TransactionPosting {
                    account: i_handler.account().into(),
                    amount: -amount.abs(),
                    currency: commodity.iso_symbol(),
                },
                TransactionPosting {
                    account: backing_account.account(),
                    amount: amount.abs(),
                    currency: commodity.iso_symbol(),
                },
            ],
        };

        Ok(Transformation {
            spec_id: id,
            label: TransactionLabel {
                payee: payee.name(),
                description,
            },
            transactions: vec![tx],
            side_transactions,
            expense_history_delta: None,
            annotations: {
                let mut v = annotations;
                v.push(Annotation::ImmaterialIncome);
                v
            },
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
            side_transactions,
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
                TransactionPosting {
                    account: backing_account.account(),
                    amount: -amount.abs(),
                    currency: commodity.iso_symbol(),
                },
                TransactionPosting {
                    account: e_handler.account().into(),
                    amount: amount.abs(),
                    currency: commodity.iso_symbol(),
                },
            ],
        };

        Ok(Transformation {
            spec_id: id,
            label: TransactionLabel {
                payee: payee.name(),
                description,
            },
            transactions: vec![tx],
            side_transactions,
            expense_history_delta: None,
            annotations: {
                let mut v = annotations;
                v.push(Annotation::ImmaterialExpense);
                v
            },
        })
    }

    fn process_reimburse(spec: DecoratedTransactionSpec<H>) -> Result<Transformation, ServerError> {
        let DecoratedTransactionSpec {
            id,
            accrual_start: _,
            accrual_end: None,
            payment_date,
            accounting_logic: AccountingLogic::Reimburse(r_handler),
            payee,
            description,
            amount,
            commodity,
            backing_account,
            annotations,
            side_transactions,
        } = spec
        else {
            return Err(InvalidArgumentsForAccountingLogic::with_debug(&spec));
        };
        amount_should_be_negative!(amount, "Reimburse", &id);

        let tx = Transaction {
            spec_id: id,
            date: payment_date,
            comment: None,
            postings: vec![
                TransactionPosting {
                    account: backing_account.account(),
                    amount: -amount.abs(),
                    currency: commodity.iso_symbol(),
                },
                TransactionPosting {
                    account: r_handler.account().into(),
                    amount: amount.abs(),
                    currency: commodity.iso_symbol(),
                },
            ],
        };

        Ok(Transformation {
            spec_id: id,
            label: TransactionLabel {
                payee: payee.name(),
                description,
            },
            transactions: vec![tx],
            side_transactions,
            expense_history_delta: None,
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
            side_transactions,
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
                    TransactionPosting {
                        account: VAT_RECEIVABLE.clone().into(),
                        amount: -amount.abs(),
                        currency: commodity.iso_symbol(),
                    },
                    TransactionPosting {
                        account: cash.account().into(),
                        amount: amount.abs(),
                        currency: commodity.iso_symbol(),
                    },
                ],
            }
        } else {
            Transaction {
                spec_id: id,
                date: accrual_date,
                comment: Some(format!("Clear VAT payable for {} - {}", from, to)),
                postings: vec![
                    TransactionPosting {
                        account: cash.account().into(),
                        amount: -amount.abs(),
                        currency: commodity.iso_symbol(),
                    },
                    TransactionPosting {
                        account: VAT_PAYABLE.clone().into(),
                        amount: amount.abs(),
                        currency: commodity.iso_symbol(),
                    },
                ],
            }
        };

        Ok(Transformation {
            spec_id: id,
            label: TransactionLabel {
                payee: payee.name(),
                description,
            },
            transactions: vec![tx],
            side_transactions,
            expense_history_delta: None,
            annotations,
        })
    }
}
