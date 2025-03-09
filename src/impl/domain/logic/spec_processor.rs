use std::collections::HashMap;

use chrono::{Datelike, Duration, NaiveDate};
use fractic_server_error::ServerError;

use crate::{
    domain::logic::utils::{month_end_dates, month_start},
    entities::{
        asset, Account, AccountingLogic, Annotation, Assertion, AssetHandler, BackingAccount,
        CashHandler, CommodityHandler, DecoratedFinancialRecordSpecs, DecoratedTransactionSpec,
        ExpenseAccount, ExpenseHandler, FinancialRecords, Handlers, IncomeHandler, PayeeHandler,
        ReimbursableEntityHandler, Transaction, TransactionLabel, TransactionPosting,
        TransactionSpecId,
    },
    errors::{
        InvalidArgumentsForAccountingLogic, NonAmortizableAsset, UnexpectedNegativeValue,
        UnexpectedPositiveValue, VariableExpenseInvalidPaymentDate,
        VariableExpenseNoHistoricalData,
    },
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
                        AccountingLogic::VariableExpense(..) => {
                            Self::process_variable_expense(spec, &state.expense_history)?
                        }
                        AccountingLogic::VariableExpenseInit { .. } => {
                            Self::process_variable_expense_init(spec, &state.expense_history)?
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
            accrual_date,
            until: None,
            payment_date,
            accounting_logic: AccountingLogic::SimpleExpense(expense),
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
                        account: expense.account().into(),
                        amount: amount.abs(),
                        currency: commodity.iso_symbol(),
                    },
                ],
            }]
        } else if payment_date < accrual_date {
            vec![
                Transaction {
                    spec_id: id,
                    date: payment_date,
                    comment: Some("Prepaid".into()),
                    postings: vec![
                        TransactionPosting {
                            account: backing_account.account(),
                            amount: -amount.abs(),
                            currency: commodity.iso_symbol(),
                        },
                        TransactionPosting {
                            account: expense.while_prepaid().into(),
                            amount: amount.abs(),
                            currency: commodity.iso_symbol(),
                        },
                    ],
                },
                Transaction {
                    spec_id: id,
                    date: accrual_date,
                    comment: Some("Clear Prepaid".into()),
                    postings: vec![
                        TransactionPosting {
                            account: expense.while_prepaid().into(),
                            amount: -amount.abs(),
                            currency: commodity.iso_symbol(),
                        },
                        TransactionPosting {
                            account: expense.account().into(),
                            amount: amount.abs(),
                            currency: commodity.iso_symbol(),
                        },
                    ],
                },
            ]
        } else {
            vec![
                Transaction {
                    spec_id: id,
                    date: accrual_date,
                    comment: Some("Accrue Expense".into()),
                    postings: vec![
                        TransactionPosting {
                            account: expense.while_payable().into(),
                            amount: -amount.abs(),
                            currency: commodity.iso_symbol(),
                        },
                        TransactionPosting {
                            account: expense.account().into(),
                            amount: amount.abs(),
                            currency: commodity.iso_symbol(),
                        },
                    ],
                },
                Transaction {
                    spec_id: id,
                    date: payment_date,
                    comment: Some("Clear Accrued Expense".into()),
                    postings: vec![
                        TransactionPosting {
                            account: backing_account.account(),
                            amount: -amount.abs(),
                            currency: commodity.iso_symbol(),
                        },
                        TransactionPosting {
                            account: expense.while_payable().into(),
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
            accrual_date,
            until: None,
            payment_date,
            accounting_logic: AccountingLogic::Capitalize(asset),
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
            vec![Transaction {
                spec_id: id,
                date: payment_date,
                comment: None,
                postings: vec![
                    TransactionPosting {
                        account: asset.account().into(),
                        amount: amount.abs(),
                        currency: commodity.iso_symbol(),
                    },
                    TransactionPosting {
                        account: backing_account.account(),
                        amount: -amount.abs(),
                        currency: commodity.iso_symbol(),
                    },
                ],
            }]
        } else if payment_date < accrual_date {
            vec![
                Transaction {
                    spec_id: id,
                    date: payment_date,
                    comment: Some("Prepaid Asset".into()),
                    postings: vec![
                        TransactionPosting {
                            account: asset.while_prepaid().into(),
                            amount: amount.abs(),
                            currency: commodity.iso_symbol(),
                        },
                        TransactionPosting {
                            account: backing_account.account(),
                            amount: -amount.abs(),
                            currency: commodity.iso_symbol(),
                        },
                    ],
                },
                Transaction {
                    spec_id: id,
                    date: accrual_date,
                    comment: Some("Clear Prepaid Asset".into()),
                    postings: vec![
                        TransactionPosting {
                            account: asset.account().into(),
                            amount: amount.abs(),
                            currency: commodity.iso_symbol(),
                        },
                        TransactionPosting {
                            account: asset.while_prepaid().into(),
                            amount: -amount.abs(),
                            currency: commodity.iso_symbol(),
                        },
                    ],
                },
            ]
        } else {
            vec![
                Transaction {
                    spec_id: id,
                    date: accrual_date,
                    comment: Some("Accrued Asset Purchase".into()),
                    postings: vec![
                        TransactionPosting {
                            account: asset.account().into(),
                            amount: amount.abs(),
                            currency: commodity.iso_symbol(),
                        },
                        TransactionPosting {
                            account: asset.while_payable().into(),
                            amount: -amount.abs(),
                            currency: commodity.iso_symbol(),
                        },
                    ],
                },
                Transaction {
                    spec_id: id,
                    date: payment_date,
                    comment: Some("Clear Asset Payable".into()),
                    postings: vec![
                        TransactionPosting {
                            account: asset.while_payable().into(),
                            amount: amount.abs(),
                            currency: commodity.iso_symbol(),
                        },
                        TransactionPosting {
                            account: backing_account.account(),
                            amount: -amount.abs(),
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
            accrual_date,
            until: Some(until_date),
            payment_date,
            accounting_logic: AccountingLogic::Amortize(asset),
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

        let cap_transformation = Self::process_capitalize(DecoratedTransactionSpec {
            id,
            accrual_date,
            until: None,
            payment_date,
            accounting_logic: AccountingLogic::Capitalize(asset.clone()),
            payee: payee.clone(),
            description: description.clone(),
            amount,
            commodity: commodity.clone(),
            backing_account,
            annotations: annotations.clone(),
            side_transactions: Default::default(),
        })?;
        transactions.extend(cap_transformation.transactions);

        let total_days = (until_date - accrual_date).num_days() + 1;
        let daily_amort = amount / (total_days as f64);
        let month_ends = month_end_dates(accrual_date, until_date)?;
        for m in month_ends {
            let month_start =
                NaiveDate::from_ymd_opt(m.year(), m.month(), 1).expect("invalid date");
            let period_start = if accrual_date > month_start {
                accrual_date
            } else {
                month_start
            };
            let period_end = if until_date < m { until_date } else { m };
            let days_in_period = (period_end - period_start).num_days() + 1;
            let monthly_amort = daily_amort * (days_in_period as f64);
            // Use asset.upon_accrual() – error if asset isn’t amortizable.
            let accrual_account = asset
                .upon_accrual()
                .ok_or_else(|| NonAmortizableAsset::new(&description))?;
            transactions.push(Transaction {
                spec_id: id,
                date: m,
                comment: Some(format!(
                    "Amortization Adjustment for Period {} to {}",
                    period_start, period_end
                )),
                postings: vec![
                    TransactionPosting {
                        account: asset.account().into(),
                        amount: -monthly_amort,
                        currency: commodity.iso_symbol(),
                    },
                    TransactionPosting {
                        account: accrual_account.into(),
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
            accrual_date,
            until: Some(until_date),
            payment_date,
            accounting_logic: AccountingLogic::FixedExpense(expense),
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

        let total_days = (until_date - accrual_date).num_days() + 1;
        let daily_expense = amount.abs() / (total_days as f64);
        let month_ends = month_end_dates(accrual_date, until_date)?;
        let mut transactions = Vec::new();
        let mut before_sum = 0.0;
        let mut after_sum = 0.0;
        for m in month_ends {
            let month_start =
                NaiveDate::from_ymd_opt(m.year(), m.month(), 1).expect("invalid date");
            let period_start = if accrual_date > month_start {
                accrual_date
            } else {
                month_start
            };
            let period_end = if until_date < m { until_date } else { m };
            let days_in_period = (period_end - period_start).num_days() + 1;
            let monthly_expense = daily_expense * (days_in_period as f64);
            if m <= payment_date {
                transactions.push(Transaction {
                    spec_id: id,
                    date: m,
                    comment: Some(format!(
                        "Accrual (Pre-Payment) for Period {} to {}",
                        period_start, period_end
                    )),
                    postings: vec![
                        TransactionPosting {
                            account: expense.account().into(),
                            amount: monthly_expense,
                            currency: commodity.iso_symbol(),
                        },
                        TransactionPosting {
                            account: expense.while_payable().into(),
                            amount: -monthly_expense,
                            currency: commodity.iso_symbol(),
                        },
                    ],
                });
                before_sum += monthly_expense;
            } else {
                transactions.push(Transaction {
                    spec_id: id,
                    date: m,
                    comment: Some(format!(
                        "Accrual (Post-Payment) for Period {} to {}",
                        period_start, period_end
                    )),
                    postings: vec![
                        TransactionPosting {
                            account: expense.while_payable().into(),
                            amount: monthly_expense,
                            currency: commodity.iso_symbol(),
                        },
                        TransactionPosting {
                            account: expense.account().into(),
                            amount: -monthly_expense,
                            currency: commodity.iso_symbol(),
                        },
                    ],
                });
                after_sum += monthly_expense;
            }
        }

        // Clearing transaction on payment date.
        transactions.push(Transaction {
            spec_id: id,
            date: payment_date,
            comment: Some("Clear Fixed Expense Adjustments".into()),
            postings: vec![
                TransactionPosting {
                    account: expense.while_payable().into(),
                    amount: before_sum,
                    currency: commodity.iso_symbol(),
                },
                TransactionPosting {
                    account: expense.while_prepaid().into(),
                    amount: -after_sum,
                    currency: commodity.iso_symbol(),
                },
                TransactionPosting {
                    account: backing_account.account(),
                    amount: -(before_sum - after_sum),
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

    /// Given a slice of variable expense records and a window defined by
    /// [window_start, window_end], this computes the “effective” daily accrual
    /// rate by summing the contributions of all overlapping records.  Days not
    /// covered by any record count as zero.
    fn compute_daily_average(
        records: &[VariableExpenseRecord],
        window_start: NaiveDate,
        window_end: NaiveDate,
    ) -> Option<f64> {
        let total_window_days = (window_end - window_start).num_days() + 1;
        if total_window_days <= 0 {
            return None;
        }
        let mut total_amount = 0.0;
        // For each record, add (daily_rate * number_of_overlapping_days)
        for record in records {
            let overlap_start = std::cmp::max(record.start, window_start);
            let overlap_end = std::cmp::min(record.end, window_end);
            if overlap_start <= overlap_end {
                let days = (overlap_end - overlap_start).num_days() + 1;
                total_amount += record.daily_rate * (days as f64);
            }
        }
        // If no days contributed (i.e. no overlapping records) then we have no history.
        if total_amount == 0.0 {
            None
        } else {
            Some(total_amount / (total_window_days as f64))
        }
    }

    /// Process a VariableExpense spec. It uses historical data (the past 90
    /// days prior to the accrual date) to compute the daily average.
    fn process_variable_expense(
        spec: DecoratedTransactionSpec<H>,
        history: &HashMap<ExpenseAccount, Vec<VariableExpenseRecord>>,
    ) -> Result<Transformation, ServerError> {
        let DecoratedTransactionSpec {
            id,
            accrual_date,
            until: Some(until_date),
            payment_date,
            accounting_logic: AccountingLogic::VariableExpense(expense),
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

        if payment_date <= until_date {
            return Err(VariableExpenseInvalidPaymentDate::new(
                &description,
                &payment_date,
                &until_date,
            ));
        }

        // Use the last 90 days of history before (and including) the accrual_date.
        let history_window_start = accrual_date - Duration::days(90);
        let history_window_end = accrual_date; // include accrual_date itself
        let expense_account = expense.account();
        let records = history
            .get(&expense_account)
            .map(|v| v.as_slice())
            .unwrap_or(&[]);

        // Compute the average daily accrual rate over the 90-day window.
        let avg_daily =
            Self::compute_daily_average(records, history_window_start, history_window_end)
                .ok_or_else(|| VariableExpenseNoHistoricalData::new(&description))?;

        let accrual_days = (until_date - accrual_date).num_days() + 1;
        let estimated_total = avg_daily * (accrual_days as f64);

        // Break the accrual period into month‐end segments.
        let month_ends = month_end_dates(accrual_date, until_date)?;
        let mut transactions = Vec::new();
        for m in month_ends {
            let period_start = if accrual_date > month_start(m) {
                accrual_date
            } else {
                month_start(m)
            };
            let period_end = if until_date < m { until_date } else { m };
            let days_in_period = (period_end - period_start).num_days() + 1;
            let monthly_estimate = avg_daily * (days_in_period as f64);

            transactions.push(Transaction {
                spec_id: id,
                date: m,
                comment: Some(format!(
                    "Variable Expense Accrual for Period {} to {}",
                    period_start, period_end
                )),
                postings: vec![
                    TransactionPosting {
                        account: expense.account().into(),
                        amount: monthly_estimate,
                        currency: commodity.iso_symbol(),
                    },
                    TransactionPosting {
                        account: expense.while_payable().into(),
                        amount: -monthly_estimate,
                        currency: commodity.iso_symbol(),
                    },
                ],
            });
        }

        let discrepancy = amount - estimated_total;
        let mut postings = vec![
            TransactionPosting {
                account: expense.while_payable().into(),
                amount: estimated_total,
                currency: commodity.iso_symbol(),
            },
            TransactionPosting {
                account: backing_account.account(),
                amount: -estimated_total,
                currency: commodity.iso_symbol(),
            },
        ];
        if discrepancy.abs() > 0.01 {
            postings.push(TransactionPosting {
                account: expense.account().into(),
                amount: discrepancy,
                currency: commodity.iso_symbol(),
            });
        }
        // Clearing transaction on the payment date.
        transactions.push(Transaction {
            spec_id: id,
            date: payment_date,
            comment: Some("Clear Variable Expense Adjustments".into()),
            postings,
        });

        // Return the transformation and record this variable expense’s daily rate for future history.
        let new_record = VariableExpenseRecord {
            start: accrual_date,
            end: until_date,
            daily_rate: avg_daily,
        };

        Ok(Transformation {
            spec_id: id,
            label: TransactionLabel {
                payee: payee.name(),
                description,
            },
            transactions,
            side_transactions,
            expense_history_delta: Some((expense_account.into(), new_record)),
            annotations,
        })
    }

    /// Process a VariableExpenseInit spec. In this case the daily accrual is
    /// computed directly from the provided estimate.
    fn process_variable_expense_init(
        spec: DecoratedTransactionSpec<H>,
        _history: &HashMap<ExpenseAccount, Vec<VariableExpenseRecord>>, // Not used for computing init daily
    ) -> Result<Transformation, ServerError> {
        let DecoratedTransactionSpec {
            id,
            accrual_date,
            until: Some(until_date),
            payment_date,
            accounting_logic:
                AccountingLogic::VariableExpenseInit {
                    account: expense,
                    estimate,
                },
            payee,
            description,
            amount: _amount, // unused aside from sign check
            commodity,
            backing_account,
            annotations,
            side_transactions,
        } = spec
        else {
            return Err(InvalidArgumentsForAccountingLogic::with_debug(&spec));
        };
        amount_should_be_negative!(_amount, "VariableExpenseInit", &id);

        let accrual_days = (until_date - accrual_date).num_days() + 1;
        let init_daily = (estimate as f64) / (accrual_days as f64);

        let month_ends = month_end_dates(accrual_date, until_date)?;
        let mut transactions = Vec::new();
        let mut estimated_sum = 0.0;
        for m in month_ends {
            let period_start = if accrual_date > month_start(m) {
                accrual_date
            } else {
                month_start(m)
            };
            let period_end = if until_date < m { until_date } else { m };
            let days_in_period = (period_end - period_start).num_days() + 1;
            let monthly_estimate = init_daily * (days_in_period as f64);
            estimated_sum += monthly_estimate;
            transactions.push(Transaction {
                spec_id: id,
                date: m,
                comment: Some(format!(
                    "Variable Expense Accrual for Period {} to {}",
                    period_start, period_end
                )),
                postings: vec![
                    TransactionPosting {
                        account: expense.account().into(),
                        amount: monthly_estimate,
                        currency: commodity.iso_symbol(),
                    },
                    TransactionPosting {
                        account: expense.while_payable().into(),
                        amount: -monthly_estimate,
                        currency: commodity.iso_symbol(),
                    },
                ],
            });
        }
        transactions.push(Transaction {
            spec_id: id,
            date: payment_date,
            comment: Some("Clear Variable Expense Adjustments".into()),
            postings: vec![
                TransactionPosting {
                    account: expense.while_payable().into(),
                    amount: estimated_sum,
                    currency: commodity.iso_symbol(),
                },
                TransactionPosting {
                    account: backing_account.account(),
                    amount: -estimated_sum,
                    currency: commodity.iso_symbol(),
                },
            ],
        });

        let new_record = VariableExpenseRecord {
            start: accrual_date,
            end: until_date,
            daily_rate: init_daily,
        };

        Ok(Transformation {
            spec_id: id,
            label: TransactionLabel {
                payee: payee.name(),
                description,
            },
            transactions,
            side_transactions,
            expense_history_delta: Some((expense.account().into(), new_record)),
            annotations,
        })
    }

    fn process_immaterial_income(
        spec: DecoratedTransactionSpec<H>,
    ) -> Result<Transformation, ServerError> {
        let DecoratedTransactionSpec {
            id,
            accrual_date: _, // Ignored.
            until: _,        // Ignored.
            payment_date,
            accounting_logic: AccountingLogic::ImmaterialIncome(income),
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
                    account: income.account().into(),
                    amount,
                    currency: commodity.iso_symbol(),
                },
                TransactionPosting {
                    account: backing_account.account(),
                    amount: -amount,
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
            accrual_date: _, // Ignored.
            until: _,        // Ignored.
            payment_date,
            accounting_logic: AccountingLogic::ImmaterialExpense(expense),
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
                    account: expense.account().into(),
                    amount: amount.abs(),
                    currency: commodity.iso_symbol(),
                },
                TransactionPosting {
                    account: backing_account.account(),
                    amount: -amount.abs(),
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
            accrual_date: _,
            until: None,
            payment_date,
            accounting_logic: AccountingLogic::Reimburse(reimbursable),
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
                    account: reimbursable.account().into(),
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
            annotations,
        })
    }

    fn process_clear_vat(spec: DecoratedTransactionSpec<H>) -> Result<Transformation, ServerError> {
        let DecoratedTransactionSpec {
            id,
            accrual_date,
            until: None,
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

        let tx = Transaction {
            spec_id: id,
            date: accrual_date,
            comment: Some(format!("Clear VAT from {} to {}", from, to)),
            postings: vec![
                TransactionPosting {
                    account: asset("VatReceivable").into(),
                    amount: -amount,
                    currency: commodity.iso_symbol(),
                },
                TransactionPosting {
                    account: cash.account().into(),
                    amount,
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
}
