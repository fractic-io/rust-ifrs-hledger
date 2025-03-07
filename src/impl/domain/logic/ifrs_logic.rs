// -- SimpleExpense --
pub fn process_simple_expense<E, R, C>(
    accrual_date: NaiveDate,
    payment_date: NaiveDate,
    expense: E,
    backing_account: &BackingAccount<R, C>,
    commodity: &str,
    description: &str,
    amount: f64,
) -> Result<Vec<LedgerTransaction>, ServerError>
where
    E: ExpenseHandler,
    R: ReimbursableEntityHandler,
    C: CashHandler,
{
    let mut transactions = Vec::new();
    if payment_date == accrual_date {
        let tx = LedgerTransaction {
            date: payment_date,
            description: description.to_string(),
            postings: vec![
                Posting {
                    account: expense.account().into(),
                    amount,
                    commodity: commodity.to_string(),
                },
                Posting {
                    account: match backing_account {
                        BackingAccount::Cash(b) => b.account().into(),
                        BackingAccount::Reimburse(r) => r.account().into(),
                    },
                    amount: -amount,
                    commodity: commodity.to_string(),
                },
            ],
            notes: vec![],
        };
        transactions.push(tx);
    } else if payment_date < accrual_date {
        // Payment made earlier: record as prepaid expense then clear on accrual.
        let tx1 = LedgerTransaction {
            date: payment_date,
            description: format!("Prepaid: {}", description),
            postings: vec![
                Posting {
                    account: expense.while_prepaid().into(),
                    amount,
                    commodity: commodity.to_string(),
                },
                Posting {
                    account: match backing_account {
                        BackingAccount::Cash(b) => b.account().into(),
                        BackingAccount::Reimburse(r) => r.account().into(),
                    },
                    amount: -amount,
                    commodity: commodity.to_string(),
                },
            ],
            notes: vec![],
        };
        let tx2 = LedgerTransaction {
            date: accrual_date,
            description: format!("Clear Prepaid: {}", description),
            postings: vec![
                Posting {
                    account: expense.account().into(),
                    amount,
                    commodity: commodity.to_string(),
                },
                Posting {
                    account: expense.while_prepaid().into(),
                    amount: -amount,
                    commodity: commodity.to_string(),
                },
            ],
            notes: vec![],
        };
        transactions.push(tx1);
        transactions.push(tx2);
    } else {
        // Payment made later: record accrued expense then clear on payment date.
        let tx1 = LedgerTransaction {
            date: accrual_date,
            description: format!("Accrued Expense: {}", description),
            postings: vec![
                Posting {
                    account: expense.account().into(),
                    amount,
                    commodity: commodity.to_string(),
                },
                Posting {
                    account: expense.while_payable().into(),
                    amount: -amount,
                    commodity: commodity.to_string(),
                },
            ],
            notes: vec![],
        };
        let tx2 = LedgerTransaction {
            date: payment_date,
            description: format!("Clear Accrued Expense: {}", description),
            postings: vec![
                Posting {
                    account: expense.while_payable().into(),
                    amount,
                    commodity: commodity.to_string(),
                },
                Posting {
                    account: match backing_account {
                        BackingAccount::Cash(b) => b.account().into(),
                        BackingAccount::Reimburse(r) => r.account().into(),
                    },
                    amount: -amount,
                    commodity: commodity.to_string(),
                },
            ],
            notes: vec![],
        };
        transactions.push(tx1);
        transactions.push(tx2);
    }
    Ok(transactions)
}

// -- Capitalize --
pub fn process_capitalize<A, R, C>(
    accrual_date: NaiveDate,
    payment_date: NaiveDate,
    asset: A,
    backing_account: &BackingAccount<R, C>,
    commodity: &str,
    description: &str,
    amount: f64,
) -> Result<Vec<LedgerTransaction>, ServerError>
where
    A: AssetHandler,
    R: ReimbursableEntityHandler,
    C: CashHandler,
{
    let mut transactions = Vec::new();
    if payment_date == accrual_date {
        let tx = LedgerTransaction {
            date: payment_date,
            description: description.to_string(),
            postings: vec![
                Posting {
                    account: asset.account().into(),
                    amount,
                    commodity: commodity.to_string(),
                },
                Posting {
                    account: match backing_account {
                        BackingAccount::Cash(b) => b.account().into(),
                        BackingAccount::Reimburse(r) => r.account().into(),
                    },
                    amount: -amount,
                    commodity: commodity.to_string(),
                },
            ],
            notes: vec![],
        };
        transactions.push(tx);
    } else if payment_date < accrual_date {
        // Payment earlier: record as prepaid asset then clear.
        let tx1 = LedgerTransaction {
            date: payment_date,
            description: format!("Prepaid Asset: {}", description),
            postings: vec![
                Posting {
                    account: asset.while_prepaid().into(),
                    amount,
                    commodity: commodity.to_string(),
                },
                Posting {
                    account: match backing_account {
                        BackingAccount::Cash(b) => b.account().into(),
                        BackingAccount::Reimburse(r) => r.account().into(),
                    },
                    amount: -amount,
                    commodity: commodity.to_string(),
                },
            ],
            notes: vec![],
        };
        let tx2 = LedgerTransaction {
            date: accrual_date,
            description: format!("Clear Prepaid Asset: {}", description),
            postings: vec![
                Posting {
                    account: asset.account().into(),
                    amount,
                    commodity: commodity.to_string(),
                },
                Posting {
                    account: asset.while_prepaid().into(),
                    amount: -amount,
                    commodity: commodity.to_string(),
                },
            ],
            notes: vec![],
        };
        transactions.push(tx1);
        transactions.push(tx2);
    } else {
        // Payment later: record as asset payable then clear.
        let tx1 = LedgerTransaction {
            date: accrual_date,
            description: format!("Accrued Asset Purchase: {}", description),
            postings: vec![
                Posting {
                    account: asset.account().into(),
                    amount,
                    commodity: commodity.to_string(),
                },
                Posting {
                    account: asset.while_payable().into(),
                    amount: -amount,
                    commodity: commodity.to_string(),
                },
            ],
            notes: vec![],
        };
        let tx2 = LedgerTransaction {
            date: payment_date,
            description: format!("Clear Asset Payable: {}", description),
            postings: vec![
                Posting {
                    account: asset.while_payable().into(),
                    amount,
                    commodity: commodity.to_string(),
                },
                Posting {
                    account: match backing_account {
                        BackingAccount::Cash(b) => b.account().into(),
                        BackingAccount::Reimburse(r) => r.account().into(),
                    },
                    amount: -amount,
                    commodity: commodity.to_string(),
                },
            ],
            notes: vec![],
        };
        transactions.push(tx1);
        transactions.push(tx2);
    }
    Ok(transactions)
}

// -- Amortize --
pub fn process_amortize<A, R, C>(
    accrual_date: NaiveDate,
    payment_date: NaiveDate,
    until_date: NaiveDate,
    asset: A,
    backing_account: &BackingAccount<R, C>,
    commodity: &str,
    description: &str,
    amount: f64,
) -> Result<Vec<LedgerTransaction>, ServerError>
where
    A: AssetHandler,
    R: ReimbursableEntityHandler,
    C: CashHandler,
{
    let mut transactions = Vec::new();
    // Record asset purchase (like Capitalize)
    let mut purchase_txns = process_capitalize(
        accrual_date,
        payment_date,
        asset.clone(),
        backing_account,
        commodity,
        description,
        amount,
    )?;
    transactions.append(&mut purchase_txns);
    // Amortize evenly over accrual period.
    let total_days = (until_date - accrual_date).num_days() + 1;
    let daily_amort = amount / (total_days as f64);
    let month_ends = month_end_dates(accrual_date, until_date)?;
    for m in month_ends {
        let month_start = NaiveDate::from_ymd_opt(m.year(), m.month(), 1)
            .expect("unexpectedly encountered invalid date");
        let period_start = if accrual_date > month_start {
            accrual_date
        } else {
            month_start
        };
        let period_end = if until_date < m { until_date } else { m };
        let days_in_period = (period_end - period_start).num_days() + 1;
        let monthly_amort = daily_amort * (days_in_period as f64);
        let tx = LedgerTransaction {
            date: m,
            description: format!(
                "Amortization adjustment: {} for period {} to {}",
                description, period_start, period_end
            ),
            postings: vec![
                Posting {
                    account: asset.account().into(),
                    amount: -monthly_amort,
                    commodity: commodity.to_string(),
                },
                Posting {
                    account: asset
                        .upon_accrual()
                        .ok_or_else(|| NonAmortizableAsset::new(&description))?
                        .into(),
                    amount: monthly_amort,
                    commodity: commodity.to_string(),
                },
            ],
            notes: vec![],
        };
        transactions.push(tx);
    }
    Ok(transactions)
}

// -- FixedExpense --
pub fn process_fixed_expense<E, R, C>(
    accrual_date: NaiveDate,
    payment_date: NaiveDate,
    until_date: NaiveDate,
    expense: E,
    backing_account: &BackingAccount<R, C>,
    commodity: &str,
    description: &str,
    amount: f64,
) -> Result<Vec<LedgerTransaction>, ServerError>
where
    E: ExpenseHandler,
    R: ReimbursableEntityHandler,
    C: CashHandler,
{
    let mut transactions = Vec::new();
    let total_days = (until_date - accrual_date).num_days() + 1;
    let daily_expense = amount / (total_days as f64);
    let month_ends = month_end_dates(accrual_date, until_date)?;
    let mut before_sum = 0.0;
    let mut after_sum = 0.0;
    for m in month_ends {
        let month_start = NaiveDate::from_ymd_opt(m.year(), m.month(), 1).expect(
            "re-constructing date from NaiveDate.year() and .month() should always yield a valid date",
        );
        let period_start = if accrual_date > month_start {
            accrual_date
        } else {
            month_start
        };
        let period_end = if until_date < m { until_date } else { m };
        let days_in_period = (period_end - period_start).num_days() + 1;
        let monthly_expense = daily_expense * (days_in_period as f64);
        if m <= payment_date {
            let tx = LedgerTransaction {
                date: m,
                description: format!(
                    "FixedExpense accrual (pre-payment): {} for period {} to {}",
                    description, period_start, period_end
                ),
                postings: vec![
                    Posting {
                        account: expense.account().into(),
                        amount: monthly_expense,
                        commodity: commodity.to_string(),
                    },
                    Posting {
                        account: expense.while_payable().into(),
                        amount: -monthly_expense,
                        commodity: commodity.to_string(),
                    },
                ],
                notes: vec![],
            };
            transactions.push(tx);
            before_sum += monthly_expense;
        } else {
            let tx = LedgerTransaction {
                date: m,
                description: format!(
                    "FixedExpense accrual (post-payment): {} for period {} to {}",
                    description, period_start, period_end
                ),
                postings: vec![
                    Posting {
                        account: expense.while_payable().into(),
                        amount: monthly_expense,
                        commodity: commodity.to_string(),
                    },
                    Posting {
                        account: expense.account().into(),
                        amount: -monthly_expense,
                        commodity: commodity.to_string(),
                    },
                ],
                notes: vec![],
            };
            transactions.push(tx);
            after_sum += monthly_expense;
        }
    }
    // On payment date, clear the accrued amounts.
    let tx_clear = LedgerTransaction {
        date: payment_date,
        description: format!("Clear FixedExpense adjustments: {}", description),
        postings: vec![
            Posting {
                account: expense.while_payable().into(),
                amount: before_sum,
                commodity: commodity.to_string(),
            },
            Posting {
                account: expense.while_prepaid().into(),
                amount: -after_sum,
                commodity: commodity.to_string(),
            },
            Posting {
                account: match backing_account {
                    BackingAccount::Cash(b) => b.account().into(),
                    BackingAccount::Reimburse(r) => r.account().into(),
                },
                amount: -(before_sum - after_sum),
                commodity: commodity.to_string(),
            },
        ],
        notes: vec![],
    };
    transactions.push(tx_clear);
    Ok(transactions)
}

// -- VariableExpense --
pub fn process_variable_expense<E, R, C>(
    accrual_date: NaiveDate,
    payment_date: NaiveDate,
    until_date: NaiveDate,
    expense: E,
    backing_account: &BackingAccount<R, C>,
    commodity: &str,
    description: &str,
    amount: f64,
    var_history: &mut HashMap<ExpenseAccount, Vec<(NaiveDate, f64)>>,
) -> Result<Vec<LedgerTransaction>, ServerError>
where
    E: ExpenseHandler,
    R: ReimbursableEntityHandler,
    C: CashHandler,
{
    if payment_date <= until_date {
        return Err(VariableExpenseInvalidPaymentDate::new(
            description,
            &payment_date,
            &until_date,
        ));
    }
    let mut transactions = Vec::new();
    let total_days = (until_date - accrual_date).num_days() + 1;
    // Use expense account name as key.
    let key = expense.account();
    let historical = var_history.get(&key).cloned().unwrap_or_default();
    let cutoff = accrual_date - Duration::days(90);
    let relevant: Vec<f64> = historical
        .iter()
        .filter(|(d, _)| *d >= cutoff)
        .map(|(_, v)| *v)
        .collect();
    if relevant.is_empty() {
        return Err(VariableExpenseNoHistoricalData::new(description));
    }
    let avg_daily: f64 = relevant.iter().sum::<f64>() / (relevant.len() as f64);
    let estimated_total = avg_daily * (total_days as f64);
    let month_ends = month_end_dates(accrual_date, until_date)?;
    for m in month_ends {
        let month_start = NaiveDate::from_ymd_opt(m.year(), m.month(), 1).expect(
            "re-constructing date from NaiveDate.year() and .month() should always yield a valid date",
        );
        let period_start = if accrual_date > month_start {
            accrual_date
        } else {
            month_start
        };
        let period_end = if until_date < m { until_date } else { m };
        let days_in_period = (period_end - period_start).num_days() + 1;
        let monthly_estimate = avg_daily * (days_in_period as f64);
        let tx = LedgerTransaction {
            date: m,
            description: format!(
                "VariableExpense accrual: {} for period {} to {}",
                description, period_start, period_end
            ),
            postings: vec![
                Posting {
                    account: expense.account().into(),
                    amount: monthly_estimate,
                    commodity: commodity.to_string(),
                },
                Posting {
                    account: expense.while_payable().into(),
                    amount: -monthly_estimate,
                    commodity: commodity.to_string(),
                },
            ],
            notes: vec![],
        };
        transactions.push(tx);
    }
    let discrepancy = amount - estimated_total;
    let mut postings = vec![
        Posting {
            account: expense.while_payable().into(),
            amount: estimated_total,
            commodity: commodity.to_string(),
        },
        Posting {
            account: match backing_account {
                BackingAccount::Cash(b) => b.account().into(),
                BackingAccount::Reimburse(r) => r.account().into(),
            },
            amount: -estimated_total,
            commodity: commodity.to_string(),
        },
    ];
    if discrepancy.abs() > 0.01 {
        postings.push(Posting {
            account: expense.account().into(),
            amount: discrepancy,
            commodity: commodity.to_string(),
        });
    }
    let tx_clear = LedgerTransaction {
        date: payment_date,
        description: format!("Clear VariableExpense adjustments: {}", description),
        postings,
        notes: vec![],
    };
    transactions.push(tx_clear);
    var_history
        .entry(key)
        .or_insert(Vec::new())
        .push((accrual_date, avg_daily));
    Ok(transactions)
}

// -- VariableExpenseInit --
pub fn process_variable_expense_init<E, R, C>(
    accrual_date: NaiveDate,
    payment_date: NaiveDate,
    until_date: NaiveDate,
    expense: E,
    estimate: i64,
    backing_account: &BackingAccount<R, C>,
    commodity: &str,
    description: &str,
    _amount: f64,
    var_history: &mut HashMap<ExpenseAccount, Vec<(NaiveDate, f64)>>,
) -> Result<Vec<LedgerTransaction>, ServerError>
where
    E: ExpenseHandler,
    R: ReimbursableEntityHandler,
    C: CashHandler,
{
    let mut transactions = Vec::new();
    let total_days = (until_date - accrual_date).num_days() + 1;
    let init_daily = (estimate as f64) / (total_days as f64);
    let month_ends = month_end_dates(accrual_date, until_date)?;
    let mut estimated_sum = 0.0;
    for m in month_ends {
        let month_start = NaiveDate::from_ymd_opt(m.year(), m.month(), 1).expect(
            "re-constructing date from NaiveDate.year() and .month() should always yield a valid date",
        );
        let period_start = if accrual_date > month_start {
            accrual_date
        } else {
            month_start
        };
        let period_end = if until_date < m { until_date } else { m };
        let days_in_period = (period_end - period_start).num_days() + 1;
        let monthly_estimate = init_daily * (days_in_period as f64);
        estimated_sum += monthly_estimate;
        let tx = LedgerTransaction {
            date: m,
            description: format!(
                "VariableExpenseInit accrual: {} for period {} to {}",
                description, period_start, period_end
            ),
            postings: vec![
                Posting {
                    account: expense.account().into(),
                    amount: monthly_estimate,
                    commodity: commodity.to_string(),
                },
                Posting {
                    account: expense.while_payable().into(),
                    amount: -monthly_estimate,
                    commodity: commodity.to_string(),
                },
            ],
            notes: vec![],
        };
        transactions.push(tx);
    }
    let tx_clear = LedgerTransaction {
        date: payment_date,
        description: format!("Clear VariableExpenseInit adjustments: {}", description),
        postings: vec![
            Posting {
                account: expense.while_payable().into(),
                amount: estimated_sum,
                commodity: commodity.to_string(),
            },
            Posting {
                account: match backing_account {
                    BackingAccount::Cash(b) => b.account().into(),
                    BackingAccount::Reimburse(r) => r.account().into(),
                },
                amount: -estimated_sum,
                commodity: commodity.to_string(),
            },
        ],
        notes: vec![],
    };
    transactions.push(tx_clear);
    var_history
        .entry(expense.account())
        .or_insert(Vec::new())
        .push((accrual_date, init_daily));
    Ok(transactions)
}

// -- ImmaterialIncome --
pub fn process_immaterial_income<I, R, C>(
    payment_date: NaiveDate,
    income: I,
    backing_account: &BackingAccount<R, C>,
    commodity: &str,
    description: &str,
    amount: f64,
) -> Result<Vec<LedgerTransaction>, ServerError>
where
    I: IncomeHandler,
    R: ReimbursableEntityHandler,
    C: CashHandler,
{
    let tx = LedgerTransaction {
        date: payment_date,
        description: format!("ImmaterialIncome: {}", description),
        postings: vec![
            Posting {
                account: income.account().into(),
                amount,
                commodity: commodity.to_string(),
            },
            Posting {
                account: match backing_account {
                    BackingAccount::Cash(b) => b.account().into(),
                    BackingAccount::Reimburse(r) => r.account().into(),
                },
                amount: -amount,
                commodity: commodity.to_string(),
            },
        ],
        notes: vec![format!("Immaterial income recorded for {}", description)],
    };
    Ok(vec![tx])
}

// -- ImmaterialExpense --
pub fn process_immaterial_expense<E, R, C>(
    payment_date: NaiveDate,
    expense: E,
    backing_account: &BackingAccount<R, C>,
    commodity: &str,
    description: &str,
    amount: f64,
) -> Result<Vec<LedgerTransaction>, ServerError>
where
    E: ExpenseHandler,
    R: ReimbursableEntityHandler,
    C: CashHandler,
{
    let tx = LedgerTransaction {
        date: payment_date,
        description: format!("ImmaterialExpense: {}", description),
        postings: vec![
            Posting {
                account: expense.account().into(),
                amount,
                commodity: commodity.to_string(),
            },
            Posting {
                account: match backing_account {
                    BackingAccount::Cash(b) => b.account().into(),
                    BackingAccount::Reimburse(r) => r.account().into(),
                },
                amount: -amount,
                commodity: commodity.to_string(),
            },
        ],
        notes: vec![format!("Immaterial expense recorded for {}", description)],
    };
    Ok(vec![tx])
}

// -- Reimburse --
pub fn process_reimburse<R, C>(
    payment_date: NaiveDate,
    reimbursable: R,
    backing_account: &BackingAccount<R, C>,
    commodity: &str,
    description: &str,
    amount: f64,
) -> Result<Vec<LedgerTransaction>, ServerError>
where
    R: ReimbursableEntityHandler,
    C: CashHandler,
{
    let tx = LedgerTransaction {
        date: payment_date,
        description: format!("Reimburse: {}", description),
        postings: vec![
            Posting {
                account: reimbursable.account().into(),
                amount: -amount,
                commodity: commodity.to_string(),
            },
            Posting {
                account: match backing_account {
                    BackingAccount::Cash(b) => b.account().into(),
                    BackingAccount::Reimburse(r) => r.account().into(),
                },
                amount,
                commodity: commodity.to_string(),
            },
        ],
        notes: vec![format!("Reimbursement processed for {}", description)],
    };
    Ok(vec![tx])
}

// -- ClearVat --
pub fn process_clear_vat(
    accrual_date: NaiveDate,
    backing_account: AssetAccount,
    commodity: &str,
    description: &str,
    amount: f64,
    start: NaiveDate,
    end: NaiveDate,
) -> Result<Vec<LedgerTransaction>, ServerError> {
    let tx = LedgerTransaction {
        date: accrual_date,
        description: format!("Clear VAT from {} to {}: {}", start, end, description),
        postings: vec![
            Posting {
                account: AssetAccount("VatReceivable".into()).into(),
                amount: -amount,
                commodity: commodity.to_string(),
            },
            Posting {
                account: backing_account.into(),
                amount,
                commodity: commodity.to_string(),
            },
        ],
        notes: vec![String::from("VAT clearance processed")],
    };
    Ok(vec![tx])
}

// ---------------------------------------------------------------------
// Decorator processing.
// This function takes the “core” transactions (assumed to be in txs[0])
// and returns any extra side transactions.
// ---------------------------------------------------------------------
pub fn apply_decorators(
    txs: &mut Vec<LedgerTransaction>,
    decorators: &Vec<Decorator>,
    _accrual_date: NaiveDate,
    payment_date: NaiveDate,
    commodity: &str,
    description: &str,
) -> Result<Vec<LedgerTransaction>, ServerError> {
    let mut side_txs = Vec::new();
    if txs.is_empty() {
        return Ok(side_txs);
    }
    let core_tx = &mut txs[0];
    for dec in decorators {
        match dec {
            Decorator::VatAwaitingInvoice {} => {
                core_tx
                    .notes
                    .push(String::from("VAT charged but awaiting invoice"));
            }
            Decorator::VatRecoverable {
                invoice: ISODate(invoice_date),
            } => {
                // Assume first posting is the main one.
                let orig_amount = core_tx.postings[0].amount;
                let core_amount = orig_amount * 100.0 / 110.0;
                let vat_amount = orig_amount - core_amount;
                core_tx.postings[0].amount = core_amount;
                // Adjust the backing posting proportionally.
                core_tx.postings[1].amount =
                    -core_tx.postings[1].amount * (core_amount / orig_amount);
                let side_tx1 = LedgerTransaction {
                    date: payment_date,
                    description: format!(
                        "VAT Recoverable: Record Vat Receipt Pending for {}",
                        description
                    ),
                    postings: vec![
                        Posting {
                            account: LiabilityAccount("VatReceiptPending".into()).into(),
                            amount: vat_amount,
                            commodity: commodity.to_string(),
                        },
                        Posting {
                            account: ExpenseAccount("VatSubtraction".into()).into(),
                            amount: -vat_amount,
                            commodity: commodity.to_string(),
                        },
                    ],
                    notes: vec![],
                };
                let side_tx2 = LedgerTransaction {
                    date: invoice_date.clone(),
                    description: format!(
                        "VAT Recoverable: Clear Vat Receipt Pending into Vat Receivable for {}",
                        description
                    ),
                    postings: vec![
                        Posting {
                            account: AssetAccount("VatReceivable".into()).into(),
                            amount: vat_amount,
                            commodity: commodity.to_string(),
                        },
                        Posting {
                            account: LiabilityAccount("VatReceiptPending".into()).into(),
                            amount: -vat_amount,
                            commodity: commodity.to_string(),
                        },
                    ],
                    notes: vec![],
                };
                side_txs.push(side_tx1);
                side_txs.push(side_tx2);
            }
            Decorator::VatUnrecoverable => {
                let orig_amount = core_tx.postings[0].amount;
                let core_amount = orig_amount * 100.0 / 110.0;
                let vat_amount = orig_amount - core_amount;
                core_tx.postings[0].amount = core_amount;
                core_tx.postings[1].amount =
                    -core_tx.postings[1].amount * (core_amount / orig_amount);
                let side_tx = LedgerTransaction {
                    date: payment_date,
                    description: format!(
                        "VAT Unrecoverable: Record unrecoverable VAT expense for {}",
                        description
                    ),
                    postings: vec![
                        Posting {
                            account: ExpenseAccount("VatUnrecoverable".into()).into(),
                            amount: vat_amount,
                            commodity: commodity.to_string(),
                        },
                        Posting {
                            account: ExpenseAccount("VatSubtraction".into()).into(),
                            amount: -vat_amount,
                            commodity: commodity.to_string(),
                        },
                    ],
                    notes: vec![],
                };
                side_txs.push(side_tx);
            }
            Decorator::VatReverseChargeExempt => {
                core_tx
                    .notes
                    .push(String::from("VAT charged on a reverse-charge basis"));
            }
            Decorator::CardFx {
                to: target_currency,
                settle: ISODate(settle_date),
                at,
            } => {
                // Simulate a mid-market spot rate.
                let spot_rate = 1.05;
                for posting in &mut core_tx.postings {
                    posting.amount *= spot_rate;
                    posting.commodity = target_currency.clone();
                }
                let spot_conversion = core_tx.postings[0].amount;
                let diff = at - spot_conversion;
                let fx_account: Account = if diff >= 0.0 {
                    IncomeAccount("FxGain".into()).into()
                } else {
                    ExpenseAccount("FxLoss".into()).into()
                };
                let side_tx = LedgerTransaction {
                    date: settle_date.clone(),
                    description: format!("FX adjustment for {} via CardFx", description),
                    postings: vec![
                        Posting {
                            account: fx_account,
                            amount: diff,
                            commodity: target_currency.clone(),
                        },
                        Posting {
                            account: ExpenseAccount("FxSubtraction".into()).into(),
                            amount: -diff,
                            commodity: target_currency.clone(),
                        },
                    ],
                    notes: vec![],
                };
                side_txs.push(side_tx);
            }
            Decorator::PaymentFee(fee) => {
                let side_tx = LedgerTransaction {
                    date: payment_date,
                    description: format!("Payment Fee: {} for {}", fee, description),
                    postings: vec![
                        Posting {
                            account: ExpenseAccount("PaymentFees".into()).into(),
                            amount: *fee,
                            commodity: commodity.to_string(),
                        },
                        Posting {
                            account: AssetAccount("Cash:Hana".into()).into(),
                            amount: -fee,
                            commodity: commodity.to_string(),
                        },
                    ],
                    notes: vec![],
                };
                side_txs.push(side_tx);
            }
        }
    }
    Ok(side_txs)
}
