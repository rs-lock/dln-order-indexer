use async_trait::async_trait;
use tokio::sync::{broadcast, mpsc::channel};
use tokio_util::sync::CancellationToken;

use crate::{
    application::worker::Worker,
    domain::{order::OrderEvent, transaction::Transaction},
};

struct Indexer {
    workers: Vec<Worker>,
}

impl Indexer {
    pub async fn run_indexer(&self, mut shutdown: broadcast::Receiver<()>) -> anyhow::Result<()> {
        let mut cursor_tx = "";

        let (tx, rx) = channel::<Transaction>(100);
        let cancel_token = CancellationToken::new();
        // TODO: drive the worker pool, paginate via cursor, exit on shutdown
        loop {
            tokio::select! {
                _ = shutdown.recv() => {
                    tracing::info!("shutdown signal received");
                    break;
                }
                _ = self.workers.last().unwrap().run_rpc(cancel_token.clone()) => {}
            }
        }
        Ok(())
    }
}

#[async_trait]
pub trait Rpc {
    async fn get_signatures(
        &self,
        program: String,
        cursor_tx: &str,
        limit: u32,
    ) -> anyhow::Result<Vec<String>>;
    async fn get_transaction_batch(&self, sig: &str) -> anyhow::Result<Vec<String>>;
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
