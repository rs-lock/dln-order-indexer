use tokio::sync::{broadcast, mpsc::channel};
use tokio_util::sync::CancellationToken;

use crate::{application::worker::Worker, domain::transaction::Transaction};

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
