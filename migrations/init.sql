CREATE DATABASE IF NOT EXISTS orders;

 CREATE TABLE IF NOT EXISTS orders.created (
    signature String,
    order_id String,
    give_amount UInt128,
    give_token String,
    blocktime DateTime64(0),
    date Date DEFAULT toDate(blocktime)
) ENGINE = ReplacingMergeTree()
PARTITION BY toYYYYMM(date)
ORDER BY order_id;

 CREATE TABLE IF NOT EXISTS orders.fulfilled (
    signature String,
    order_id String,
    taker String,
    blocktime DateTime64(0),
    date Date DEFAULT toDate(blocktime)
) ENGINE = ReplacingMergeTree()
PARTITION BY toYYYYMM(date)
ORDER BY order_id;


 CREATE TABLE IF NOT EXISTS orders.cursor (
    program String,
    signature String,
    timestamp DateTime64(0)
) ENGINE = ReplacingMergeTree(timestamp)
ORDER BY program;