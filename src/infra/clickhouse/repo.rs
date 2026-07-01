use async_trait::async_trait;
use chrono::Utc;
use clickhouse::Client;

use crate::{
    application::ports::{CursorRepo, OrdersRepo, WriteError},
    domain::order::{EnrichedEvent, OrderEvent},
    infra::clickhouse::rows::{CursorRow, OrderCreatedRow, OrderFulfilledRow},
};

pub struct ClickhouseRepo {
    client: Client,
}

impl ClickhouseRepo {
    pub fn new(client: Client) -> Self {
        Self { client }
    }

    async fn write_events(&self, events: &[EnrichedEvent]) -> Result<(), clickhouse::error::Error> {
        let mut insert_fulfilled = self.client.insert::<OrderFulfilledRow>("fulfilled").await?;

        let mut insert_created = self.client.insert::<OrderCreatedRow>("created").await?;

        for e in events {
            let sig = e.signature.to_string();
            for order_e in &e.order_events {
                match order_e {
                    OrderEvent::Created(order_created) => {
                        insert_created
                            .write(&OrderCreatedRow::from_parts(
                                order_created,
                                &sig,
                                e.blocktime,
                            ))
                            .await?;
                    }

                    OrderEvent::Fulfilled(order_fulfilled) => {
                        insert_fulfilled
                            .write(&OrderFulfilledRow::from_parts(
                                order_fulfilled,
                                &sig,
                                e.blocktime,
                            ))
                            .await?;
                    }
                }
            }
        }
        insert_created.end().await?;
        insert_fulfilled.end().await?;
        Ok(())
    }
}

#[async_trait]
impl OrdersRepo for ClickhouseRepo {
    async fn insert_events(&self, events: &[EnrichedEvent]) -> Result<(), WriteError> {
        self.write_events(events).await.map_err(classify)
    }
}
#[async_trait]
impl CursorRepo for ClickhouseRepo {
    async fn load_cursor(&self, program: &str) -> anyhow::Result<Option<String>> {
        let query = "SELECT ?fields FROM orders.cursor
            WHERE program = ?
            ORDER BY timestamp DESC
            LIMIT 1";

        let row = self
            .client
            .query(query)
            .bind(program)
            .fetch_optional::<CursorRow>()
            .await?;
        Ok(row.map(|r| r.signature))
    }

    async fn save_cursor(&self, signature: &str, program: &str) -> anyhow::Result<()> {
        let mut insert = self.client.insert::<CursorRow>("cursor").await?;
        insert
            .write(&CursorRow {
                program: program.to_string(),
                signature: signature.to_string(),
                timestamp: Utc::now(),
            })
            .await?;
        insert.end().await?;
        Ok(())
    }
}

fn classify(e: clickhouse::error::Error) -> WriteError {
    use clickhouse::error::Error as E;
    let permanent = match &e {
        E::SchemaMismatch(_)
        | E::NotEnoughData
        | E::InvalidParams(_)
        | E::Unsupported(_)
        | E::Custom(_) => true,
        E::BadResponse(msg) => is_permanent_code(msg),
        _ => false,
    };
    if permanent {
        WriteError::Permanent(anyhow::anyhow!("{e}"))
    } else {
        WriteError::Transient(anyhow::anyhow!("{e}"))
    }
}

fn is_permanent_code(msg: &str) -> bool {
    // Permanent: 516 auth, 60 UNKNOWN_TABLE, 47 UNKNOWN_IDENTIFIER,
    //            53 TYPE_MISMATCH, 16 NO_SUCH_COLUMN, 62 SYNTAX_ERROR
    // НЕ permanent (transient): 252 TOO_MANY_PARTS, 241 MEMORY_LIMIT,
    //            242 TABLE_IS_READ_ONLY, 202 TOO_MANY_SIMULTANEOUS_QUERIES
    const PERMANENT: &[&str] = &[
        "Code: 516.",
        "Code: 60.",
        "Code: 47.",
        "Code: 53.",
        "Code: 16.",
        "Code: 62.",
    ];
    PERMANENT.iter().any(|c| msg.contains(c))
}
