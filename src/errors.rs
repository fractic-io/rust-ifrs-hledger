use std::process::Command;

use chrono::NaiveDate;
use fractic_server_error::{define_client_error, define_internal_error};

use crate::entities::{LiabilityAccount, TransactionSpecId};

// IO-related.
define_client_error!(ReadError, "Error reading file.");

// Parsing-related.
define_client_error!(InvalidCsv, "Invalid CSV format.");
define_client_error!(
    InvalidCsvContent,
    "@ Line {line_id}: Invalid CSV content: {details}.",
    { line_id: u64, details: &str }
);
define_client_error!(InvalidRon, "Invalid {ron_type} (invalid RON format).", { ron_type: &str });
define_client_error!(InvalidIsoDate, "Invalid ISO date: {date}.", { date: &str });
define_client_error!(InvalidIsoCurrencyCode, "Invalid ISO currency code: {code}.", { code: &str });
define_client_error!(
    InvalidAccountingAmount,
    "Invalid accounting amount: '{value}'.",
    { value: &str }
);

// Accounting-related.
define_client_error!(
    CommonStockCannotBePrepaid,
    "@ Line {spec_id}; CommonStock: '{description}' cannot have a payment date before accrual, since this would indicate prepayment for stock.",
    { spec_id: &TransactionSpecId, description: &str }
);
define_client_error!(
    NonAmortizableAsset,
    "Asset '{name}' does not have any value defined for 'upon_accrual()'. Provide a non-None value to support amortization.",
    { name: &str }
);
define_client_error!(
    VariableExpenseInvalidPaymentDate,
    "@ Line {spec_id}; Invalid VariableExpense: '{description}'. Payment date ({payment_date}) must be after accrual period (accrual end: {until_date}), otherwise it would indicate we're prepaying for an unknown expense.",
    { spec_id: &TransactionSpecId, description: &str, payment_date: &NaiveDate, until_date: &NaiveDate }
);
define_client_error!(
    VariableExpenseNotEnoughHistoricalData,
    "@ Line {spec_id}; No historical data for VariableExpense: '{description}' in the previous 90 days.",
    { spec_id: &TransactionSpecId, description: &str }
);
define_client_error!(
    VariableExpenseNoInit,
    "@ Line {spec_id}; VariableExpense: '{description}' not initialized. Must initiate with a VariableExpenseInit entry.",
    { spec_id: &TransactionSpecId, description: &str }
);
define_client_error!(
    VariableExpenseDoubleInit,
    "@ Line {spec_id}; VariableExpense: '{description}' already initialized. Cannot initialize twice.",
    { spec_id: &TransactionSpecId, description: &str }
);
define_client_error!(
    ClearVatInvalidBackingAccount,
    "@ Line {spec_id}; ClearVat entry '{description}' requires a Cash backing account.",
    { spec_id: &TransactionSpecId, description: &str }
);
define_client_error!(
    InvalidArgumentsForAccountingLogic,
    "@ Line {spec_id}; Invalid arguments provided for accounting logic type.",
    { spec_id: &TransactionSpecId }
);
define_client_error!(
    UnexpectedNegativeValue,
    "@ Line {spec_id}; Unexpected negative amount ({amount}) for '{accounting_logic}' accounting logic.",
    { amount: f64, accounting_logic: &str, spec_id: &TransactionSpecId }
);
define_client_error!(
    UnexpectedPositiveValue,
    "@ Line {spec_id}; Unexpected positive amount ({amount}) for '{accounting_logic}' accounting logic.",
    { amount: f64, accounting_logic: &str, spec_id: &TransactionSpecId }
);
define_internal_error!(
    ReimbursementTracingError,
    "Error tracing reimbursements: {details}.",
    { details: &str }
);
define_client_error!(
    NoTransactionsToReimburse,
    "@ Line {spec_id}; Reimburse spec can't be mapped to any unreimbursed transactions for '{account:?}'.",
    { spec_id: &TransactionSpecId, account: &LiabilityAccount }
);
define_client_error!(
    UnexpectedPartialReimbursement,
    "@ Line {spec_id}; Reimburse spec unexpectedly leaves an unreimbursed amount of {amount} for '{account:?}'.",
    { spec_id: &TransactionSpecId, account: &LiabilityAccount, amount: f64 }
);

// Hledger-related.
define_client_error!(
    HledgerInvalidPath,
    "Invalid path to hledger ledger file: '{ledger}'.",
    { ledger: &str }
);
define_internal_error!(
    HledgerCommandFailed,
    "hledger command failed for ledger '{ledger}':\n\n{command:?}",
    { ledger: &str, command: &Command }
);
define_internal_error!(
    HledgerQueryInvalidResponse,
    "hledger command returned an unexpected response. Could not parse total change during the given period:\n\n{command:?}\n\nQuery: {query}\n\nReturn: {fetch}",
    { command: &Command, query: String, fetch: String }
);
define_internal_error!(
    HledgerCloseInvalidResponse,
    "'hledger close' returned an unexpected response: {details}.",
    { details: String }
);

// Custom statement generation.
define_client_error!(
    InvalidCashFlowStatementPeriods,
    "The cash flow statement requires at least 1 period."
);
define_internal_error!(
    UnreplacedPlaceholdersRemain,
    "Unexpected placeholders remain: {unreplaced:?}.",
    { unreplaced: &Vec<String> }
);

// Derived record generation.
define_client_error!(
    NoAccountsToClose,
    "No income/expense accounts to close for year {year}. Does the ledger already have a close entry for {year}?",
    { year: i32 }
);
