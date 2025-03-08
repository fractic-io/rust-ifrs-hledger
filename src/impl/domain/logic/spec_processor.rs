use std::collections::HashMap;

use chrono::{Datelike, Duration, NaiveDate};
use fractic_server_error::ServerError;

use crate::{
    domain::logic::utils::{month_end_dates, month_start},
    entities::{
        asset, Account, AccountingLogic, Assertion, AssetHandler, BackingAccount, CashHandler,
        CommodityHandler, ExpenseAccount, ExpenseHandler, FinancialRecordSpecs, FinancialRecords,
        Handlers, IncomeHandler, Note, PayeeHandler, Posting, ReimbursableEntityHandler,
        Transaction, TransactionSpec,
    },
    errors::{
        InvalidArgumentsForAccountingLogic, NonAmortizableAsset, UnexpectedNegativeValue,
        UnexpectedPositiveValue, VariableExpenseInvalidPaymentDate,
        VariableExpenseNoHistoricalData,
    },
};

pub(crate) struct SpecProcessor<H: Handlers> {
    specs: FinancialRecordSpecs<H>,
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
    transactions: Vec<Transaction>,
    expense_history_delta: Option<(ExpenseAccount, VariableExpenseRecord)>,
    notes: Vec<Note>,
}

struct FoldState {
    accounts: Vec<Account>,
    transactions: Vec<Transaction>,
    expense_history: HashMap<ExpenseAccount, Vec<VariableExpenseRecord>>,
    notes: Vec<Note>,
}

impl FoldState {
    fn new() -> Self {
        Self {
            accounts: Vec::new(),
            transactions: Vec::new(),
            expense_history: HashMap::new(),
            notes: Vec::new(),
        }
    }

    /// Update current state with the given transformation.
    fn step(self, t: Transformation) -> Self {
        let mut accounts = self.accounts;
        let mut transactions = self.transactions;
        let mut expense_history = self.expense_history;
        let mut notes = self.notes;

        accounts.extend(
            t.transactions
                .iter()
                .flat_map(|t| t.postings.iter().map(|p| p.account.clone())),
        );

        transactions.extend(t.transactions);
        if let Some((account, record)) = t.expense_history_delta {
            expense_history.entry(account).or_default().push(record);
        }
        notes.extend(t.notes);

        Self {
            accounts,
            transactions,
            expense_history,
            notes,
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
    pub(crate) fn new(specs: FinancialRecordSpecs<H>) -> Self {
        Self { specs }
    }

    pub(crate) fn process(self) -> Result<FinancialRecords, ServerError> {
        let FinancialRecordSpecs {
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
            notes: transactions_fold_result.notes,
            assertions,
        })
    }

    fn process_simple_expense(spec: TransactionSpec<H>) -> Result<Transformation, ServerError> {
        let TransactionSpec {
            id,
            accrual_date,
            until: None,
            payment_date,
            accounting_logic: AccountingLogic::SimpleExpense(expense),
            payee: entity,
            description,
            amount,
            commodity,
            backing_account,
            ..
        } = spec
        else {
            return Err(InvalidArgumentsForAccountingLogic::with_debug(&spec));
        };
        amount_should_be_negative!(amount, "SimpleExpense", &id);

        let transactions = if payment_date == accrual_date {
            vec![Transaction {
                spec_id: id,
                date: payment_date,
                entity: entity.name(),
                description: description,
                postings: vec![
                    Posting {
                        account: backing_account.account(),
                        amount: -amount.abs(),
                        currency: commodity.iso_symbol(),
                    },
                    Posting {
                        account: expense.account().into(),
                        amount: amount.abs(),
                        currency: commodity.iso_symbol(),
                    },
                ],
                notes: vec![],
            }]
        } else if payment_date < accrual_date {
            vec![
                Transaction {
                    spec_id: id.clone(),
                    date: payment_date,
                    entity: entity.name(),
                    description: format!("Prepaid: {}", description),
                    postings: vec![
                        Posting {
                            account: backing_account.account(),
                            amount: -amount.abs(),
                            currency: commodity.iso_symbol(),
                        },
                        Posting {
                            account: expense.while_prepaid().into(),
                            amount: amount.abs(),
                            currency: commodity.iso_symbol(),
                        },
                    ],
                    notes: vec![],
                },
                Transaction {
                    spec_id: id,
                    date: accrual_date,
                    entity: entity.name(),
                    description: format!("Clear Prepaid: {}", description),
                    postings: vec![
                        Posting {
                            account: expense.while_prepaid().into(),
                            amount: -amount.abs(),
                            currency: commodity.iso_symbol(),
                        },
                        Posting {
                            account: expense.account().into(),
                            amount: amount.abs(),
                            currency: commodity.iso_symbol(),
                        },
                    ],
                    notes: vec![],
                },
            ]
        } else {
            vec![
                Transaction {
                    spec_id: id.clone(),
                    date: accrual_date,
                    entity: entity.name(),
                    description: format!("Accrued Expense: {}", description),
                    postings: vec![
                        Posting {
                            account: expense.while_payable().into(),
                            amount: -amount.abs(),
                            currency: commodity.iso_symbol(),
                        },
                        Posting {
                            account: expense.account().into(),
                            amount: amount.abs(),
                            currency: commodity.iso_symbol(),
                        },
                    ],
                    notes: vec![],
                },
                Transaction {
                    spec_id: id,
                    date: payment_date,
                    entity: entity.name(),
                    description: format!("Clear Accrued Expense: {}", description),
                    postings: vec![
                        Posting {
                            account: backing_account.account(),
                            amount: -amount.abs(),
                            currency: commodity.iso_symbol(),
                        },
                        Posting {
                            account: expense.while_payable().into(),
                            amount: amount.abs(),
                            currency: commodity.iso_symbol(),
                        },
                    ],
                    notes: vec![],
                },
            ]
        };

        Ok(Transformation {
            transactions,
            expense_history_delta: None,
            notes: vec![],
        })
    }

    fn process_capitalize(spec: TransactionSpec<H>) -> Result<Transformation, ServerError> {
        let TransactionSpec {
            id,
            accrual_date,
            until: None,
            payment_date,
            accounting_logic: AccountingLogic::Capitalize(asset),
            payee: entity,
            description,
            amount,
            commodity,
            backing_account,
            ..
        } = spec
        else {
            return Err(InvalidArgumentsForAccountingLogic::with_debug(&spec));
        };
        amount_should_be_negative!(amount, "Capitalize", &id);

        let transactions = if payment_date == accrual_date {
            vec![Transaction {
                spec_id: id,
                date: payment_date,
                entity: entity.name(),
                description: description,
                postings: vec![
                    Posting {
                        account: asset.account().into(),
                        amount: amount.abs(),
                        currency: commodity.iso_symbol(),
                    },
                    Posting {
                        account: backing_account.account(),
                        amount: -amount.abs(),
                        currency: commodity.iso_symbol(),
                    },
                ],
                notes: vec![],
            }]
        } else if payment_date < accrual_date {
            vec![
                Transaction {
                    spec_id: id.clone(),
                    date: payment_date,
                    entity: entity.name(),
                    description: format!("Prepaid Asset: {}", description),
                    postings: vec![
                        Posting {
                            account: asset.while_prepaid().into(),
                            amount: amount.abs(),
                            currency: commodity.iso_symbol(),
                        },
                        Posting {
                            account: backing_account.account(),
                            amount: -amount.abs(),
                            currency: commodity.iso_symbol(),
                        },
                    ],
                    notes: vec![],
                },
                Transaction {
                    spec_id: id,
                    date: accrual_date,
                    entity: entity.name(),
                    description: format!("Clear Prepaid Asset: {}", description),
                    postings: vec![
                        Posting {
                            account: asset.account().into(),
                            amount: amount.abs(),
                            currency: commodity.iso_symbol(),
                        },
                        Posting {
                            account: asset.while_prepaid().into(),
                            amount: -amount.abs(),
                            currency: commodity.iso_symbol(),
                        },
                    ],
                    notes: vec![],
                },
            ]
        } else {
            vec![
                Transaction {
                    spec_id: id.clone(),
                    date: accrual_date,
                    entity: entity.name(),
                    description: format!("Accrued Asset Purchase: {}", description),
                    postings: vec![
                        Posting {
                            account: asset.account().into(),
                            amount: amount.abs(),
                            currency: commodity.iso_symbol(),
                        },
                        Posting {
                            account: asset.while_payable().into(),
                            amount: -amount.abs(),
                            currency: commodity.iso_symbol(),
                        },
                    ],
                    notes: vec![],
                },
                Transaction {
                    spec_id: id,
                    date: payment_date,
                    entity: entity.name(),
                    description: format!("Clear Asset Payable: {}", description),
                    postings: vec![
                        Posting {
                            account: asset.while_payable().into(),
                            amount: amount.abs(),
                            currency: commodity.iso_symbol(),
                        },
                        Posting {
                            account: backing_account.account(),
                            amount: -amount.abs(),
                            currency: commodity.iso_symbol(),
                        },
                    ],
                    notes: vec![],
                },
            ]
        };

        Ok(Transformation {
            transactions,
            expense_history_delta: None,
            notes: vec![],
        })
    }

    fn process_amortize(spec: TransactionSpec<H>) -> Result<Transformation, ServerError> {
        let TransactionSpec {
            id,
            accrual_date,
            until: Some(until_date),
            payment_date,
            accounting_logic: AccountingLogic::Amortize(asset),
            payee: entity,
            description,
            amount,
            commodity,
            backing_account,
            ..
        } = spec
        else {
            return Err(InvalidArgumentsForAccountingLogic::with_debug(&spec));
        };
        amount_should_be_negative!(amount, "Amortize", &id);

        let mut transactions = Vec::new();

        let cap_transformation = Self::process_capitalize(TransactionSpec {
            id: id.clone(),
            accrual_date,
            until: None,
            payment_date,
            accounting_logic: AccountingLogic::Capitalize(asset.clone()),
            decorators: Default::default(),
            payee: entity.clone(),
            description: description.clone(),
            amount,
            commodity: commodity.clone(),
            backing_account,
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
                spec_id: id.clone(),
                date: m,
                entity: entity.name(),
                description: format!(
                    "Amortization adjustment: {} for period {} to {}",
                    description, period_start, period_end
                ),
                postings: vec![
                    Posting {
                        account: asset.account().into(),
                        amount: -monthly_amort,
                        currency: commodity.iso_symbol(),
                    },
                    Posting {
                        account: accrual_account.into(),
                        amount: monthly_amort,
                        currency: commodity.iso_symbol(),
                    },
                ],
                notes: vec![],
            });
        }

        Ok(Transformation {
            transactions,
            expense_history_delta: None,
            notes: vec![],
        })
    }

    fn process_fixed_expense(spec: TransactionSpec<H>) -> Result<Transformation, ServerError> {
        let TransactionSpec {
            id,
            accrual_date,
            until: Some(until_date),
            payment_date,
            accounting_logic: AccountingLogic::FixedExpense(expense),
            payee: entity,
            description,
            amount,
            commodity,
            backing_account,
            ..
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
                    spec_id: id.clone(),
                    date: m,
                    entity: entity.name(),
                    description: format!(
                        "FixedExpense accrual (pre-payment): {} for period {} to {}",
                        description, period_start, period_end
                    ),
                    postings: vec![
                        Posting {
                            account: expense.account().into(),
                            amount: monthly_expense,
                            currency: commodity.iso_symbol(),
                        },
                        Posting {
                            account: expense.while_payable().into(),
                            amount: -monthly_expense,
                            currency: commodity.iso_symbol(),
                        },
                    ],
                    notes: vec![],
                });
                before_sum += monthly_expense;
            } else {
                transactions.push(Transaction {
                    spec_id: id.clone(),
                    date: m,
                    entity: entity.name(),
                    description: format!(
                        "FixedExpense accrual (post-payment): {} for period {} to {}",
                        description, period_start, period_end
                    ),
                    postings: vec![
                        Posting {
                            account: expense.while_payable().into(),
                            amount: monthly_expense,
                            currency: commodity.iso_symbol(),
                        },
                        Posting {
                            account: expense.account().into(),
                            amount: -monthly_expense,
                            currency: commodity.iso_symbol(),
                        },
                    ],
                    notes: vec![],
                });
                after_sum += monthly_expense;
            }
        }

        // Clearing transaction on payment date.
        transactions.push(Transaction {
            spec_id: id,
            date: payment_date,
            entity: entity.name(),
            description: format!("Clear FixedExpense adjustments: {}", description),
            postings: vec![
                Posting {
                    account: expense.while_payable().into(),
                    amount: before_sum,
                    currency: commodity.iso_symbol(),
                },
                Posting {
                    account: expense.while_prepaid().into(),
                    amount: -after_sum,
                    currency: commodity.iso_symbol(),
                },
                Posting {
                    account: backing_account.account(),
                    amount: -(before_sum - after_sum),
                    currency: commodity.iso_symbol(),
                },
            ],
            notes: vec![],
        });

        Ok(Transformation {
            transactions,
            expense_history_delta: None,
            notes: vec![],
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
        spec: TransactionSpec<H>,
        history: &HashMap<ExpenseAccount, Vec<VariableExpenseRecord>>,
    ) -> Result<Transformation, ServerError> {
        let TransactionSpec {
            id,
            accrual_date,
            until: Some(until_date),
            payment_date,
            accounting_logic: AccountingLogic::VariableExpense(expense),
            payee: entity,
            description,
            amount,
            commodity,
            backing_account,
            ..
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
                spec_id: id.clone(),
                date: m,
                entity: entity.name(),
                description: format!(
                    "VariableExpense accrual: {} for period {} to {}",
                    description, period_start, period_end
                ),
                postings: vec![
                    Posting {
                        account: expense.account().into(),
                        amount: monthly_estimate,
                        currency: commodity.iso_symbol(),
                    },
                    Posting {
                        account: expense.while_payable().into(),
                        amount: -monthly_estimate,
                        currency: commodity.iso_symbol(),
                    },
                ],
                notes: vec![],
            });
        }

        let discrepancy = amount - estimated_total;
        let mut postings = vec![
            Posting {
                account: expense.while_payable().into(),
                amount: estimated_total,
                currency: commodity.iso_symbol(),
            },
            Posting {
                account: backing_account.account(),
                amount: -estimated_total,
                currency: commodity.iso_symbol(),
            },
        ];
        if discrepancy.abs() > 0.01 {
            postings.push(Posting {
                account: expense.account().into(),
                amount: discrepancy,
                currency: commodity.iso_symbol(),
            });
        }
        // Clearing transaction on the payment date.
        transactions.push(Transaction {
            spec_id: id,
            date: payment_date,
            entity: entity.name(),
            description: format!("Clear VariableExpense adjustments: {}", description),
            postings,
            notes: vec![],
        });

        // Return the transformation and record this variable expense’s daily rate for future history.
        let new_record = VariableExpenseRecord {
            start: accrual_date,
            end: until_date,
            daily_rate: avg_daily,
        };

        Ok(Transformation {
            transactions,
            expense_history_delta: Some((expense_account.into(), new_record)),
            notes: vec![],
        })
    }

    /// Process a VariableExpenseInit spec. In this case the daily accrual is
    /// computed directly from the provided estimate.
    fn process_variable_expense_init(
        spec: TransactionSpec<H>,
        _history: &HashMap<ExpenseAccount, Vec<VariableExpenseRecord>>, // Not used for computing init daily
    ) -> Result<Transformation, ServerError> {
        let TransactionSpec {
            id,
            accrual_date,
            until: Some(until_date),
            payment_date,
            accounting_logic:
                AccountingLogic::VariableExpenseInit {
                    account: expense,
                    estimate,
                },
            payee: entity,
            description,
            amount: _amount, // unused aside from sign check
            commodity,
            backing_account,
            ..
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
                spec_id: id.clone(),
                date: m,
                entity: entity.name(),
                description: format!(
                    "VariableExpenseInit accrual: {} for period {} to {}",
                    description, period_start, period_end
                ),
                postings: vec![
                    Posting {
                        account: expense.account().into(),
                        amount: monthly_estimate,
                        currency: commodity.iso_symbol(),
                    },
                    Posting {
                        account: expense.while_payable().into(),
                        amount: -monthly_estimate,
                        currency: commodity.iso_symbol(),
                    },
                ],
                notes: vec![],
            });
        }
        transactions.push(Transaction {
            spec_id: id,
            date: payment_date,
            entity: entity.name(),
            description: format!("Clear VariableExpenseInit adjustments: {}", description),
            postings: vec![
                Posting {
                    account: expense.while_payable().into(),
                    amount: estimated_sum,
                    currency: commodity.iso_symbol(),
                },
                Posting {
                    account: backing_account.account(),
                    amount: -estimated_sum,
                    currency: commodity.iso_symbol(),
                },
            ],
            notes: vec![],
        });

        let new_record = VariableExpenseRecord {
            start: accrual_date,
            end: until_date,
            daily_rate: init_daily,
        };

        Ok(Transformation {
            transactions,
            expense_history_delta: Some((expense.account().into(), new_record)),
            notes: vec![],
        })
    }

    fn process_immaterial_income(spec: TransactionSpec<H>) -> Result<Transformation, ServerError> {
        let TransactionSpec {
            id,
            accrual_date: _,
            until: None,
            payment_date,
            accounting_logic: AccountingLogic::ImmaterialIncome(income),
            payee: entity,
            description,
            amount,
            commodity,
            backing_account,
            ..
        } = spec
        else {
            return Err(InvalidArgumentsForAccountingLogic::with_debug(&spec));
        };
        amount_should_be_positive!(amount, "ImmaterialIncome", &id);

        let tx = Transaction {
            spec_id: id,
            date: payment_date,
            entity: entity.name(),
            description: format!("ImmaterialIncome: {}", description),
            postings: vec![
                Posting {
                    account: income.account().into(),
                    amount,
                    currency: commodity.iso_symbol(),
                },
                Posting {
                    account: backing_account.account(),
                    amount: -amount,
                    currency: commodity.iso_symbol(),
                },
            ],
            notes: vec![format!("Immaterial income recorded for {}", description)],
        };

        Ok(Transformation {
            transactions: vec![tx],
            expense_history_delta: None,
            notes: vec![],
        })
    }

    fn process_immaterial_expense(spec: TransactionSpec<H>) -> Result<Transformation, ServerError> {
        let TransactionSpec {
            id,
            accrual_date: _,
            until: None,
            payment_date,
            accounting_logic: AccountingLogic::ImmaterialExpense(expense),
            payee: entity,
            description,
            amount,
            commodity,
            backing_account,
            ..
        } = spec
        else {
            return Err(InvalidArgumentsForAccountingLogic::with_debug(&spec));
        };
        amount_should_be_negative!(amount, "ImmaterialExpense", &id);

        let tx = Transaction {
            spec_id: id,
            date: payment_date,
            entity: entity.name(),
            description: format!("ImmaterialExpense: {}", description),
            postings: vec![
                Posting {
                    account: expense.account().into(),
                    amount: amount.abs(),
                    currency: commodity.iso_symbol(),
                },
                Posting {
                    account: backing_account.account(),
                    amount: -amount.abs(),
                    currency: commodity.iso_symbol(),
                },
            ],
            notes: vec![format!("Immaterial expense recorded for {}", description)],
        };

        Ok(Transformation {
            transactions: vec![tx],
            expense_history_delta: None,
            notes: vec![],
        })
    }

    fn process_reimburse(spec: TransactionSpec<H>) -> Result<Transformation, ServerError> {
        let TransactionSpec {
            id,
            accrual_date: _,
            until: None,
            payment_date,
            accounting_logic: AccountingLogic::Reimburse(reimbursable),
            payee: entity,
            description,
            amount,
            commodity,
            backing_account,
            ..
        } = spec
        else {
            return Err(InvalidArgumentsForAccountingLogic::with_debug(&spec));
        };
        amount_should_be_negative!(amount, "Reimburse", &id);

        let tx = Transaction {
            spec_id: id,
            date: payment_date,
            entity: entity.name(),
            description: format!("Reimburse: {}", description),
            postings: vec![
                Posting {
                    account: reimbursable.account().into(),
                    amount: -amount.abs(),
                    currency: commodity.iso_symbol(),
                },
                Posting {
                    account: backing_account.account(),
                    amount: amount.abs(),
                    currency: commodity.iso_symbol(),
                },
            ],
            notes: vec![format!("Reimbursement processed for {}", description)],
        };

        Ok(Transformation {
            transactions: vec![tx],
            expense_history_delta: None,
            notes: vec![],
        })
    }

    fn process_clear_vat(spec: TransactionSpec<H>) -> Result<Transformation, ServerError> {
        let TransactionSpec {
            id,
            accrual_date,
            until: None,
            payment_date: _,
            accounting_logic: AccountingLogic::ClearVat { from, to },
            payee: entity,
            description,
            amount,
            commodity,
            backing_account: BackingAccount::Cash(cash),
            ..
        } = spec
        else {
            return Err(InvalidArgumentsForAccountingLogic::with_debug(&spec));
        };

        let tx = Transaction {
            spec_id: id,
            date: accrual_date,
            entity: entity.name(),
            description: format!("Clear VAT from {} to {}: {}", from, to, description),
            postings: vec![
                Posting {
                    account: asset("VatReceivable").into(),
                    amount: -amount,
                    currency: commodity.iso_symbol(),
                },
                Posting {
                    account: cash.account().into(),
                    amount,
                    currency: commodity.iso_symbol(),
                },
            ],
            notes: vec![String::from("VAT clearance processed")],
        };

        Ok(Transformation {
            transactions: vec![tx],
            expense_history_delta: None,
            notes: vec![],
        })
    }
}
