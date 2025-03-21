use fractic_server_error::ServerError;

use crate::{
    domain::usecases::process_usecase::{ProcessUsecase as _, ProcessUsecaseImpl},
    entities::{
        AssetHandler, CashHandler, CommodityHandler, DecoratorHandler, ExpenseHandler,
        FinancialRecords, HandlersImpl, IncomeHandler, NotesToFinancialRecords, PayeeHandler,
        ReimbursableEntityHandler, ShareholderHandler,
    },
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
    ) -> Result<(FinancialRecords, NotesToFinancialRecords, Ledger), ServerError> {
        let (financial_records, notes_to_financial_records) = self
            .process_usecase
            .from_string(transactions_csv, balances_csv)
            .await?;
        let ledger = self.printer.print_ledger(&financial_records);
        Ok((financial_records, notes_to_financial_records, ledger))
    }

    pub async fn from_file<T>(
        &self,
        transactions_csv: T,
        balances_csv: T,
    ) -> Result<(FinancialRecords, NotesToFinancialRecords, Ledger), ServerError>
    where
        T: AsRef<std::path::Path> + Send,
    {
        let (financial_records, notes_to_financial_records) = self
            .process_usecase
            .from_file(transactions_csv, balances_csv)
            .await?;
        let ledger = self.printer.print_ledger(&financial_records);
        Ok((financial_records, notes_to_financial_records, ledger))
    }
}
