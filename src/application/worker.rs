use std::sync::Arc;

use tokio::sync::mpsc::Sender;
use tokio_util::sync::CancellationToken;

use crate::{
    application::ports::{CursorRepo, Rpc},
    domain::transaction::Transaction,
};

pub struct Worker {
    program_id: String,
    rpc: Arc<dyn Rpc>,
    cursor_repo: Arc<dyn CursorRepo>,
    sender: Sender<Transaction>,
}

impl Worker {
    pub fn new(
        program_id: String,
        rpc: Arc<dyn Rpc>,
        cursor_repo: Arc<dyn CursorRepo>,
        sender: Sender<Transaction>,
    ) -> Self {
        Self {
            program_id,
            rpc,
            cursor_repo,
            sender,
        }
    }

    // TODO: fetch signatures by cursor, fan out to RPC, send Transactions downstream
    pub async fn run_rpc(&self, cancel_token: CancellationToken) {
        loop {
            tokio::select! {
                _ = cancel_token.cancelled() => {
                    tracing::info!(program_id = %self.program_id, "worker cancelled");
                    break;
                }
                _ = self.rpc.get_transaction_batch("") => {}
            }
        }
    }
}
