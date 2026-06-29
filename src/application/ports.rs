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

impl RpcError {
    pub fn is_transient(&self) -> bool {
        matches!(self, RpcError::Transport(_) | RpcError::CircuitOpen)
    }
}

pub enum WriteError {
    Transient(anyhow::Error),
    Permanent(anyhow::Error),
}

pub enum WriterMsg {
    Data(EnrichedEvent),
    Checkpoint { program: String, signature: String },
}

#[async_trait]
pub trait Rpc: Send + Sync {
    async fn get_signatures(
        &self,
        program: &str,
        cursor_tx: Option<&str>,
        limit: Option<usize>,
    ) -> Result<Vec<String>, RpcError>;

    async fn get_transaction(&self, sigs: &str) -> Result<Option<Transaction>, RpcError>;
}

#[async_trait]
pub trait OrdersRepo: Send + Sync {
    async fn insert_events(&self, events: &[EnrichedEvent]) -> Result<(), WriteError>;
}

#[async_trait]
pub trait CursorRepo: Send + Sync {
    async fn load_cursor(&self, program: &str) -> anyhow::Result<Option<String>>;
    async fn save_cursor(&self, cursor: &str, program: &str) -> anyhow::Result<()>;
}
