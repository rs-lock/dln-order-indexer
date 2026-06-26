use clickhouse::Row;
use serde::{Deserialize, Serialize};

#[derive(Row, Serialize, Deserialize)]
pub struct OrderRow {
    pub id: u64,
}
