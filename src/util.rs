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
        logic::ifrs_logic::IfrsLogicImpl,
        usecases::process_usecase::{ProcessUsecase as _, ProcessUsecaseImpl},
    },
    entities::{
        AssetHandler, CashHandler, CommodityHandler, DecoratorHandler, ExpenseHandler,
        FinancialRecords, Handlers, HandlersImpl, IncomeHandler, ReimbursableEntityHandler,
    },
    presentation::hledger_printer::HledgerPrinter,
};

pub type Ledger = String;

pub struct IfrsHledgerUtil<A, I, E, C, R, D, M, H = HandlersImpl<A, I, E, R, C, D, M>>
where
    A: AssetHandler,
    I: IncomeHandler,
    E: ExpenseHandler,
    R: ReimbursableEntityHandler,
    C: CashHandler,
    D: DecoratorHandler,
    M: CommodityHandler,
    H: Handlers,
{
    process_usecase: ProcessUsecaseImpl<
        H,
        RecordsRepositoryImpl<H, BalancesCsvDatasourceImpl<H>, TransactionsCsvDatasourceImpl<H>>,
        IfrsLogicImpl<H>,
    >,
    printer: HledgerPrinter,
    _phantom: std::marker::PhantomData<(A, I, E, R, C, D, M)>,
}

impl<A, I, E, C, R, D, M> IfrsHledgerUtil<A, I, E, C, R, D, M>
where
    A: AssetHandler,
    I: IncomeHandler,
    E: ExpenseHandler,
    C: CashHandler,
    R: ReimbursableEntityHandler,
    D: DecoratorHandler,
    M: CommodityHandler,
{
    pub fn new() -> Self {
        Self {
            process_usecase: ProcessUsecaseImpl::new(),
            printer: HledgerPrinter::new(),
            _phantom: std::marker::PhantomData,
        }
    }

    pub fn from_string(
        &self,
        transactions_csv: &str,
        balances_csv: &str,
    ) -> Result<(Ledger, FinancialRecords), ServerError> {
        let financial_records = self
            .process_usecase
            .from_string(transactions_csv, balances_csv)?;
        Ok((
            self.printer.print_ledger(&financial_records),
            financial_records,
        ))
    }

    pub fn from_file<P>(
        &self,
        transactions_csv: P,
        balances_csv: P,
    ) -> Result<(Ledger, FinancialRecords), ServerError>
    where
        P: AsRef<std::path::Path>,
    {
        let financial_records = self
            .process_usecase
            .from_file(transactions_csv, balances_csv)?;
        Ok((
            self.printer.print_ledger(&financial_records),
            financial_records,
        ))
    }
}
