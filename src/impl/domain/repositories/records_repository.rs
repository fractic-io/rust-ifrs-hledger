use fractic_server_error::ServerError;

use crate::entities::{FinancialRecordSpecs, Handlers};

pub trait RecordsRepository<H: Handlers> {
    fn from_string(
        &self,
        transactions_csv: &str,
        balances_csv: &str,
    ) -> Result<FinancialRecordSpecs<H>, ServerError>;

    fn from_file<P>(
        &self,
        transactions_csv: P,
        balances_csv: P,
    ) -> Result<FinancialRecordSpecs<H>, ServerError>
    where
        P: AsRef<std::path::Path>;
}
