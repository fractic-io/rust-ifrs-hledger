use crate::entities::{CloseLogic, OperationLogic};

#[derive(Debug, serde_derive::Deserialize)]
pub enum CloseLogicModel {
    Retain,
}

#[derive(Debug, serde_derive::Deserialize)]
pub enum OperationLogicModel<O> {
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

impl<O> Into<OperationLogic<O>> for OperationLogicModel<O> {
    fn into(self) -> OperationLogic<O> {
        match self {
            OperationLogicModel::Close(logic) => OperationLogic::Close(logic.into()),
            OperationLogicModel::Correction(logic) => OperationLogic::Correction(logic),
        }
    }
}
