use std::{sync::Arc, time::Duration};

use tokio::{sync::mpsc, task::JoinSet};
use tokio_util::sync::CancellationToken;

use crate::application::{
    ports::{CursorRepo, OrdersRepo, Rpc},
    worker::Worker,
    writer::Writer,
};

/// Orchestrates the indexing pipeline: spawns one worker per program (fan-out reads)
/// feeding a single Writer (fan-in batched writes), and coordinates graceful shutdown
pub struct Indexer {
    rpc: Arc<dyn Rpc>,
    orders_repo: Arc<dyn OrdersRepo>,
    cursor_repo: Arc<dyn CursorRepo>,
    programs: Vec<String>,
    channel_capacity: usize,
    page_size: usize,
    batch_size: usize,
    flush_interval: Duration,
}

impl Indexer {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        rpc: Arc<dyn Rpc>,
        orders_repo: Arc<dyn OrdersRepo>,
        cursor_repo: Arc<dyn CursorRepo>,
        programs: Vec<String>,
        channel_capacity: usize,
        page_size: usize,
        batch_size: usize,
        flush_interval: Duration,
    ) -> Self {
        Self {
            rpc,
            orders_repo,
            cursor_repo,
            programs,
            channel_capacity,
            page_size,
            batch_size,
            flush_interval,
        }
    }

    pub async fn run(self, cancel: CancellationToken) -> anyhow::Result<()> {
        let (tx, rx) = mpsc::channel(self.channel_capacity);

        // --- Writer (single consumer, fan-in): drains channel, batches, flushes to storage ---
        let mut writer = Writer::new(
            self.batch_size,
            self.flush_interval,
            rx,
            self.orders_repo,
            self.cursor_repo.clone(),
        );
        let writer_token = cancel.clone();
        let writer_handle = tokio::spawn(async move { writer.run(writer_token).await });

        // --- Workers (producers, fan-out): one per program ---
        let mut workers: JoinSet<anyhow::Result<()>> = JoinSet::new();
        for program in self.programs {
            let worker = Worker::new(
                program,
                self.rpc.clone(),
                self.cursor_repo.clone(),
                tx.clone(),
                self.page_size,
            );
            let worker_token = cancel.clone();
            workers.spawn(async move { worker.run_rpc(worker_token).await });
        }
        drop(tx);

        // --- Shutdown coordination + fail-fast ---
        let mut had_failure = false;
        while let Some(joined) = workers.join_next().await {
            match joined {
                Ok(Ok(())) => {}
                Ok(Err(e)) => {
                    tracing::error!(error = %e, "worker failed, shutting down");
                    had_failure = true;
                    cancel.cancel();
                }
                Err(e) => {
                    tracing::error!(error = %e, "worker task panicked, shutting down");
                    had_failure = true;
                    cancel.cancel();
                }
            }
        }

        if let Err(e) = writer_handle.await {
            tracing::error!(error = %e, "writer task panicked");
            had_failure = true;
        }

        if had_failure {
            anyhow::bail!("indexer exited due to a worker/writer failure");
        }
        Ok(())
    }
}
