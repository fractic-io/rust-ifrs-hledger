use crate::entities::{CloseLogic, CommandLogic};

#[derive(Debug, serde_derive::Deserialize)]
pub enum CloseLogicModel {
    Retain,
}

#[derive(Debug, serde_derive::Deserialize)]
pub enum CommandLogicModel<O> {
    Close(CloseLogicModel),
    Correction(O),
}

impl Into<CloseLogic> for CloseLogicModel {
    fn into(self) -> CloseLogic {
        match self {
            CloseLogicModel::Retain => CloseLogic::Retain,
        }
    }
}

impl<O> Into<CommandLogic<O>> for CommandLogicModel<O> {
    fn into(self) -> CommandLogic<O> {
        match self {
            CommandLogicModel::Close(logic) => CommandLogic::Close(logic.into()),
            CommandLogicModel::Correction(logic) => CommandLogic::Correction(logic),
        }
    }
}
