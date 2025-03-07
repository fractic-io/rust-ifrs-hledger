use fractic_server_error::ServerError;

use crate::entities::{
    AssetHandler, CashHandler, CommodityHandler, DecoratorHandler, ExpenseHandler,
    FinancialRecordSpecs, IncomeHandler, ReimbursableEntityHandler,
};

pub trait RecordsRepository<A, I, E, C, R, D, M>
where
    A: AssetHandler,
    I: IncomeHandler,
    E: ExpenseHandler,
    C: CashHandler,
    R: ReimbursableEntityHandler,
    D: DecoratorHandler,
    M: CommodityHandler,
{
    fn from_string(
        &self,
        transactions_csv: &str,
        balances_csv: &str,
    ) -> Result<FinancialRecordSpecs<A, I, E, C, R, D, M>, ServerError>;

    fn from_file<P>(
        &self,
        transactions_csv: P,
        balances_csv: P,
    ) -> Result<FinancialRecordSpecs<A, I, E, C, R, D, M>, ServerError>
    where
        P: AsRef<std::path::Path>;
}
