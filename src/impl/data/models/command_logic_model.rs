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

impl<O> From<CommandLogicModel<O>> for CommandLogic<O> {
    fn from(val: CommandLogicModel<O>) -> Self {
        match val {
            CommandLogicModel::Close(logic) => CommandLogic::Close(match logic {
                CloseLogicModel::Retain => CloseLogic::Retain,
            }),
            CommandLogicModel::Correction(logic) => CommandLogic::Correction(logic),
        }
    }
}
