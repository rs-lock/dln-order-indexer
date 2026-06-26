use std::sync::Arc;

use tokio::sync::mpsc::Receiver;

use crate::{application::ports::OrdersRepo, domain::transaction::Transaction};

struct Writer {
    receiver: Receiver<Transaction>,
    repository: Arc<dyn OrdersRepo>,
}

impl Writer {
    async fn run(&mut self) {
        while let Some(orders) = self.receiver.recv().await {
            self.repository.insert_events(vec![]).await;
        }
    }
}
