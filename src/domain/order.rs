use solana_sdk::{pubkey::Pubkey, signature::Signature};

pub struct EnrichedEvent {
    pub signature: Signature,
    pub order_events: Vec<OrderEvent>,
    pub blocktime: i64,
}

#[derive(Debug)]
pub enum OrderEvent {
    Created(OrderCreated),
    Fulfilled(OrderFulfilled),
}

#[derive(Debug)]
pub struct OrderCreated {
    pub order_id: [u8; 32],
    pub give_amount: u128,
    pub give_token: Pubkey,
    pub price_usd: Option<f64>,
    pub decimals: Option<u8>,
}

#[derive(Debug)]
pub struct OrderFulfilled {
    pub order_id: [u8; 32],
    pub taker: Pubkey,
}
