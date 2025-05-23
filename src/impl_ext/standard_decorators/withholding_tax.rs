use async_trait::async_trait;
use fractic_server_error::ServerError;

use crate::entities::{Annotation, DecoratedTransactionSpec, DecoratorLogic, Handlers};

#[derive(Debug)]
enum LogicType {
    Unrecoverable(i32),
}

#[derive(Debug)]
pub struct StandardDecoratorWithholdingTax {
    logic: LogicType,
}

impl StandardDecoratorWithholdingTax {
    pub fn unrecoverable(w: i32) -> Result<Self, ServerError> {
        Ok(Self {
            logic: LogicType::Unrecoverable(w),
        })
    }

    // --

    fn apply_unrecoverable<H: Handlers>(
        &self,
        mut tx: DecoratedTransactionSpec<H>,
        w: i32,
    ) -> Result<DecoratedTransactionSpec<H>, ServerError> {
        // Tag this transaction, since the accounting logic deserves a note in
        // the financial records.
        tx.annotations
            .push(Annotation::WithholdingTaxUnrecoverable(w));
        Ok(tx)
    }
}

#[async_trait]
impl<H: Handlers> DecoratorLogic<H> for StandardDecoratorWithholdingTax {
    async fn apply(
        &self,
        tx: DecoratedTransactionSpec<H>,
    ) -> Result<DecoratedTransactionSpec<H>, ServerError> {
        match &self.logic {
            LogicType::Unrecoverable(w) => self.apply_unrecoverable(tx, *w),
        }
    }
}
