use async_trait::async_trait;
use fractic_server_error::ServerError;

use crate::entities::{FinancialRecordSpecs, Handlers};

#[async_trait]
pub trait RecordsRepository<H: Handlers>: Send + Sync {
    fn from_string(
        &self,
        transactions_csv: &str,
        balances_csv: &str,
    ) -> Result<FinancialRecordSpecs<H>, ServerError>;

    async fn from_file<P>(
        &self,
        transactions_csv: P,
        balances_csv: P,
    ) -> Result<FinancialRecordSpecs<H>, ServerError>
    where
        P: AsRef<std::path::Path> + Send;
}
