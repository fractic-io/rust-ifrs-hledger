use fractic_server_error::ServerError;

use crate::{
    data::datasources::{
        balances_csv_datasource::{BalancesCsvDatasource, BalancesCsvDatasourceImpl},
        transactions_csv_datasource::{TransactionsCsvDatasource, TransactionsCsvDatasourceImpl},
    },
    domain::repositories::records_repository::RecordsRepository,
    entities::{
        AssetHandler, CashHandler, CommodityHandler, DecoratorHandler, ExpenseHandler,
        FinancialRecordSpecs, IncomeHandler, ReimbursableEntityHandler,
    },
};

pub(crate) struct RecordsRepositoryImpl<A, I, E, C, R, D, M, DS1, DS2>
where
    A: AssetHandler,
    I: IncomeHandler,
    E: ExpenseHandler,
    C: CashHandler,
    R: ReimbursableEntityHandler,
    D: DecoratorHandler,
    M: CommodityHandler,
    DS1: BalancesCsvDatasource<C, M>,
    DS2: TransactionsCsvDatasource<A, I, E, C, R, D, M>,
{
    transactions_datasource: DS2,
    balances_datasource: DS1,
    _phantom: std::marker::PhantomData<(A, I, E, C, R, D, M)>,
}

impl<A, I, E, C, R, D, M, DS1, DS2> RecordsRepository<A, I, E, C, R, D, M>
    for RecordsRepositoryImpl<A, I, E, C, R, D, M, DS1, DS2>
where
    A: AssetHandler,
    I: IncomeHandler,
    E: ExpenseHandler,
    C: CashHandler,
    R: ReimbursableEntityHandler,
    D: DecoratorHandler,
    M: CommodityHandler,
    DS1: BalancesCsvDatasource<C, M>,
    DS2: TransactionsCsvDatasource<A, I, E, C, R, D, M>,
{
    fn from_string(
        &self,
        transactions_csv: &str,
        balances_csv: &str,
    ) -> Result<FinancialRecordSpecs<A, I, E, C, R, D, M>, ServerError> {
        Ok(FinancialRecordSpecs {
            transaction_specs: self.transactions_datasource.from_string(transactions_csv)?,
            assertion_specs: self.balances_datasource.from_string(balances_csv)?,
        })
    }

    fn from_file<P>(
        &self,
        transactions_csv: P,
        balances_csv: P,
    ) -> Result<FinancialRecordSpecs<A, I, E, C, R, D, M>, ServerError>
    where
        P: AsRef<std::path::Path>,
    {
        Ok(FinancialRecordSpecs {
            transaction_specs: self.transactions_datasource.from_file(transactions_csv)?,
            assertion_specs: self.balances_datasource.from_file(balances_csv)?,
        })
    }
}

impl<A, I, E, C, R, D, M>
    RecordsRepositoryImpl<
        A,
        I,
        E,
        C,
        R,
        D,
        M,
        BalancesCsvDatasourceImpl<C, M>,
        TransactionsCsvDatasourceImpl<A, I, E, C, R, D, M>,
    >
where
    A: AssetHandler,
    I: IncomeHandler,
    E: ExpenseHandler,
    C: CashHandler,
    R: ReimbursableEntityHandler,
    D: DecoratorHandler,
    M: CommodityHandler,
{
    pub(crate) fn new() -> Self {
        RecordsRepositoryImpl {
            transactions_datasource: TransactionsCsvDatasourceImpl::new(),
            balances_datasource: BalancesCsvDatasourceImpl::new(),
            _phantom: std::marker::PhantomData,
        }
    }
}
