use solana_sdk::pubkey::Pubkey;

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
    // TODO: order_id + give/take amounts, mints, timestamp
}

#[derive(Debug)]
pub struct OrderFulfilled {
    pub order_id: [u8; 32],
    pub taker: Pubkey,
    // TODO: order_id + taker, timestamp
}
