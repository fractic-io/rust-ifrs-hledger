use chrono::{Datelike, Duration, NaiveDate};
use csv;
use fractic_server_error::{define_client_error, CriticalError, ServerError};
use ron::de::from_str;
use serde::Deserialize;
use std::collections::HashMap;
use std::fmt;
use std::str::FromStr;
use strum::IntoEnumIterator;

define_client_error!(
    NonAmortizableAsset,
    "Asset '{name}' does not have any value defined for 'upon_accrual()'. Provide a non-None value to support amortization.",
    { name: &str }
);
define_client_error!(
    VariableExpenseInvalidPaymentDate,
    "Invalid VariableExpense: '{description}'. Payment date ({payment_date}) must be after accrual period (accrual end: {until_date}), otherwise it would indicate we're prepaying for an unknown expense.",
    { description: &str, payment_date: &NaiveDate, until_date: &NaiveDate }
);
define_client_error!(
    VariableExpenseNoHistoricalData,
    "No historical data for VariableExpense: '{description}'. Must initiate with a VariableExpenseInit entry.",
    { description: &str }
);
define_client_error!(
    ClearVatInvalidBackingAccount,
    "ClearVat entry '{description}' requires a Cash backing account.",
    { description: &str }
);
define_client_error!(
    AccrualRangeRequired,
    "{account_logic} requires an accrual range, but entry '{description}' does not have accrual until-date.",
    { account_logic: &str, description: &str }
);
define_client_error!(
    AccrualRangeNotSupported,
    "{account_logic} requires an accrual range, but entry '{description}' has an accrual until-date.",
    { account_logic: &str, description: &str }
);
define_client_error!(InvalidCsv, "Invalid CSV format.");
define_client_error!(InvalidRon, "Invalid {ron_type} (invalid RON format).", { ron_type: &str });
define_client_error!(InvalidIsoDate, "Invalid ISO date: {date}.", { date: &str });
define_client_error!(
    InvalidAccountingAmount,
    "Invalid accounting amount: '{value}'.",
    { value: &str }
);

// ---------------------------------------------------------------------
// Account interface traits (client must implement these)
// ---------------------------------------------------------------------

#[derive(Debug, PartialEq, Eq, Hash)]
pub struct Asset(String);
impl Asset {
    pub fn new(name: impl Into<String>) -> Self {
        Asset(name.into())
    }
}

#[derive(Debug, PartialEq, Eq, Hash)]
pub struct Liability(String);
impl Liability {
    pub fn new(name: impl Into<String>) -> Self {
        Liability(name.into())
    }
}

#[derive(Debug, PartialEq, Eq, Hash)]
pub struct Income(String);
impl Income {
    pub fn new(name: impl Into<String>) -> Self {
        Income(name.into())
    }
}

#[derive(Debug, PartialEq, Eq, Hash)]
pub struct Expense(String);
impl Expense {
    pub fn new(name: impl Into<String>) -> Self {
        Expense(name.into())
    }
}

#[derive(Debug, PartialEq, Eq, Hash)]
pub enum Account {
    Asset(Asset),
    Liability(Liability),
    Income(Income),
    Expense(Expense),
}

impl fmt::Display for Account {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Account::Asset(s) => write!(f, "Assets:{}", s.0),
            Account::Liability(s) => write!(f, "Liabilities:{}", s.0),
            Account::Income(s) => write!(f, "Income:{}", s.0),
            Account::Expense(s) => write!(f, "Expenses:{}", s.0),
        }
    }
}
impl Into<Account> for Asset {
    fn into(self) -> Account {
        Account::Asset(self)
    }
}
impl Into<Account> for Liability {
    fn into(self) -> Account {
        Account::Liability(self)
    }
}
impl Into<Account> for Income {
    fn into(self) -> Account {
        Account::Income(self)
    }
}
impl Into<Account> for Expense {
    fn into(self) -> Account {
        Account::Expense(self)
    }
}

pub trait AssetHandler: Clone + IntoEnumIterator {
    fn account(&self) -> Asset;
    fn while_prepaid(&self) -> Asset;
    fn while_payable(&self) -> Liability;
    fn upon_accrual(&self) -> Option<Expense>;
}

pub trait IncomeHandler: Clone + IntoEnumIterator {
    fn account(&self) -> Income;
    fn while_prepaid(&self) -> Liability;
    fn while_receivable(&self) -> Asset;
}

pub trait ExpenseHandler: Clone + IntoEnumIterator {
    fn account(&self) -> Expense;
    fn while_prepaid(&self) -> Asset;
    fn while_payable(&self) -> Liability;
}

pub trait ReimbursableEntityHandler: Clone + IntoEnumIterator {
    fn account(&self) -> Liability;
}

pub trait CashHandler: Clone + IntoEnumIterator {
    fn account(&self) -> Asset;
}

// ---------------------------------------------------------------------
// RON-parsable enums for Accounting Logic and Decorators.
// ---------------------------------------------------------------------

#[derive(Debug)]
pub struct ISODate(NaiveDate);
impl FromStr for ISODate {
    type Err = ServerError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let d = NaiveDate::parse_from_str(s, "%Y-%m-%d")
            .map_err(|e| InvalidIsoDate::with_debug(s, &e))?;
        Ok(ISODate(d))
    }
}
impl<'de> Deserialize<'de> for ISODate {
    fn deserialize<D>(deserializer: D) -> Result<ISODate, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        ISODate::from_str(&s).map_err(serde::de::Error::custom)
    }
}

type Currency = String;

#[derive(Debug, serde_derive::Deserialize)]
pub enum AccountingLogic<E, A, I, R> {
    SimpleExpense(E),
    Capitalize(A),
    Amortize(A),
    FixedExpense(E),
    VariableExpense(E),
    VariableExpenseInit { account: E, estimate: i64 },
    ImmaterialIncome(I),
    ImmaterialExpense(E),
    Reimburse(R),
    ClearVat { from: ISODate, to: ISODate },
}

#[derive(Debug, serde_derive::Deserialize)]
pub enum Decorator {
    VatAwaitingInvoice,
    VatRecoverable {
        invoice: ISODate,
    },
    VatUnrecoverable,
    VatReverseChargeExempt,
    CardFx {
        to: Currency,
        settle: ISODate,
        at: f64,
    },
    PaymentFee(f64),
}

#[derive(Debug, serde_derive::Deserialize)]
pub enum BackingAccount<R, B> {
    Reimburse(R),
    Cash(B),
}

// ---------------------------------------------------------------------
// Structures for ledger transactions and postings.
// ---------------------------------------------------------------------

pub struct Posting {
    pub account: Account,
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

fn month_end_dates(start: NaiveDate, end: NaiveDate) -> Result<Vec<NaiveDate>, ServerError> {
    let mut dates = Vec::new();
    let mut current = start;
    while current <= end {
        // Compute first day of next month.
        let (year, month) = if current.month() == 12 {
            (current.year() + 1, 1)
        } else {
            (current.year(), current.month() + 1)
        };
        let next_month = NaiveDate::from_ymd_opt(year, month, 1).ok_or_else(|| {
            CriticalError::with_debug(
                "last-date-of-month calculation unexpectedly resulted in invalid date",
                &format!("year: {}, month: {}", year, month),
            )
        })?;
        let last_day = next_month - Duration::days(1);
        if last_day >= start && last_day <= end {
            dates.push(last_day);
        }
        current = next_month;
    }
    Ok(dates)
}

// ---------------------------------------------------------------------
// Accounting logic implementations
// Each function returns one or more ledger transactions.
// ---------------------------------------------------------------------

// -- SimpleExpense --
pub fn process_simple_expense<E, R, B>(
    accrual_date: NaiveDate,
    payment_date: NaiveDate,
    expense: E,
    backing_account: &BackingAccount<R, B>,
    commodity: &str,
    description: &str,
    amount: f64,
) -> Result<Vec<LedgerTransaction>, ServerError>
where
    E: ExpenseHandler,
    R: ReimbursableEntityHandler,
    B: CashHandler,
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
pub fn process_capitalize<A, R, B>(
    accrual_date: NaiveDate,
    payment_date: NaiveDate,
    asset: A,
    backing_account: &BackingAccount<R, B>,
    commodity: &str,
    description: &str,
    amount: f64,
) -> Result<Vec<LedgerTransaction>, ServerError>
where
    A: AssetHandler,
    R: ReimbursableEntityHandler,
    B: CashHandler,
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
pub fn process_amortize<A, R, B>(
    accrual_date: NaiveDate,
    payment_date: NaiveDate,
    until_date: NaiveDate,
    asset: A,
    backing_account: &BackingAccount<R, B>,
    commodity: &str,
    description: &str,
    amount: f64,
) -> Result<Vec<LedgerTransaction>, ServerError>
where
    A: AssetHandler,
    R: ReimbursableEntityHandler,
    B: CashHandler,
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
pub fn process_fixed_expense<E, R, B>(
    accrual_date: NaiveDate,
    payment_date: NaiveDate,
    until_date: NaiveDate,
    expense: E,
    backing_account: &BackingAccount<R, B>,
    commodity: &str,
    description: &str,
    amount: f64,
) -> Result<Vec<LedgerTransaction>, ServerError>
where
    E: ExpenseHandler,
    R: ReimbursableEntityHandler,
    B: CashHandler,
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
pub fn process_variable_expense<E, R, B>(
    accrual_date: NaiveDate,
    payment_date: NaiveDate,
    until_date: NaiveDate,
    expense: E,
    backing_account: &BackingAccount<R, B>,
    commodity: &str,
    description: &str,
    amount: f64,
    var_history: &mut HashMap<Expense, Vec<(NaiveDate, f64)>>,
) -> Result<Vec<LedgerTransaction>, ServerError>
where
    E: ExpenseHandler,
    R: ReimbursableEntityHandler,
    B: CashHandler,
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
pub fn process_variable_expense_init<E, R, B>(
    accrual_date: NaiveDate,
    payment_date: NaiveDate,
    until_date: NaiveDate,
    expense: E,
    estimate: i64,
    backing_account: &BackingAccount<R, B>,
    commodity: &str,
    description: &str,
    _amount: f64,
    var_history: &mut HashMap<Expense, Vec<(NaiveDate, f64)>>,
) -> Result<Vec<LedgerTransaction>, ServerError>
where
    E: ExpenseHandler,
    R: ReimbursableEntityHandler,
    B: CashHandler,
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
pub fn process_immaterial_income<I, R, B>(
    payment_date: NaiveDate,
    income: I,
    backing_account: &BackingAccount<R, B>,
    commodity: &str,
    description: &str,
    amount: f64,
) -> Result<Vec<LedgerTransaction>, ServerError>
where
    I: IncomeHandler,
    R: ReimbursableEntityHandler,
    B: CashHandler,
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
pub fn process_immaterial_expense<E, R, B>(
    payment_date: NaiveDate,
    expense: E,
    backing_account: &BackingAccount<R, B>,
    commodity: &str,
    description: &str,
    amount: f64,
) -> Result<Vec<LedgerTransaction>, ServerError>
where
    E: ExpenseHandler,
    R: ReimbursableEntityHandler,
    B: CashHandler,
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
pub fn process_reimburse<R, B>(
    payment_date: NaiveDate,
    reimbursable: R,
    backing_account: &BackingAccount<R, B>,
    commodity: &str,
    description: &str,
    amount: f64,
) -> Result<Vec<LedgerTransaction>, ServerError>
where
    R: ReimbursableEntityHandler,
    B: CashHandler,
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
    backing_account: Asset,
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
                account: Asset("VatReceivable".into()).into(),
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
                            account: Liability("VatReceiptPending".into()).into(),
                            amount: vat_amount,
                            commodity: commodity.to_string(),
                        },
                        Posting {
                            account: Expense("VatSubtraction".into()).into(),
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
                            account: Asset("VatReceivable".into()).into(),
                            amount: vat_amount,
                            commodity: commodity.to_string(),
                        },
                        Posting {
                            account: Liability("VatReceiptPending".into()).into(),
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
                            account: Expense("VatUnrecoverable".into()).into(),
                            amount: vat_amount,
                            commodity: commodity.to_string(),
                        },
                        Posting {
                            account: Expense("VatSubtraction".into()).into(),
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
                    Income("FxGain".into()).into()
                } else {
                    Expense("FxLoss".into()).into()
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
                            account: Expense("FxSubtraction".into()).into(),
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
                            account: Expense("PaymentFees".into()).into(),
                            amount: *fee,
                            commodity: commodity.to_string(),
                        },
                        Posting {
                            account: Asset("Cash:Hana".into()).into(),
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

// ---------------------------------------------------------------------
// Main processing function.
// Reads CSV content and returns (ledger_output, list of notes).
// ---------------------------------------------------------------------

#[derive(Debug)]
pub struct AccountingAmount(pub f64);
impl FromStr for AccountingAmount {
    type Err = ServerError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let raw = s.replace(",", "");
        let is_negative = raw.trim().starts_with("(") && raw.trim().ends_with(")");
        let numeric_part = raw.trim_matches(|c| c == '(' || c == ')');
        let amount = numeric_part
            .parse::<f64>()
            .map_err(|_| InvalidAccountingAmount::new(numeric_part))?;
        Ok(AccountingAmount(if is_negative { -amount } else { amount }))
    }
}

pub struct CsvRecord<E, A, I, R, B>
where
    E: ExpenseHandler,
    A: AssetHandler,
    I: IncomeHandler,
    R: ReimbursableEntityHandler,
    B: CashHandler,
{
    pub accrual_date: ISODate,
    pub until: Option<ISODate>,
    pub payment_date: ISODate,
    pub accounting_logic: AccountingLogic<E, A, I, R>,
    pub decorators: Vec<Decorator>,
    pub entity: String,
    pub description: String,
    pub amount: AccountingAmount,
    pub commodity: String,
    pub backing_account: BackingAccount<R, B>,
}

impl<E, A, I, R, B> CsvRecord<E, A, I, R, B>
where
    E: ExpenseHandler + for<'de> Deserialize<'de> + std::fmt::Debug,
    A: AssetHandler + for<'de> Deserialize<'de> + std::fmt::Debug,
    I: IncomeHandler + for<'de> Deserialize<'de> + std::fmt::Debug,
    R: ReimbursableEntityHandler + for<'de> Deserialize<'de> + std::fmt::Debug,
    B: CashHandler + for<'de> Deserialize<'de> + std::fmt::Debug,
{
    pub fn parse(
        accrual_date: &str,
        until: Option<&str>,
        payment_date: &str,
        accounting_logic: &str,
        decorators: &str,
        entity: impl Into<String>,
        description: impl Into<String>,
        amount: &str,
        commodity: impl Into<String>,
        backing_account: &str,
    ) -> Result<Self, ServerError> {
        Ok(Self {
            accrual_date: ISODate::from_str(accrual_date)?,
            until: until.map(ISODate::from_str).transpose()?,
            payment_date: ISODate::from_str(payment_date)?,
            accounting_logic: from_str(accounting_logic)
                .map_err(|e| InvalidRon::with_debug("AccountingLogic", &e))?,
            decorators: from_str(&format!("[{}]", decorators))
                .map_err(|e| InvalidRon::with_debug("Decorator", &e))?,
            entity: entity.into(),
            description: description.into(),
            amount: AccountingAmount::from_str(amount)?,
            commodity: commodity.into(),
            backing_account: from_str(backing_account)
                .map_err(|e| InvalidRon::with_debug("BackingAccount", &e))?,
        })
    }
}

pub fn process_csv<E, A, I, R, B>(csv_content: &str) -> Result<(String, Vec<String>), ServerError>
where
    E: ExpenseHandler + for<'de> Deserialize<'de> + std::fmt::Debug,
    A: AssetHandler + for<'de> Deserialize<'de> + std::fmt::Debug,
    I: IncomeHandler + for<'de> Deserialize<'de> + std::fmt::Debug,
    R: ReimbursableEntityHandler + for<'de> Deserialize<'de> + std::fmt::Debug,
    B: CashHandler + for<'de> Deserialize<'de> + std::fmt::Debug,
{
    let mut ledger_transactions: Vec<LedgerTransaction> = Vec::new();
    let mut global_notes: Vec<String> = Vec::new();
    // For VariableExpense historical data.
    let mut var_exp_history: HashMap<Expense, Vec<(NaiveDate, f64)>> = HashMap::new();

    let mut rdr = csv::Reader::from_reader(csv_content.as_bytes());
    for result in rdr.records() {
        let record = result.map_err(|e| InvalidCsv::with_debug(&e))?;
        // Map CSV columns (order as specified):
        let csv_record = CsvRecord::<E, A, I, R, B>::parse(
            record.get(0).unwrap_or(""),
            match record.get(1) {
                Some(s) if !s.is_empty() => Some(s),
                _ => None,
            },
            record.get(2).unwrap_or(""),
            record.get(3).unwrap_or(""),
            record.get(4).unwrap_or(""),
            record.get(5).unwrap_or(""),
            record.get(6).unwrap_or(""),
            record.get(7).unwrap_or("0"),
            record.get(8).unwrap_or(""),
            record.get(9).unwrap_or(""),
        )?;

        let mut txs: Vec<LedgerTransaction> = Vec::new();
        // Process based on accounting logic.
        match csv_record.accounting_logic {
            AccountingLogic::SimpleExpense(expense) => {
                if csv_record.until.is_some() {
                    return Err(AccrualRangeNotSupported::new(
                        "SimpleExpense",
                        &csv_record.description,
                    ));
                }
                let transactions = process_simple_expense(
                    csv_record.accrual_date.0,
                    csv_record.payment_date.0,
                    expense,
                    &csv_record.backing_account,
                    &csv_record.commodity,
                    &csv_record.description,
                    csv_record.amount.0,
                )?;
                txs.extend(transactions);
            }
            AccountingLogic::Capitalize(asset) => {
                if csv_record.until.is_some() {
                    return Err(AccrualRangeNotSupported::new(
                        "Capitalize",
                        &csv_record.description,
                    ));
                }
                let transactions = process_capitalize(
                    csv_record.accrual_date.0,
                    csv_record.payment_date.0,
                    asset,
                    &csv_record.backing_account,
                    &csv_record.commodity,
                    &csv_record.description,
                    csv_record.amount.0,
                )?;
                txs.extend(transactions);
            }
            AccountingLogic::Amortize(asset) => {
                let transactions = process_amortize(
                    csv_record.accrual_date.0,
                    csv_record.payment_date.0,
                    csv_record
                        .until
                        .ok_or_else(|| {
                            AccrualRangeRequired::new("Amortize", &csv_record.description)
                        })?
                        .0,
                    asset,
                    &csv_record.backing_account,
                    &csv_record.commodity,
                    &csv_record.description,
                    csv_record.amount.0,
                )?;
                txs.extend(transactions);
            }
            AccountingLogic::FixedExpense(expense) => {
                let transactions = process_fixed_expense(
                    csv_record.accrual_date.0,
                    csv_record.payment_date.0,
                    csv_record
                        .until
                        .ok_or_else(|| {
                            AccrualRangeRequired::new("FixedExpense", &csv_record.description)
                        })?
                        .0,
                    expense,
                    &csv_record.backing_account,
                    &csv_record.commodity,
                    &csv_record.description,
                    csv_record.amount.0,
                )?;
                txs.extend(transactions);
            }
            AccountingLogic::VariableExpense(expense) => {
                let transactions = process_variable_expense(
                    csv_record.accrual_date.0,
                    csv_record.payment_date.0,
                    csv_record
                        .until
                        .ok_or_else(|| {
                            AccrualRangeRequired::new("VariableExpense", &csv_record.description)
                        })?
                        .0,
                    expense,
                    &csv_record.backing_account,
                    &csv_record.commodity,
                    &csv_record.description,
                    csv_record.amount.0,
                    &mut var_exp_history,
                )?;
                txs.extend(transactions);
            }
            AccountingLogic::VariableExpenseInit { account, estimate } => {
                let transactions = process_variable_expense_init(
                    csv_record.accrual_date.0,
                    csv_record.payment_date.0,
                    csv_record
                        .until
                        .ok_or_else(|| {
                            AccrualRangeRequired::new(
                                "VariableExpenseInit",
                                &csv_record.description,
                            )
                        })?
                        .0,
                    account,
                    estimate,
                    &csv_record.backing_account,
                    &csv_record.commodity,
                    &csv_record.description,
                    csv_record.amount.0,
                    &mut var_exp_history,
                )?;
                txs.extend(transactions);
            }
            AccountingLogic::ImmaterialIncome(income) => {
                let transactions = process_immaterial_income(
                    csv_record.payment_date.0,
                    income,
                    &csv_record.backing_account,
                    &csv_record.commodity,
                    &csv_record.description,
                    csv_record.amount.0,
                )?;
                txs.extend(transactions);
                global_notes.push(format!(
                    "Immaterial income applied: {}",
                    csv_record.description
                ));
            }
            AccountingLogic::ImmaterialExpense(expense) => {
                let transactions = process_immaterial_expense(
                    csv_record.payment_date.0,
                    expense,
                    &csv_record.backing_account,
                    &csv_record.commodity,
                    &csv_record.description,
                    csv_record.amount.0,
                )?;
                txs.extend(transactions);
                global_notes.push(format!(
                    "Immaterial expense applied: {}",
                    csv_record.description
                ));
            }
            AccountingLogic::Reimburse(reimbursable) => {
                let transactions = process_reimburse(
                    csv_record.payment_date.0,
                    reimbursable,
                    &csv_record.backing_account,
                    &csv_record.commodity,
                    &csv_record.description,
                    csv_record.amount.0,
                )?;
                txs.extend(transactions);
            }
            AccountingLogic::ClearVat { from, to } => {
                let transactions = process_clear_vat(
                    csv_record.accrual_date.0,
                    match csv_record.backing_account {
                        BackingAccount::Cash(b) => Ok(b.account()),
                        _ => Err(ClearVatInvalidBackingAccount::with_debug(
                            &csv_record.description,
                            &csv_record.backing_account,
                        )),
                    }?,
                    &csv_record.commodity,
                    &csv_record.description,
                    csv_record.amount.0,
                    from.0,
                    to.0,
                )?;
                txs.extend(transactions);
            }
        }
        // Process decorators.
        let side_txs = apply_decorators(
            &mut txs,
            &csv_record.decorators,
            csv_record.accrual_date.0,
            csv_record.payment_date.0,
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
