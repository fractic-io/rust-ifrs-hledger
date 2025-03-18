use fractic_server_error::ServerError;

use crate::entities::DecoratedTransactionSpec;

use super::handlers::Handlers;

pub trait DecoratorLogic<H: Handlers> {
    fn apply(
        &self,
        tx: DecoratedTransactionSpec<H>,
    ) -> Result<DecoratedTransactionSpec<H>, ServerError>;
}
