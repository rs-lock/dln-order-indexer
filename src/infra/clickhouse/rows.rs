use chrono::Utc;
use clickhouse::Row;
use serde::{Deserialize, Serialize};

#[derive(Row, Serialize, Deserialize)]
pub struct OrderCreatedRow {
    pub signature: String,
    pub order_id: String,
    pub give_amount: u128,
    pub give_token: String,
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
