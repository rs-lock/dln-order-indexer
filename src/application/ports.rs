use crate::domain::{order::EnrichedEvent, transaction::Transaction};
use async_trait::async_trait;

#[derive(thiserror::Error, Debug)]
pub enum RpcError {
    #[error("circuit breaker open")]
    CircuitOpen,
    #[error(transparent)]
    Transport(#[from] anyhow::Error),
    #[error("invalid cursor signature: {0}")]
    InvalidCursor(String),
    #[error("invalid program id: {0}")]
    InvalidProgramId(String),
}

#[async_trait]
pub trait Rpc {
    async fn get_signatures(
        &self,
        program: &str,
        cursor_tx: Option<&str>,
        limit: Option<usize>,
    ) -> Result<Vec<String>, RpcError>;

    async fn get_transaction(&self, sigs: &str) -> Result<Option<Transaction>, RpcError>;
}

#[async_trait]
pub trait OrdersRepo {
    async fn insert_events(&self, events: &[EnrichedEvent]) -> anyhow::Result<()>;
}

#[async_trait]
pub trait CursorRepo {
    async fn load_cursor(&self) -> anyhow::Result<Option<String>>;
    async fn save_cursor(&self, cursor: &str) -> anyhow::Result<()>;
}
