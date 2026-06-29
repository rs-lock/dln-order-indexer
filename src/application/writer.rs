use std::{collections::HashMap, sync::Arc, time::Duration};

use tokio::{sync::mpsc::Receiver, time::sleep};
use tokio_util::sync::CancellationToken;

use crate::{
    application::ports::{CursorRepo, OrdersRepo, WriteError, WriterMsg},
    domain::order::EnrichedEvent,
};

enum FlushOutcome {
    Flushed,
    Cancelled,
}

pub struct Writer {
    capacity: usize,
    flush_interval: Duration,
    receiver: Receiver<WriterMsg>,
    repository: Arc<dyn OrdersRepo>,
    cursor_repo: Arc<dyn CursorRepo>,
}

impl Writer {
    pub fn new(
        capacity: usize,
        flush_interval: Duration,
        receiver: Receiver<WriterMsg>,
        repository: Arc<dyn OrdersRepo>,
        cursor_repo: Arc<dyn CursorRepo>,
    ) -> Self {
        Self {
            capacity,
            flush_interval,
            receiver,
            repository,
            cursor_repo,
        }
    }
}

impl Writer {
    pub async fn run(&mut self, cancel: CancellationToken) {
        let mut pending: HashMap<String, String> = HashMap::new();
        let mut batch: Vec<EnrichedEvent> = Vec::with_capacity(self.capacity);

        let mut timer = tokio::time::interval(self.flush_interval);
        loop {
            tokio::select! {
                biased;
                maybe_tx = self.receiver.recv() => {
                    let Some(msg) = maybe_tx else {
                        if let Err(e) = self.flush_and_clear(&mut batch, &mut pending, &cancel).await {
                            tracing::error!("flush error {e}");
                        }
                        break;
                    };

                    match msg {
                        WriterMsg::Data(event) => {
                            if !event.order_events.is_empty() {
                                batch.push(event);
                            }

                            if batch.len() >= self.capacity {
                                if let Err(e) = self.flush_and_clear(&mut batch, &mut pending, &cancel).await {
                                    tracing::error!("flush error {e}");
                                }
                            };
                        },
                        WriterMsg::Checkpoint { program, signature } => {
                            pending.insert(program.clone(), signature.clone());

                        },
                    };



                },

                _ = timer.tick() => {
                    tracing::debug!("interval tick, flushing");
                    if let Err(e) = self.flush_and_clear(&mut batch, &mut pending, &cancel).await {
                        tracing::error!("flush error {e}");
                    };
                }

            }
        }
    }

    async fn flush_and_clear(
        &self,
        batch: &mut Vec<EnrichedEvent>,
        cursor_map: &mut HashMap<String, String>,
        cancel: &CancellationToken,
    ) -> anyhow::Result<()> {
        let res = self
            .with_backoff(|| self.flush_batch(batch), cancel)
            .await?;
        match res {
            FlushOutcome::Cancelled => {}
            FlushOutcome::Flushed => {
                self.save_cursors(cursor_map).await?;
                cursor_map.clear();
                batch.clear();
            }
        };
        Ok(())
    }

    async fn save_cursors(&self, cursor_map: &HashMap<String, String>) -> anyhow::Result<()> {
        for (prog, sig) in cursor_map {
            self.cursor_repo.save_cursor(sig, prog).await?;
        }
        Ok(())
    }

    async fn flush_batch(&self, batch: &Vec<EnrichedEvent>) -> Result<(), WriteError> {
        if !batch.is_empty() {
            self.repository.insert_events(batch).await?;
        }
        Ok(())
    }

    async fn with_backoff<F, Fut>(
        &self,
        f: F,
        cancel: &CancellationToken,
    ) -> anyhow::Result<FlushOutcome>
    where
        F: Fn() -> Fut,
        Fut: Future<Output = Result<(), WriteError>>,
    {
        let mut delay = Duration::from_millis(90);

        loop {
            let res = f().await;
            match res {
                Ok(_) => {
                    return Ok(FlushOutcome::Flushed);
                }

                Err(WriteError::Permanent(e)) => return Err(e),

                Err(WriteError::Transient(e)) => {
                    //retry only for ch lag
                    tracing::warn!(error = ?e, "transient flush error, retrying");
                    tokio::select! {
                        _ = sleep(delay) => {
                            delay = (delay * 2).min(Duration::from_secs(30));
                        }
                        _ = cancel.cancelled() => {
                            return Ok(FlushOutcome::Cancelled);
                        }
                    }
                }
            };
        }
    }
}
