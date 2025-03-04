use chrono::{Datelike, Duration, NaiveDate};
use csv;
use ron::de::from_str;
use serde::Deserialize;
use std::collections::HashMap;
use std::error::Error;

// ---------------------------------------------------------------------
// Account interface traits (client must implement these)
// ---------------------------------------------------------------------

pub trait ExpenseAccounts: Clone {
    fn account_name(&self) -> String;
    fn prepaid_account_name() -> String;
    fn payable_account_name() -> String;
}

pub trait AssetAccounts: Clone {
    fn account_name(&self) -> String;
    fn payable_account_name() -> String;
    fn prepaid_account_name() -> String;
}

pub trait IncomeAccounts: Clone {
    fn account_name(&self) -> String;
}

pub trait ReimbursableEntities: Clone {
    fn liability_account_name(&self) -> String;
}

pub trait CashAccounts: Clone {
    fn account_name(&self) -> String;
}

// ---------------------------------------------------------------------
// RON-parsable enums for Accounting Logic and Decorators.
// ---------------------------------------------------------------------

#[derive(Debug, serde_derive::Deserialize)]
pub enum AccountingLogic<E, A, I, R> {
    SimpleExpense(E),
    Capitalize(A),
    Amortize(A, E),
    FixedExpense(E),
    VariableExpense(E),
    VariableExpenseInit(E, i64),
    ImmaterialIncome(I),
    ImmaterialExpense(E),
    Reimburse(R),
    ClearVat(String, String), // (start_date, end_date)
}

#[derive(Debug, serde_derive::Deserialize)]
pub enum Decorator {
    VatRecoverable(String), // invoice date as ISO string
    VatUnrecoverable,
    VatReverseChargeExempt,
    CardFx(String, String, f64), // (target currency, settle date, at value)
}

#[derive(Debug, serde_derive::Deserialize)]
#[serde(tag = "type", content = "value")]
pub enum BackingAccount<R, B> {
    PersonalCard(R),
    Cash(B),
}

// ---------------------------------------------------------------------
// CSV record structure (maps the 10 CSV columns)
// ---------------------------------------------------------------------

pub struct CsvRecord {
    pub accrual_date: String,
    pub until: Option<String>,
    pub payment_date: String,
    pub accounting_logic: String,
    pub decorators: String,
    pub entity: String,
    pub description: String,
    pub amount: f64,
    pub commodity: String,
    pub backing_account: String,
}

// ---------------------------------------------------------------------
// Structures for ledger transactions and postings.
// ---------------------------------------------------------------------

pub struct Posting {
    pub account: String,
    pub amount: f64,
    pub commodity: String,
}

pub struct LedgerTransaction {
    pub date: NaiveDate,
    pub description: String,
    pub postings: Vec<Posting>,
    pub notes: Vec<String>,
}

// ---------------------------------------------------------------------
// Helper: calculate all month-end dates between two dates (inclusive)
// ---------------------------------------------------------------------

fn month_end_dates(start: NaiveDate, end: NaiveDate) -> Vec<NaiveDate> {
    let mut dates = Vec::new();
    let mut current = start;
    while current <= end {
        // Compute first day of next month:
        let (year, month) = if current.month() == 12 {
            (current.year() + 1, 1)
        } else {
            (current.year(), current.month() + 1)
        };
        let next_month =
            NaiveDate::from_ymd_opt(year, month, 1).expect("unexpectedly encountered invalid date");
        let last_day = next_month - Duration::days(1);
        if last_day >= start && last_day <= end {
            dates.push(last_day);
        }
        current = next_month;
    }
    dates
}

// ---------------------------------------------------------------------
// Accounting logic implementations
// Each function returns one or more ledger transactions.
// ---------------------------------------------------------------------

// -- SimpleExpense --
pub fn process_simple_expense<E, R, B>(
    _record: &CsvRecord,
    accrual_date: NaiveDate,
    payment_date: NaiveDate,
    expense: E,
    backing_account: &BackingAccount<R, B>,
    commodity: &str,
    description: &str,
    amount: f64,
) -> Result<Vec<LedgerTransaction>, Box<dyn Error>>
where
    E: ExpenseAccounts,
    R: ReimbursableEntities,
    B: CashAccounts,
{
    let mut transactions = Vec::new();
    if payment_date == accrual_date {
        let tx = LedgerTransaction {
            date: payment_date,
            description: description.to_string(),
            postings: vec![
                Posting {
                    account: expense.account_name(),
                    amount,
                    commodity: commodity.to_string(),
                },
                Posting {
                    account: match backing_account {
                        BackingAccount::Cash(b) => b.account_name(),
                        BackingAccount::PersonalCard(r) => r.liability_account_name(),
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
                    account: E::prepaid_account_name(),
                    amount,
                    commodity: commodity.to_string(),
                },
                Posting {
                    account: match backing_account {
                        BackingAccount::Cash(b) => b.account_name(),
                        BackingAccount::PersonalCard(r) => r.liability_account_name(),
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
                    account: expense.account_name(),
                    amount,
                    commodity: commodity.to_string(),
                },
                Posting {
                    account: E::prepaid_account_name(),
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
                    account: expense.account_name(),
                    amount,
                    commodity: commodity.to_string(),
                },
                Posting {
                    account: E::payable_account_name(),
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
                    account: E::payable_account_name(),
                    amount,
                    commodity: commodity.to_string(),
                },
                Posting {
                    account: match backing_account {
                        BackingAccount::Cash(b) => b.account_name(),
                        BackingAccount::PersonalCard(r) => r.liability_account_name(),
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
pub fn process_capitalize<A, R, B>(
    _record: &CsvRecord,
    accrual_date: NaiveDate,
    payment_date: NaiveDate,
    asset: A,
    backing_account: &BackingAccount<R, B>,
    commodity: &str,
    description: &str,
    amount: f64,
) -> Result<Vec<LedgerTransaction>, Box<dyn Error>>
where
    A: AssetAccounts,
    R: ReimbursableEntities,
    B: CashAccounts,
{
    let mut transactions = Vec::new();
    if payment_date == accrual_date {
        let tx = LedgerTransaction {
            date: payment_date,
            description: description.to_string(),
            postings: vec![
                Posting {
                    account: asset.account_name(),
                    amount,
                    commodity: commodity.to_string(),
                },
                Posting {
                    account: match backing_account {
                        BackingAccount::Cash(b) => b.account_name(),
                        BackingAccount::PersonalCard(r) => r.liability_account_name(),
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
                    account: A::prepaid_account_name(),
                    amount,
                    commodity: commodity.to_string(),
                },
                Posting {
                    account: match backing_account {
                        BackingAccount::Cash(b) => b.account_name(),
                        BackingAccount::PersonalCard(r) => r.liability_account_name(),
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
                    account: asset.account_name(),
                    amount,
                    commodity: commodity.to_string(),
                },
                Posting {
                    account: A::prepaid_account_name(),
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
                    account: asset.account_name(),
                    amount,
                    commodity: commodity.to_string(),
                },
                Posting {
                    account: A::payable_account_name(),
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
                    account: A::payable_account_name(),
                    amount,
                    commodity: commodity.to_string(),
                },
                Posting {
                    account: match backing_account {
                        BackingAccount::Cash(b) => b.account_name(),
                        BackingAccount::PersonalCard(r) => r.liability_account_name(),
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
pub fn process_amortize<A, R, B, E>(
    record: &CsvRecord,
    accrual_date: NaiveDate,
    payment_date: NaiveDate,
    until_date: NaiveDate,
    asset: A,
    expense: E,
    backing_account: &BackingAccount<R, B>,
    commodity: &str,
    description: &str,
    amount: f64,
) -> Result<Vec<LedgerTransaction>, Box<dyn Error>>
where
    A: AssetAccounts,
    E: ExpenseAccounts, // for amortization expense
    R: ReimbursableEntities,
    B: CashAccounts,
{
    let mut transactions = Vec::new();
    // Record asset purchase (like Capitalize)
    let mut purchase_txns = process_capitalize(
        record,
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
    let month_ends = month_end_dates(accrual_date, until_date);
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
                    account: asset.account_name(),
                    amount: -monthly_amort,
                    commodity: commodity.to_string(),
                },
                Posting {
                    account: expense.account_name(),
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
pub fn process_fixed_expense<E, R, B>(
    _record: &CsvRecord,
    accrual_date: NaiveDate,
    payment_date: NaiveDate,
    until_date: NaiveDate,
    expense: E,
    backing_account: &BackingAccount<R, B>,
    commodity: &str,
    description: &str,
    amount: f64,
) -> Result<Vec<LedgerTransaction>, Box<dyn Error>>
where
    E: ExpenseAccounts,
    R: ReimbursableEntities,
    B: CashAccounts,
{
    let mut transactions = Vec::new();
    let total_days = (until_date - accrual_date).num_days() + 1;
    let daily_expense = amount / (total_days as f64);
    let month_ends = month_end_dates(accrual_date, until_date);
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
                        account: expense.account_name(),
                        amount: monthly_expense,
                        commodity: commodity.to_string(),
                    },
                    Posting {
                        account: E::payable_account_name(),
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
                        account: E::prepaid_account_name(),
                        amount: monthly_expense,
                        commodity: commodity.to_string(),
                    },
                    Posting {
                        account: expense.account_name(),
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
                account: E::payable_account_name(),
                amount: before_sum,
                commodity: commodity.to_string(),
            },
            Posting {
                account: E::prepaid_account_name(),
                amount: -after_sum,
                commodity: commodity.to_string(),
            },
            Posting {
                account: match backing_account {
                    BackingAccount::Cash(b) => b.account_name(),
                    BackingAccount::PersonalCard(r) => r.liability_account_name(),
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
pub fn process_variable_expense<E, R, B>(
    _record: &CsvRecord,
    accrual_date: NaiveDate,
    payment_date: NaiveDate,
    until_date: NaiveDate,
    expense: E,
    backing_account: &BackingAccount<R, B>,
    commodity: &str,
    description: &str,
    amount: f64,
    var_history: &mut HashMap<String, Vec<(NaiveDate, f64)>>,
) -> Result<Vec<LedgerTransaction>, Box<dyn Error>>
where
    E: ExpenseAccounts,
    R: ReimbursableEntities,
    B: CashAccounts,
{
    if payment_date <= until_date {
        return Err("VariableExpense payment date must be after accrual period".into());
    }
    let mut transactions = Vec::new();
    let total_days = (until_date - accrual_date).num_days() + 1;
    // Use expense account name as key.
    let key = expense.account_name();
    let historical = var_history.get(&key).cloned().unwrap_or_default();
    let cutoff = accrual_date - Duration::days(90);
    let relevant: Vec<f64> = historical
        .iter()
        .filter(|(d, _)| *d >= cutoff)
        .map(|(_, v)| *v)
        .collect();
    if relevant.is_empty() {
        return Err(format!("No historical data for VariableExpense: {}", description).into());
    }
    let avg_daily: f64 = relevant.iter().sum::<f64>() / (relevant.len() as f64);
    let estimated_total = avg_daily * (total_days as f64);
    let month_ends = month_end_dates(accrual_date, until_date);
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
                    account: expense.account_name(),
                    amount: monthly_estimate,
                    commodity: commodity.to_string(),
                },
                Posting {
                    account: E::payable_account_name(),
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
            account: E::payable_account_name(),
            amount: estimated_total,
            commodity: commodity.to_string(),
        },
        Posting {
            account: match backing_account {
                BackingAccount::Cash(b) => b.account_name(),
                BackingAccount::PersonalCard(r) => r.liability_account_name(),
            },
            amount: -estimated_total,
            commodity: commodity.to_string(),
        },
    ];
    if discrepancy.abs() > 0.01 {
        let disc_account = format!("{}:Discrepancy", expense.account_name());
        postings.push(Posting {
            account: disc_account,
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
pub fn process_variable_expense_init<E, R, B>(
    _record: &CsvRecord,
    accrual_date: NaiveDate,
    payment_date: NaiveDate,
    until_date: NaiveDate,
    expense: E,
    estimate: i64,
    backing_account: &BackingAccount<R, B>,
    commodity: &str,
    description: &str,
    _amount: f64,
    var_history: &mut HashMap<String, Vec<(NaiveDate, f64)>>,
) -> Result<Vec<LedgerTransaction>, Box<dyn Error>>
where
    E: ExpenseAccounts,
    R: ReimbursableEntities,
    B: CashAccounts,
{
    let mut transactions = Vec::new();
    let total_days = (until_date - accrual_date).num_days() + 1;
    let init_daily = (estimate as f64) / (total_days as f64);
    let month_ends = month_end_dates(accrual_date, until_date);
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
                    account: expense.account_name(),
                    amount: monthly_estimate,
                    commodity: commodity.to_string(),
                },
                Posting {
                    account: E::payable_account_name(),
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
                account: E::payable_account_name(),
                amount: estimated_sum,
                commodity: commodity.to_string(),
            },
            Posting {
                account: match backing_account {
                    BackingAccount::Cash(b) => b.account_name(),
                    BackingAccount::PersonalCard(r) => r.liability_account_name(),
                },
                amount: -estimated_sum,
                commodity: commodity.to_string(),
            },
        ],
        notes: vec![],
    };
    transactions.push(tx_clear);
    var_history
        .entry(expense.account_name())
        .or_insert(Vec::new())
        .push((accrual_date, init_daily));
    Ok(transactions)
}

// -- ImmaterialIncome --
pub fn process_immaterial_income<I, R, B>(
    _record: &CsvRecord,
    payment_date: NaiveDate,
    income: I,
    backing_account: &BackingAccount<R, B>,
    commodity: &str,
    description: &str,
    amount: f64,
) -> Result<Vec<LedgerTransaction>, Box<dyn Error>>
where
    I: IncomeAccounts,
    R: ReimbursableEntities,
    B: CashAccounts,
{
    let tx = LedgerTransaction {
        date: payment_date,
        description: format!("ImmaterialIncome: {}", description),
        postings: vec![
            Posting {
                account: income.account_name(),
                amount,
                commodity: commodity.to_string(),
            },
            Posting {
                account: match backing_account {
                    BackingAccount::Cash(b) => b.account_name(),
                    BackingAccount::PersonalCard(r) => r.liability_account_name(),
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
pub fn process_immaterial_expense<E, R, B>(
    _record: &CsvRecord,
    payment_date: NaiveDate,
    expense: E,
    backing_account: &BackingAccount<R, B>,
    commodity: &str,
    description: &str,
    amount: f64,
) -> Result<Vec<LedgerTransaction>, Box<dyn Error>>
where
    E: ExpenseAccounts,
    R: ReimbursableEntities,
    B: CashAccounts,
{
    let tx = LedgerTransaction {
        date: payment_date,
        description: format!("ImmaterialExpense: {}", description),
        postings: vec![
            Posting {
                account: expense.account_name(),
                amount,
                commodity: commodity.to_string(),
            },
            Posting {
                account: match backing_account {
                    BackingAccount::Cash(b) => b.account_name(),
                    BackingAccount::PersonalCard(r) => r.liability_account_name(),
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
pub fn process_reimburse<R, B>(
    _record: &CsvRecord,
    payment_date: NaiveDate,
    reimbursable: R,
    backing_account: &BackingAccount<R, B>,
    commodity: &str,
    description: &str,
    amount: f64,
) -> Result<Vec<LedgerTransaction>, Box<dyn Error>>
where
    R: ReimbursableEntities,
    B: CashAccounts,
{
    let tx = LedgerTransaction {
        date: payment_date,
        description: format!("Reimburse: {}", description),
        postings: vec![
            Posting {
                account: reimbursable.liability_account_name(),
                amount: -amount,
                commodity: commodity.to_string(),
            },
            Posting {
                account: match backing_account {
                    BackingAccount::Cash(b) => b.account_name(),
                    BackingAccount::PersonalCard(r) => r.liability_account_name(),
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
    _record: &CsvRecord,
    accrual_date: NaiveDate,
    commodity: &str,
    description: &str,
    amount: f64,
    start: &str,
    end: &str,
) -> Result<Vec<LedgerTransaction>, Box<dyn Error>> {
    let tx = LedgerTransaction {
        date: accrual_date,
        description: format!("Clear VAT from {} to {}: {}", start, end, description),
        postings: vec![
            Posting {
                account: String::from("VatReceivable"),
                amount: -amount,
                commodity: commodity.to_string(),
            },
            Posting {
                account: String::from("Vat Receipt Pending"),
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
) -> Result<Vec<LedgerTransaction>, Box<dyn Error>> {
    let mut side_txs = Vec::new();
    if txs.is_empty() {
        return Ok(side_txs);
    }
    let core_tx = &mut txs[0];
    for dec in decorators {
        match dec {
            Decorator::VatRecoverable(invoice_date_str) => {
                let invoice_date = NaiveDate::parse_from_str(invoice_date_str, "%Y-%m-%d")?;
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
                            account: String::from("Vat Receipt Pending"),
                            amount: vat_amount,
                            commodity: commodity.to_string(),
                        },
                        Posting {
                            account: String::from("Vat Recoverable Clearing"),
                            amount: -vat_amount,
                            commodity: commodity.to_string(),
                        },
                    ],
                    notes: vec![],
                };
                let side_tx2 = LedgerTransaction {
                    date: invoice_date,
                    description: format!(
                        "VAT Recoverable: Clear Vat Receipt Pending into Vat Receivable for {}",
                        description
                    ),
                    postings: vec![
                        Posting {
                            account: String::from("Vat Receivable"),
                            amount: vat_amount,
                            commodity: commodity.to_string(),
                        },
                        Posting {
                            account: String::from("Vat Receipt Pending"),
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
                            account: String::from("Expenses:VatUnrecoverable"),
                            amount: vat_amount,
                            commodity: commodity.to_string(),
                        },
                        Posting {
                            account: String::from("Vat Unrecoverable Clearing"),
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
            Decorator::CardFx(target_currency, settle_date_str, at_value) => {
                let settle_date = NaiveDate::parse_from_str(settle_date_str, "%Y-%m-%d")?;
                // Simulate a mid-market spot rate.
                let spot_rate = 1.05;
                for posting in &mut core_tx.postings {
                    posting.amount *= spot_rate;
                    posting.commodity = target_currency.clone();
                }
                let spot_conversion = core_tx.postings[0].amount;
                let diff = at_value - spot_conversion;
                let fx_account = if diff >= 0.0 { "FxGain" } else { "FxLoss" };
                let side_tx = LedgerTransaction {
                    date: settle_date,
                    description: format!("FX adjustment for {} via CardFx", description),
                    postings: vec![
                        Posting {
                            account: fx_account.to_string(),
                            amount: diff,
                            commodity: target_currency.clone(),
                        },
                        Posting {
                            account: String::from("Fx Adjustment Clearing"),
                            amount: -diff,
                            commodity: target_currency.clone(),
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

// ---------------------------------------------------------------------
// Main processing function.
// Reads CSV content and returns (ledger_output, list of notes).
// ---------------------------------------------------------------------
pub fn process_csv<E, A, I, R, B>(
    csv_content: &str,
) -> Result<(String, Vec<String>), Box<dyn Error>>
where
    E: ExpenseAccounts + for<'de> Deserialize<'de> + std::fmt::Debug,
    A: AssetAccounts + for<'de> Deserialize<'de> + std::fmt::Debug,
    I: IncomeAccounts + for<'de> Deserialize<'de> + std::fmt::Debug,
    R: ReimbursableEntities + for<'de> Deserialize<'de> + std::fmt::Debug,
    B: CashAccounts + for<'de> Deserialize<'de> + std::fmt::Debug,
{
    let mut ledger_transactions: Vec<LedgerTransaction> = Vec::new();
    let mut global_notes: Vec<String> = Vec::new();
    // For VariableExpense historical data.
    let mut var_exp_history: HashMap<String, Vec<(NaiveDate, f64)>> = HashMap::new();

    let mut rdr = csv::Reader::from_reader(csv_content.as_bytes());
    for result in rdr.records() {
        let record = result?;
        // Map CSV columns (order as specified):
        let csv_record = CsvRecord {
            accrual_date: record.get(0).unwrap_or("").to_string(),
            until: {
                let s = record.get(1).unwrap_or("").trim();
                if s.is_empty() {
                    None
                } else {
                    Some(s.to_string())
                }
            },
            payment_date: record.get(2).unwrap_or("").to_string(),
            accounting_logic: record.get(3).unwrap_or("").to_string(),
            decorators: record.get(4).unwrap_or("").to_string(),
            entity: record.get(5).unwrap_or("").to_string(),
            description: record.get(6).unwrap_or("").to_string(),
            amount: record.get(7).unwrap_or("0").parse::<f64>()?,
            commodity: record.get(8).unwrap_or("").to_string(),
            backing_account: record.get(9).unwrap_or("").to_string(),
        };

        let accrual_date = NaiveDate::parse_from_str(&csv_record.accrual_date, "%Y-%m-%d")?;
        let payment_date = NaiveDate::parse_from_str(&csv_record.payment_date, "%Y-%m-%d")?;
        // Parse accounting logic (RON)
        let accounting_logic: AccountingLogic<E, A, I, R> = from_str(&csv_record.accounting_logic)?;
        // Parse decorators (add square brackets to form a valid RON list)
        let decorators_ron = format!("[{}]", csv_record.decorators);
        let decorators: Vec<Decorator> = from_str(&decorators_ron)?;
        // Parse backing account.
        let backing_account: BackingAccount<R, B> = from_str(&csv_record.backing_account)?;

        let mut txs: Vec<LedgerTransaction> = Vec::new();
        // Process based on accounting logic.
        match accounting_logic {
            AccountingLogic::SimpleExpense(expense) => {
                let transactions = process_simple_expense(
                    &csv_record,
                    accrual_date,
                    payment_date,
                    expense,
                    &backing_account,
                    &csv_record.commodity,
                    &csv_record.description,
                    csv_record.amount,
                )?;
                txs.extend(transactions);
            }
            AccountingLogic::Capitalize(asset) => {
                let transactions = process_capitalize(
                    &csv_record,
                    accrual_date,
                    payment_date,
                    asset,
                    &backing_account,
                    &csv_record.commodity,
                    &csv_record.description,
                    csv_record.amount,
                )?;
                txs.extend(transactions);
            }
            AccountingLogic::Amortize(asset, expense) => {
                let until_str = csv_record
                    .until
                    .as_ref()
                    .ok_or("Missing Until date for Amortize")?;
                let until_date = NaiveDate::parse_from_str(until_str, "%Y-%m-%d")?;
                let transactions = process_amortize(
                    &csv_record,
                    accrual_date,
                    payment_date,
                    until_date,
                    asset,
                    expense,
                    &backing_account,
                    &csv_record.commodity,
                    &csv_record.description,
                    csv_record.amount,
                )?;
                txs.extend(transactions);
            }
            AccountingLogic::FixedExpense(expense) => {
                let until_str = csv_record
                    .until
                    .as_ref()
                    .ok_or("Missing Until date for FixedExpense")?;
                let until_date = NaiveDate::parse_from_str(until_str, "%Y-%m-%d")?;
                let transactions = process_fixed_expense(
                    &csv_record,
                    accrual_date,
                    payment_date,
                    until_date,
                    expense,
                    &backing_account,
                    &csv_record.commodity,
                    &csv_record.description,
                    csv_record.amount,
                )?;
                txs.extend(transactions);
            }
            AccountingLogic::VariableExpense(expense) => {
                let until_str = csv_record
                    .until
                    .as_ref()
                    .ok_or("Missing Until date for VariableExpense")?;
                let until_date = NaiveDate::parse_from_str(until_str, "%Y-%m-%d")?;
                let transactions = process_variable_expense(
                    &csv_record,
                    accrual_date,
                    payment_date,
                    until_date,
                    expense,
                    &backing_account,
                    &csv_record.commodity,
                    &csv_record.description,
                    csv_record.amount,
                    &mut var_exp_history,
                )?;
                txs.extend(transactions);
            }
            AccountingLogic::VariableExpenseInit(expense, estimate) => {
                let until_str = csv_record
                    .until
                    .as_ref()
                    .ok_or("Missing Until date for VariableExpenseInit")?;
                let until_date = NaiveDate::parse_from_str(until_str, "%Y-%m-%d")?;
                let transactions = process_variable_expense_init(
                    &csv_record,
                    accrual_date,
                    payment_date,
                    until_date,
                    expense,
                    estimate,
                    &backing_account,
                    &csv_record.commodity,
                    &csv_record.description,
                    csv_record.amount,
                    &mut var_exp_history,
                )?;
                txs.extend(transactions);
            }
            AccountingLogic::ImmaterialIncome(income) => {
                let transactions = process_immaterial_income(
                    &csv_record,
                    payment_date,
                    income,
                    &backing_account,
                    &csv_record.commodity,
                    &csv_record.description,
                    csv_record.amount,
                )?;
                txs.extend(transactions);
                global_notes.push(format!(
                    "Immaterial income applied: {}",
                    csv_record.description
                ));
            }
            AccountingLogic::ImmaterialExpense(expense) => {
                let transactions = process_immaterial_expense(
                    &csv_record,
                    payment_date,
                    expense,
                    &backing_account,
                    &csv_record.commodity,
                    &csv_record.description,
                    csv_record.amount,
                )?;
                txs.extend(transactions);
                global_notes.push(format!(
                    "Immaterial expense applied: {}",
                    csv_record.description
                ));
            }
            AccountingLogic::Reimburse(reimbursable) => {
                let transactions = process_reimburse(
                    &csv_record,
                    payment_date,
                    reimbursable,
                    &backing_account,
                    &csv_record.commodity,
                    &csv_record.description,
                    csv_record.amount,
                )?;
                txs.extend(transactions);
            }
            AccountingLogic::ClearVat(start, end) => {
                let transactions = process_clear_vat(
                    &csv_record,
                    accrual_date,
                    &csv_record.commodity,
                    &csv_record.description,
                    csv_record.amount,
                    &start,
                    &end,
                )?;
                txs.extend(transactions);
            }
        }
        // Process decorators.
        let side_txs = apply_decorators(
            &mut txs,
            &decorators,
            accrual_date,
            payment_date,
            &csv_record.commodity,
            &csv_record.description,
        )?;
        txs.extend(side_txs);
        // Append all transactions for this CSV record.
        ledger_transactions.extend(txs);
    }

    // Format ledger transactions into hledger format.
    let mut ledger_output = String::new();
    for tx in ledger_transactions {
        ledger_output.push_str(&format!("{} {}\n", tx.date, tx.description));
        for posting in tx.postings {
            ledger_output.push_str(&format!(
                "    {:30} {:10.2} {}\n",
                posting.account, posting.amount, posting.commodity
            ));
        }
        for note in tx.notes {
            ledger_output.push_str(&format!("    ; {}\n", note));
        }
        ledger_output.push('\n');
    }
    Ok((ledger_output, global_notes))
}
