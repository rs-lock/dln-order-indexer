use std::{sync::Arc, time::Duration};

use futures::{StreamExt, stream};
use tokio::sync::mpsc::Sender;
use tokio_util::sync::CancellationToken;

use crate::{
    application::ports::{CursorRepo, Rpc, WriterMsg},
    domain::parser::parser::parse_transaction,
};

enum FetchOutcome {
    Continue(Option<String>),
    Done,
}

pub struct Worker {
    program_id: String,
    rpc: Arc<dyn Rpc>,
    cursor_repo: Arc<dyn CursorRepo>,
    sender: Sender<WriterMsg>,
    page_size: usize,
}

impl Worker {
    pub fn new(
        program_id: String,
        rpc: Arc<dyn Rpc>,
        cursor_repo: Arc<dyn CursorRepo>,
        sender: Sender<WriterMsg>,
        page_size: usize,
    ) -> Self {
        Self {
            program_id,
            rpc,
            cursor_repo,
            sender,
            page_size,
        }
    }

    // TODO: fetch signatures by cursor, fan out to RPC, send Transactions downstream
    pub async fn run_rpc(&self, cancel_token: CancellationToken) -> anyhow::Result<()> {
        let mut cursor = self.cursor_repo.load_cursor(&self.program_id).await?;
        loop {
            tokio::select! {
                _ = cancel_token.cancelled() => {
                    tracing::info!(program_id = %self.program_id, "worker cancelled");
                    break;
                }
                res = self.fetch(cursor.as_deref()) => {
                    match res? {                     // permanent Err пробрасывается → процесс выходит
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
                    let mut delay = Duration::from_millis(100);
                    loop {
                        match rpc.get_transaction(&sig).await {
                            Ok(v) => return Ok(v),
                            Err(e) if e.is_transient() => {
                                tokio::time::sleep(delay).await;
                                delay = (delay * 2).min(Duration::from_secs(30));
                            }
                            Err(e) => return Err(e),
                        }
                    }
                }
            })
            .buffer_unordered(20);

        while let Some(res) = stream.next().await {
            match res {
                Ok(Some(tx)) => {
                    let event = parse_transaction(&tx);
                    let _ = self.sender.send(WriterMsg::Data(event)).await;
                }
                Ok(None) => {}
                Err(e) if e.is_transient() => {
                    tracing::warn!(%e, "get_transaction transient");
                }
                Err(e) => {
                    return Err(e.into());
                }
            }
        }

        if let Some(last) = &last_sig {
            let _ = self
                .sender
                .send(WriterMsg::Checkpoint {
                    program: self.program_id.clone(),
                    signature: last.clone(),
                })
                .await;
        };

        Ok(FetchOutcome::Continue(last_sig))
    }
}
