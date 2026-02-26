use chrono::NaiveDate;

use super::handlers::Handlers;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct CommandSpecId(pub(crate) u64);

#[derive(Debug, Clone, serde_derive::Deserialize)]
pub enum CloseLogic {
    Retain,
}

#[derive(Debug, serde_derive::Deserialize)]
pub enum CommandLogic<F> {
    Close(CloseLogic),
    Correction(F),
}

#[derive(Debug)]
pub(crate) struct Command<H: Handlers> {
    pub id: CommandSpecId,
    pub date: NaiveDate,
    pub exec: CommandLogic<H::F>,
    pub arguments: Vec<String>,
    pub description: Option<String>,
    pub amount: Option<f64>,
    pub commodity: Option<H::M>,
}

// --

impl std::fmt::Display for CommandSpecId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}
