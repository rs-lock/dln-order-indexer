use std::{sync::Arc, time::Duration};

use futures::{StreamExt, stream};
use tokio::sync::mpsc::Sender;
use tokio_util::sync::CancellationToken;

use crate::{
    application::ports::{CursorRepo, PriceProvider, Rpc, WriterMsg},
    domain::{
        order::{OrderCreated, OrderEvent},
        parser::parser::parse_transaction,
    },
};

enum FetchOutcome {
    Continue(Option<String>),
    Done,
}

pub struct Worker {
    program_id: String,
    rpc: Arc<dyn Rpc>,
    cursor_repo: Arc<dyn CursorRepo>,
    price_provider: Arc<dyn PriceProvider>,
    sender: Sender<WriterMsg>,
    page_size: usize,
}

impl Worker {
    pub fn new(
        program_id: String,
        rpc: Arc<dyn Rpc>,
        cursor_repo: Arc<dyn CursorRepo>,
        price_provider: Arc<dyn PriceProvider>,
        sender: Sender<WriterMsg>,
        page_size: usize,
    ) -> Self {
        Self {
            program_id,
            rpc,
            cursor_repo,
            price_provider,
            sender,
            page_size,
        }
    }

    pub async fn run_rpc(&self, cancel_token: CancellationToken) -> anyhow::Result<()> {
        let mut cursor = self.cursor_repo.load_cursor(&self.program_id).await?;
        loop {
            tokio::select! {
                _ = cancel_token.cancelled() => {
                    tracing::info!(program_id = %self.program_id, "worker cancelled");
                    break;
                }
                res = self.fetch(cursor.as_deref()) => {
                    match res? {
                        FetchOutcome::Done => {
                            tracing::info!(program_id = %self.program_id, "backfill done");
                            break;
                        }
                        FetchOutcome::Continue(next) => cursor = next
                    }
                }

            }
        }

        Ok(())
    }

    async fn fetch(&self, cursor: Option<&str>) -> anyhow::Result<FetchOutcome> {
        let sigs_res = self
            .rpc
            .get_signatures(&self.program_id, cursor, Some(self.page_size))
            .await;

        let signatures = match sigs_res {
            Ok(s) => s,
            Err(e) if e.is_transient() => {
                tracing::warn!(%e, "get_signatures transient, retry page");
                return Ok(FetchOutcome::Continue(cursor.map(String::from)));
            }
            Err(e) => return Err(e.into()),
        };

        if signatures.is_empty() {
            return Ok(FetchOutcome::Done);
        }

        let last_sig = signatures.last().cloned();
        let mut stream = stream::iter(signatures)
            .map(|sig| {
                let rpc = self.rpc.clone();
                async move {
                    const MAX_ATTEMPTS: u32 = 8;
                    let mut delay = Duration::from_millis(100);
                    let mut attempts = 0u32;
                    let result = loop {
                        match rpc.get_transaction(&sig).await {
                            Ok(v) => break Ok(v),
                            Err(e) if e.is_transient() => {
                                attempts += 1;
                                if attempts >= MAX_ATTEMPTS {
                                    break Err(e);
                                }
                                tokio::time::sleep(delay).await;
                                delay = (delay * 2).min(Duration::from_secs(30));
                            }
                            Err(e) => break Err(e),
                        }
                    };
                    (sig, result)
                }
            })
            .buffer_unordered(20);

        while let Some((sig, res)) = stream.next().await {
            match res {
                Ok(Some(tx)) => {
                    let mut event = parse_transaction(&tx);
                    for order in &mut event.order_events {
                        if let OrderEvent::Created(created) = order {
                            self.try_fill_with_price(created).await;
                        }
                    }
                    self.sender
                        .send(WriterMsg::Data(event))
                        .await
                        .map_err(|_| anyhow::anyhow!("writer channel closed"))?;
                }
                Ok(None) => {
                    tracing::warn!(%sig, "transaction not found, cursor advances past it");
                }
                Err(e) if e.is_transient() => {
                    tracing::warn!(%sig, %e, "get_transaction exhausted retries, skipping signature");
                }
                Err(e) => {
                    return Err(e.into());
                }
            }
        }

        if let Some(last) = &last_sig {
            self.sender
                .send(WriterMsg::Checkpoint {
                    program: self.program_id.clone(),
                    signature: last.clone(),
                })
                .await
                .map_err(|_| anyhow::anyhow!("writer channel closed"))?;
        };

        Ok(FetchOutcome::Continue(last_sig))
    }

    async fn try_fill_with_price(&self, order: &mut OrderCreated) {
        match self
            .price_provider
            .get_price(&order.give_token.to_string())
            .await
        {
            Ok(Some(info)) => {
                order.price_usd = Some(info.usd_price);
                order.decimals = Some(info.decimals);
            }
            Ok(None) => tracing::warn!(mint = %order.give_token, "unknown token"),
            Err(e) => tracing::warn!(?e, "price fetch failed"),
        }
    }
}
