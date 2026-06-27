use borsh::BorshDeserialize;

pub enum Events {
    CreatedOrderId(CreatedOrderIdEvent),
    CreatedOrder(CreatedOrderEvent),
    FulfilledOrder(FulfilledEvent),
}

#[derive(BorshDeserialize)]
pub struct FulfilledEvent {
    pub order_id: [u8; 32],
    pub taker: [u8; 32],
}

#[derive(BorshDeserialize)]
pub struct CreatedOrderIdEvent {
    pub order_id: [u8; 32],
}

#[derive(BorshDeserialize)]
pub struct CreatedOrderEvent {
    pub order: Order,
    pub fix_fee: u64,
    pub percent_fee: u64,
}

#[derive(BorshDeserialize)]
pub struct Order {
    pub make_order_nonce: u64,
    pub maker_src: Vec<u8>,
    pub give: Offer,
    pub take: Offer,
    pub receiver_dst: Vec<u8>,
    pub give_patch_authority_src: Vec<u8>,
    pub order_authority_address_dst: Vec<u8>,
    pub allowed_taker_dst: Option<Vec<u8>>,
    pub allowed_cancel_beneficiary_src: Option<Vec<u8>>,
    pub external_call: Option<ExternalCallParams>,
}

#[derive(BorshDeserialize)]
pub struct Offer {
    pub chain_id: [u8; 32],
    pub mint: Vec<u8>,
    pub amount: [u8; 32],
}

#[derive(BorshDeserialize)]
struct ExternalCallParams {
    pub external_call_shortcut: [u8; 32],
}
