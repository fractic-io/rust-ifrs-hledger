use fractic_server_error::ServerError;

use crate::{
    data::{
        datasources::{
            balances_csv_datasource::BalancesCsvDatasourceImpl,
            transactions_csv_datasource::TransactionsCsvDatasourceImpl,
        },
        repositories::records_repository_impl::RecordsRepositoryImpl,
    },
    domain::{
        logic::ifrs_logic::{IfrsLogic, IfrsLogicImpl},
        repositories::records_repository::RecordsRepository,
    },
    entities::{FinancialRecords, Handlers},
};

pub trait ProcessUsecase {
    fn from_string(
        &self,
        balances_csv: &str,
        transactions_csv: &str,
    ) -> Result<FinancialRecords, ServerError>;

    fn from_file<P>(
        &self,
        balances_csv: P,
        transactions_csv: P,
    ) -> Result<FinancialRecords, ServerError>
    where
        P: AsRef<std::path::Path>;
}

pub(crate) struct ProcessUsecaseImpl<H: Handlers, R1, L1>
where
    R1: RecordsRepository<H>,
    L1: IfrsLogic<H>,
{
    records_repository: R1,
    ifrs_logic: L1,
    _phantom: std::marker::PhantomData<H>,
}

impl<H: Handlers, R1, L1> ProcessUsecase for ProcessUsecaseImpl<H, R1, L1>
where
    R1: RecordsRepository<H>,
    L1: IfrsLogic<H>,
{
    fn from_string(
        &self,
        transactions_csv: &str,
        balances_csv: &str,
    ) -> Result<FinancialRecords, ServerError> {
        let specs = self
            .records_repository
            .from_string(transactions_csv, balances_csv)?;
        Ok(self.ifrs_logic.process(specs))
    }

    fn from_file<P>(
        &self,
        transactions_csv: P,
        balances_csv: P,
    ) -> Result<FinancialRecords, ServerError>
    where
        P: AsRef<std::path::Path>,
    {
        let specs = self
            .records_repository
            .from_file(transactions_csv, balances_csv)?;
        Ok(self.ifrs_logic.process(specs))
    }
}

impl<H: Handlers>
    ProcessUsecaseImpl<
        H,
        RecordsRepositoryImpl<H, BalancesCsvDatasourceImpl<H>, TransactionsCsvDatasourceImpl<H>>,
        IfrsLogicImpl<H>,
    >
{
    pub(crate) fn new() -> Self {
        ProcessUsecaseImpl {
            records_repository: RecordsRepositoryImpl::new(),
            ifrs_logic: IfrsLogicImpl::new(),
            _phantom: std::marker::PhantomData,
        }
    }
}
