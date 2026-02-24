use fractic_server_error::ServerError;
use futures::future::join_all;

use crate::{
    domain::usecases::process_usecase::{ProcessUsecase as _, ProcessUsecaseImpl},
    entities::{
        AssetHandler, CashHandler, CommodityHandler, DecoratorHandler, ExpenseHandler,
        FinancialRecords, HandlersImpl, IncomeHandler, NotesToFinancialRecords, PayeeHandler,
        ReimbursableEntityHandler, ShareholderHandler,
    },
    errors::ReadError,
    presentation::hledger_printer::HledgerPrinter,
};

pub type Ledger = String;

pub struct IfrsHledgerUtil<A, I, E, C, S, R, D, M, P>
where
    A: AssetHandler,
    I: IncomeHandler,
    E: ExpenseHandler,
    R: ReimbursableEntityHandler,
    C: CashHandler,
    S: ShareholderHandler,
    D: DecoratorHandler,
    M: CommodityHandler,
    P: PayeeHandler,
{
    process_usecase: ProcessUsecaseImpl<HandlersImpl<A, I, E, R, C, S, D, M, P>>,
    printer: HledgerPrinter,
}

impl<A, I, E, C, S, R, D, M, P> IfrsHledgerUtil<A, I, E, C, S, R, D, M, P>
where
    A: AssetHandler,
    I: IncomeHandler,
    E: ExpenseHandler,
    C: CashHandler,
    S: ShareholderHandler,
    R: ReimbursableEntityHandler,
    D: DecoratorHandler,
    M: CommodityHandler,
    P: PayeeHandler,
{
    pub fn new() -> Self {
        Self {
            process_usecase: ProcessUsecaseImpl::new(),
            printer: HledgerPrinter::new(),
        }
    }

    pub async fn from_string(
        &self,
        transactions_csv: &str,
        balances_csv: &str,
        prev_close: impl IntoIterator<Item = &'_ str>,
    ) -> Result<(FinancialRecords, NotesToFinancialRecords, Ledger), ServerError> {
        let (financial_records, notes_to_financial_records) = self
            .process_usecase
            .from_string(transactions_csv, balances_csv)
            .await?;
        let ledger = self.printer.print_ledger(&financial_records, prev_close);
        Ok((financial_records, notes_to_financial_records, ledger))
    }

    pub async fn from_file<T, U>(
        &self,
        transactions_csv: T,
        balances_csv: T,
        prev_close: impl IntoIterator<Item = U>,
    ) -> Result<(FinancialRecords, NotesToFinancialRecords, Ledger), ServerError>
    where
        T: AsRef<std::path::Path> + Send,
        U: AsRef<std::path::Path>,
    {
        let (financial_records, notes_to_financial_records) = self
            .process_usecase
            .from_file(transactions_csv, balances_csv)
            .await?;

        let prev_close_contents = join_all(prev_close.into_iter().map(|path| async move {
            tokio::fs::read_to_string(path.as_ref())
                .await
                .map_err(|e| ServerError::from(ReadError::with_debug(&e)))
        }))
        .await
        .into_iter()
        .collect::<Result<Vec<_>, ServerError>>()?;

        let ledger = self.printer.print_ledger(
            &financial_records,
            prev_close_contents.iter().map(String::as_str),
        );
        Ok((financial_records, notes_to_financial_records, ledger))
    }
}
