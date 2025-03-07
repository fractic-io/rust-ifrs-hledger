use fractic_server_error::ServerError;

use crate::{
    data::datasources::{
        balances_csv_datasource::{BalancesCsvDatasource, BalancesCsvDatasourceImpl},
        transactions_csv_datasource::{TransactionsCsvDatasource, TransactionsCsvDatasourceImpl},
    },
    domain::repositories::records_repository::RecordsRepository,
    entities::{FinancialRecordSpecs, Handlers},
};

pub(crate) struct RecordsRepositoryImpl<
    H,
    DS1 = BalancesCsvDatasourceImpl<H>,     // Default.
    DS2 = TransactionsCsvDatasourceImpl<H>, // Default.
> where
    H: Handlers,
    DS1: BalancesCsvDatasource<H>,
    DS2: TransactionsCsvDatasource<H>,
{
    transactions_datasource: DS2,
    balances_datasource: DS1,
    _phantom: std::marker::PhantomData<H>,
}

impl<H, DS1, DS2> RecordsRepository<H> for RecordsRepositoryImpl<H, DS1, DS2>
where
    H: Handlers,
    DS1: BalancesCsvDatasource<H>,
    DS2: TransactionsCsvDatasource<H>,
{
    fn from_string(
        &self,
        transactions_csv: &str,
        balances_csv: &str,
    ) -> Result<FinancialRecordSpecs<H>, ServerError> {
        Ok(FinancialRecordSpecs {
            transaction_specs: self.transactions_datasource.from_string(transactions_csv)?,
            assertion_specs: self.balances_datasource.from_string(balances_csv)?,
        })
    }

    fn from_file<P>(
        &self,
        transactions_csv: P,
        balances_csv: P,
    ) -> Result<FinancialRecordSpecs<H>, ServerError>
    where
        P: AsRef<std::path::Path>,
    {
        Ok(FinancialRecordSpecs {
            transaction_specs: self.transactions_datasource.from_file(transactions_csv)?,
            assertion_specs: self.balances_datasource.from_file(balances_csv)?,
        })
    }
}

impl<H: Handlers> RecordsRepositoryImpl<H> {
    pub(crate) fn new() -> Self {
        RecordsRepositoryImpl {
            transactions_datasource: TransactionsCsvDatasourceImpl::new(),
            balances_datasource: BalancesCsvDatasourceImpl::new(),
            _phantom: std::marker::PhantomData,
        }
    }
}
