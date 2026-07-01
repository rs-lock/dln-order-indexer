use chrono::Utc;
use clickhouse::Row;
use serde::{Deserialize, Serialize};

use crate::domain::order::{OrderCreated, OrderFulfilled};

#[derive(Row, Serialize, Deserialize)]
pub struct OrderCreatedRow {
    pub signature: String,
    pub order_id: String,
    pub give_amount: u128,
    pub give_token: String,
    pub price_usd: Option<f64>,
    pub decimals: Option<u8>,
    #[serde(with = "clickhouse::serde::chrono::datetime64::secs")]
    pub blocktime: chrono::DateTime<Utc>,
}

#[derive(Row, Serialize, Deserialize)]
pub struct OrderFulfilledRow {
    pub signature: String,
    pub order_id: String,
    pub taker: String,
    #[serde(with = "clickhouse::serde::chrono::datetime64::secs")]
    pub blocktime: chrono::DateTime<Utc>,
}

impl OrderCreatedRow {
    pub fn from_parts(order_event: &OrderCreated, signature: &str, blocktime: i64) -> Self {
        Self {
            signature: signature.to_string(),
            order_id: bs58::encode(order_event.order_id).into_string(),
            give_amount: order_event.give_amount,
            give_token: order_event.give_token.to_string(),
            price_usd: order_event.price_usd,
            decimals: order_event.decimals,
            blocktime: chrono::DateTime::from_timestamp_secs(blocktime)
                .expect("block_time validated upstream"),
        }
    }
}
impl OrderFulfilledRow {
    pub fn from_parts(order_event: &OrderFulfilled, signature: &str, blocktime: i64) -> Self {
        Self {
            signature: signature.to_string(),
            order_id: bs58::encode(order_event.order_id).into_string(),
            taker: order_event.taker.to_string(),
            blocktime: chrono::DateTime::from_timestamp_secs(blocktime)
                .expect("block_time validated upstream"),
        }
    }
}

#[derive(Row, Serialize, Deserialize)]
pub struct CursorRow {
    pub program: String,
    pub signature: String,
    #[serde(with = "clickhouse::serde::chrono::datetime64::secs")]
    pub timestamp: chrono::DateTime<Utc>,
}
