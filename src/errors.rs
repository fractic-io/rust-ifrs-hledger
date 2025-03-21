use chrono::NaiveDate;
use fractic_server_error::define_client_error;

use crate::entities::TransactionSpecId;

// IO-related.
define_client_error!(ReadError, "Error reading file.");

// Parsing-related.
define_client_error!(InvalidCsv, "Invalid CSV format.");
define_client_error!(InvalidCsvContent, "Invalid CSV content: {details}.", { details: &str });
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
    "CommonStock: '{description}' cannot have a payment date before accrual, since this would indicate prepayment for stock.",
    { description: &str }
);
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
    VariableExpenseNotEnoughHistoricalData,
    "No historical data for VariableExpense: '{description}' in the previous 90 days.",
    { description: &str }
);
define_client_error!(
    VariableExpenseNoInit,
    "VariableExpense: '{description}' not initialized. Must initiate with a VariableExpenseInit entry.",
    { description: &str }
);
define_client_error!(
    VariableExpenseDoubleInit,
    "VariableExpense: '{description}' already initialized. Cannot initialize twice.",
    { description: &str }
);
define_client_error!(
    ClearVatInvalidBackingAccount,
    "ClearVat entry '{description}' requires a Cash backing account.",
    { description: &str }
);
define_client_error!(
    InvalidArgumentsForAccountingLogic,
    "Invalid arguments provided for accounting logic type."
);
define_client_error!(
    UnexpectedNegativeValue,
    "Unexpected negative amount ({amount}) for '{accounting_logic}' accounting logic (id: {spec_id:?}).",
    { amount: f64, accounting_logic: &str, spec_id: &TransactionSpecId }
);
define_client_error!(
    UnexpectedPositiveValue,
    "Unexpected positive amount ({amount}) for '{accounting_logic}' accounting logic (id: {spec_id:?}).",
    { amount: f64, accounting_logic: &str, spec_id: &TransactionSpecId }
);
