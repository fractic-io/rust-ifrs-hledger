use chrono::NaiveDate;

use super::handlers::Handlers;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct OperationSpecId(pub(crate) u64);

#[derive(Debug, Clone, serde_derive::Deserialize)]
pub enum CloseLogic {
    Retain,
}

#[derive(Debug, serde_derive::Deserialize)]
pub enum OperationLogic<O> {
    Close(CloseLogic),
    Correction(O),
}

#[derive(Debug)]
pub(crate) struct OperationSpec<H: Handlers> {
    pub id: OperationSpecId,
    pub date: NaiveDate,
    pub operation_logic: OperationLogic<H::O>,
    pub arguments: Vec<String>,
    pub description: String,
    pub amount: Option<f64>,
    pub commodity: Option<H::M>,
}

// --

impl std::fmt::Display for OperationSpecId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}
