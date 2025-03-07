use chrono::NaiveDate;
use fractic_server_error::define_client_error;

// IO-related.
define_client_error!(ReadError, "Error reading file.");

// Parsing-related.
define_client_error!(InvalidCsv, "Invalid CSV format.");
define_client_error!(InvalidRon, "Invalid {ron_type} (invalid RON format).", { ron_type: &str });
define_client_error!(InvalidIsoDate, "Invalid ISO date: {date}.", { date: &str });
define_client_error!(
    InvalidAccountingAmount,
    "Invalid accounting amount: '{value}'.",
    { value: &str }
);

// Accounting-related.
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
