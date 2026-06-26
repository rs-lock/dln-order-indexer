use async_trait::async_trait;
use clickhouse::Client;

use crate::{
    application::ports::{CursorRepo, OrdersRepo},
    domain::order::OrderEvent,
    infra::clickhouse::models::OrderRow,
};

struct ClickhouseRepo {
    client: Client,
}

#[async_trait]
impl OrdersRepo for ClickhouseRepo {
    async fn insert_events(&self, events: Vec<OrderEvent>) -> anyhow::Result<()> {
        let mut insert = self.client.insert::<OrderRow>("orders").await?;

        for order in events {
            insert.write(&order.into()).await?;
        }

        insert.end().await.map_err(Into::into)
    }
}

impl From<OrderEvent> for OrderRow {
    fn from(_event: OrderEvent) -> Self {
        todo!("map domain OrderEvent -> OrderRow")
    }
}

#[async_trait]
impl CursorRepo for ClickhouseRepo {
    async fn load_cursor(&self) -> anyhow::Result<Option<String>> {
        todo!("load last processed signature from ClickHouse")
    }

    async fn save_cursor(&self, _cursor: &str) -> anyhow::Result<()> {
        todo!("persist last processed signature")
    }
}
