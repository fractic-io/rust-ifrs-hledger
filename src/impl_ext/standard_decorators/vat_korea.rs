use fractic_server_error::ServerError;

use crate::entities::{DecoratedTransactionSpec, DecoratorLogic, Handlers};

pub struct StandardDecoratorVatKorea;

impl<'a, H: Handlers> DecoratorLogic<'a, H> for StandardDecoratorVatKorea {
    fn apply(
        &self,
        tx: DecoratedTransactionSpec<H>,
    ) -> Result<DecoratedTransactionSpec<H>, ServerError> {
        todo!()
    }
}
