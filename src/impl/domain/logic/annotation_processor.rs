use fractic_server_error::ServerError;

use crate::entities::{FinancialRecords, NotesToFinancialRecords};

pub(crate) struct AnnotationProcessor<'a> {
    records: &'a FinancialRecords,
}

impl<'a> AnnotationProcessor<'a> {
    pub(crate) fn new(records: &'a FinancialRecords) -> Self {
        Self { records }
    }

    pub(crate) fn process(self) -> Result<NotesToFinancialRecords, ServerError> {
        unimplemented!()
    }
}
