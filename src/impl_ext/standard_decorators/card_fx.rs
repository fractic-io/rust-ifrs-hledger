use fractic_server_error::ServerError;

use crate::entities::{DecoratedTransactionSpec, DecoratorLogic, Handlers};

pub struct StandardDecoratorCardFx;

impl<'a, H: Handlers> DecoratorLogic<'a, H> for StandardDecoratorCardFx {
    fn apply(
        &self,
        tx: DecoratedTransactionSpec<H>,
    ) -> Result<DecoratedTransactionSpec<H>, ServerError> {
        todo!()
    }
}
