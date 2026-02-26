use async_trait::async_trait;
use fractic_server_error::ServerError;

use crate::{
    data::repositories::records_repository_impl::RecordsRepositoryImpl,
    domain::{
        logic::{
            annotation_processor::AnnotationProcessor, command_processor::CommandProcessor,
            decorator_processor::DecoratorProcessor, spec_processor::SpecProcessor,
        },
        repositories::records_repository::RecordsRepository,
    },
    entities::{FinancialRecords, Handlers, NotesToFinancialRecords},
};

#[async_trait]
pub trait ProcessUsecase: Send + Sync {
    async fn from_string(
        &self,
        balances_csv: &str,
        transactions_csv: &str,
    ) -> Result<(FinancialRecords, NotesToFinancialRecords), ServerError>;

    async fn from_file<P>(
        &self,
        balances_csv: P,
        transactions_csv: P,
    ) -> Result<(FinancialRecords, NotesToFinancialRecords), ServerError>
    where
        P: AsRef<std::path::Path> + Send;
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

#[async_trait]
impl<H, R1> ProcessUsecase for ProcessUsecaseImpl<H, R1>
where
    H: Handlers,
    R1: RecordsRepository<H>,
{
    async fn from_string(
        &self,
        transactions_csv: &str,
        balances_csv: &str,
    ) -> Result<(FinancialRecords, NotesToFinancialRecords), ServerError> {
        let input = self
            .records_repository
            .from_string(transactions_csv, balances_csv)?;
        let intermediate_1 = DecoratorProcessor::new(input).process().await?;
        let intermediate_2 = SpecProcessor::new(intermediate_1).process()?;
        let output = CommandProcessor::new(intermediate_2).process()?;
        let output_notes = AnnotationProcessor::new(&output).process()?;
        Ok((output, output_notes))
    }

    async fn from_file<P>(
        &self,
        transactions_csv: P,
        balances_csv: P,
    ) -> Result<(FinancialRecords, NotesToFinancialRecords), ServerError>
    where
        P: AsRef<std::path::Path> + Send,
    {
        let input = self
            .records_repository
            .from_file(transactions_csv, balances_csv)
            .await?;
        let intermediate_1 = DecoratorProcessor::new(input).process().await?;
        let intermediate_2 = SpecProcessor::new(intermediate_1).process()?;
        let output = CommandProcessor::new(intermediate_2).process()?;
        let output_notes = AnnotationProcessor::new(&output).process()?;
        Ok((output, output_notes))
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
