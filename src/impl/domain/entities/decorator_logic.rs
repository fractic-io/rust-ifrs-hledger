use async_trait::async_trait;
use fractic_server_error::ServerError;

use crate::entities::DecoratedTransactionSpec;

use super::handlers::Handlers;

#[async_trait]
pub trait DecoratorLogic<H: Handlers>: std::fmt::Debug + Send + Sync {
    async fn apply(
        &self,
        tx: DecoratedTransactionSpec<H>,
    ) -> Result<DecoratedTransactionSpec<H>, ServerError>;
}

// --

#[derive(Debug)]
pub struct NoOpDecorator;

impl NoOpDecorator {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl<H: Handlers> DecoratorLogic<H> for NoOpDecorator {
    async fn apply(
        &self,
        tx: DecoratedTransactionSpec<H>,
    ) -> Result<DecoratedTransactionSpec<H>, ServerError> {
        Ok(tx)
    }
}
