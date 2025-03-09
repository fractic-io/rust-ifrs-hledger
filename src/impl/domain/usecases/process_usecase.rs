use fractic_server_error::ServerError;

use crate::{
    data::repositories::records_repository_impl::RecordsRepositoryImpl,
    domain::{
        logic::{
            annotation_processor::AnnotationProcessor, decorator_processor::DecoratorProcessor,
            spec_processor::SpecProcessor,
        },
        repositories::records_repository::RecordsRepository,
    },
    entities::{FinancialRecords, Handlers, NotesToFinancialRecords},
};

pub trait ProcessUsecase {
    fn from_string(
        &self,
        balances_csv: &str,
        transactions_csv: &str,
    ) -> Result<(FinancialRecords, NotesToFinancialRecords), ServerError>;

    fn from_file<P>(
        &self,
        balances_csv: P,
        transactions_csv: P,
    ) -> Result<(FinancialRecords, NotesToFinancialRecords), ServerError>
    where
        P: AsRef<std::path::Path>;
}

pub(crate) struct ProcessUsecaseImpl<
    H,
    R1 = RecordsRepositoryImpl<H>, // Default.
> where
    H: Handlers,
    R1: RecordsRepository<H>,
{
    records_repository: R1,
    _phantom: std::marker::PhantomData<H>,
}

impl<H, R1> ProcessUsecase for ProcessUsecaseImpl<H, R1>
where
    H: Handlers,
    R1: RecordsRepository<H>,
{
    fn from_string(
        &self,
        transactions_csv: &str,
        balances_csv: &str,
    ) -> Result<(FinancialRecords, NotesToFinancialRecords), ServerError> {
        let specs = self
            .records_repository
            .from_string(transactions_csv, balances_csv)?;
        let decorated_specs = DecoratorProcessor::new(specs).process()?;
        let financial_records = SpecProcessor::new(decorated_specs).process()?;
        let notes_to_financial_records = AnnotationProcessor::new(&financial_records).process()?;
        Ok((financial_records, notes_to_financial_records))
    }

    fn from_file<P>(
        &self,
        transactions_csv: P,
        balances_csv: P,
    ) -> Result<(FinancialRecords, NotesToFinancialRecords), ServerError>
    where
        P: AsRef<std::path::Path>,
    {
        let specs = self
            .records_repository
            .from_file(transactions_csv, balances_csv)?;
        let decorated_specs = DecoratorProcessor::new(specs).process()?;
        let financial_records = SpecProcessor::new(decorated_specs).process()?;
        let notes_to_financial_records = AnnotationProcessor::new(&financial_records).process()?;
        Ok((financial_records, notes_to_financial_records))
    }
}

impl<H: Handlers> ProcessUsecaseImpl<H> {
    pub(crate) fn new() -> Self {
        ProcessUsecaseImpl {
            records_repository: RecordsRepositoryImpl::new(),
            _phantom: std::marker::PhantomData,
        }
    }
}
