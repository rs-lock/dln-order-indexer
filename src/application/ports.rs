use crate::domain::order::OrderEvent;
use async_trait::async_trait;

#[derive(thiserror::Error, Debug)]
pub enum RpcError {
    #[error("circuit breaker open")]
    CircuitOpen,
    #[error(transparent)]
    Transport(#[from] anyhow::Error),
}

#[async_trait]
pub trait Rpc {
    async fn get_signatures(
        &self,
        program: String,
        cursor_tx: &str,
        limit: u32,
    ) -> Result<Vec<String>, RpcError>;

    async fn get_transaction_batch(&self, sig: &str) -> Result<Vec<String>, RpcError>;
}

#[async_trait]
pub trait OrdersRepo {
    async fn insert_events(&self, events: Vec<OrderEvent>) -> anyhow::Result<()>;
}

#[async_trait]
pub trait CursorRepo {
    async fn load_cursor(&self) -> anyhow::Result<Option<String>>;
    async fn save_cursor(&self, cursor: &str) -> anyhow::Result<()>;
}
