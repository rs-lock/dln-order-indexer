use base64::Engine as _;
use borsh::BorshDeserialize;
use sha2::Digest;
use solana_sdk::pubkey::Pubkey;

use crate::domain::{
    order::{EnrichedEvent, OrderCreated, OrderEvent, OrderFulfilled},
    parser::events::{CreatedOrderEvent, CreatedOrderIdEvent, Events, FulfilledEvent},
    transaction::Transaction,
};

const PROGRAM_DATA_PREFIX: &str = "Program data: ";

pub fn parse_transaction(tx: &Transaction) -> EnrichedEvent {
    let raw_events: Vec<Events> = tx
        .logs
        .iter()
        .filter_map(|line| decode_program_data(line))
        .filter_map(|bytes| parse_event(&bytes))
        .collect();
    let events = map_events(&raw_events);
    EnrichedEvent {
        signature: tx.signature,
        order_events: events,
        blocktime: tx.blocktime,
    }
}

fn decode_program_data(line: &str) -> Option<Vec<u8>> {
    let stripped_b64 = line.strip_prefix(PROGRAM_DATA_PREFIX)?;
    base64::engine::general_purpose::STANDARD
        .decode(stripped_b64)
        .ok()
}

fn parse_event(bytes: &[u8]) -> Option<Events> {
    let (disc, rest) = bytes.split_at_checked(8)?;

    if disc == event_discriminator("Fulfilled") {
        let event = FulfilledEvent::try_from_slice(rest).ok()?;
        return Some(Events::FulfilledOrder(event));
    }

    if disc == event_discriminator("CreatedOrder") {
        let event = CreatedOrderEvent::try_from_slice(rest).ok()?;
        return Some(Events::CreatedOrder(event));
    }

    if disc == event_discriminator("CreatedOrderId") {
        let event = CreatedOrderIdEvent::try_from_slice(rest).ok()?;
        return Some(Events::CreatedOrderId(event));
    }

    None
}

fn map_events(events: &[Events]) -> Vec<OrderEvent> {
    let mut order_events = vec![];

    for idx in 0..events.len() {
        let current = &events[idx];
        match current {
            Events::CreatedOrderId(_) => {
                continue;
            }
            Events::CreatedOrder(order_event) => {
                let Some(Events::CreatedOrderId(order_id)) = events.get(idx + 1) else {
                    tracing::info!("CreatedOrder without following order_id");
                    continue;
                };

                let Some(order) = try_build_created(order_event, order_id.order_id) else {
                    tracing::info!("Skipped CreatedOrder: amount overflow or bad mint");
                    continue;
                };

                order_events.push(OrderEvent::Created(order));
            }
            Events::FulfilledOrder(fulfilled_event) => {
                order_events.push(OrderEvent::Fulfilled(OrderFulfilled {
                    order_id: fulfilled_event.order_id,
                    taker: Pubkey::new_from_array(fulfilled_event.taker),
                }));
            }
        }
    }
    order_events
}

fn event_discriminator(event_str: &str) -> [u8; 8] {
    let hash = sha2::Sha256::digest(format!("event:{event_str}").as_bytes());
    hash[..8]
        .try_into()
        .expect("sha256 digest is always 32 bytes")
}

fn try_build_created(order: &CreatedOrderEvent, id: [u8; 32]) -> Option<OrderCreated> {
    let overflow_part: [u8; 16] = order.order.give.amount[..16].try_into().ok()?;
    if !overflow_part.iter().all(|b| *b == 0u8) {
        return None;
    }

    let amount: [u8; 16] = order.order.give.amount[16..32]
        .try_into()
        .expect("slice 16..32 is always 16 bytes");

    let mint = order.order.give.mint.clone().try_into().ok()?;

    Some(OrderCreated {
        order_id: id,
        give_amount: u128::from_be_bytes(amount),
        give_token: Pubkey::new_from_array(mint),
    })
}

#[cfg(test)]
mod tests {
    use solana_sdk::signature::Signature;

    use crate::domain::parser::events::{Offer, Order};

    use super::*;

    #[test]
    fn parse_fulfilled() {
        let logs = vec!["Program data: 0q6D1Si2U25xJg6fclWE4Mg65YKWB9zfviorRehVNFseLrrf3hE6DBvd7TdfV8J3SPCc5St6bi3Eb2wE1h2kflV0fk/tfnkx".to_string()];
        let tx = Transaction {
            signature: Signature::new_unique(),
            logs,
            blocktime: 0,
        };
        let events = parse_transaction(&tx).order_events;
        let OrderEvent::Fulfilled(f) = events.first().unwrap() else {
            panic!("expected Fulfilled");
        };

        assert_eq!(
            f.taker.to_string(),
            "2snHHreXbpJ7UwZxPe37gnUNf7Wx7wv6UKDSR2JckKuS"
        );

        assert_eq!(
            [
                113, 38, 14, 159, 114, 85, 132, 224, 200, 58, 229, 130, 150, 7, 220, 223, 190, 42,
                43, 69, 232, 85, 52, 91, 30, 46, 186, 223, 222, 17, 58, 12
            ],
            f.order_id
        )
    }

    #[test]
    fn parse_order_created() {
        let logs = vec![
            "Program data: aldK5iB4R9JjVIcInwEAACAAAABtpy2XFQT8HHppHw5pew3wrUrAZlBlxPm2CJoZnTzj0gAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAc29sIAAAAMb6evO+2606PWXzaqvJdDGxu+TC0vbg5HymAgNFL11hAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAGMHLwAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAiRQAAADCEy0F0xyRSofGYRwQdIrrBLWOjwAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAABgECRFAAAAGpPw5oHq3kyBJZkESLbLbPBTTCLIAAAAG2nLZcVBPwcemkfDml7DfCtSsBmUGXE+bYImhmdPOPSFAAAAAdG5+TRXzCIVha0rD0nQ5M1ToDAARQAAABVXOI2wCIGlbaDQbxIxo1SIQzDWwEgAAAAbactlxUE/Bx6aR8OaXsN8K1KwGZQZcT5tgiaGZ0849IAwOHkAAAAAACUKAAAAAAAAA==".to_string(),
            "Program data: uykbSQw/DVTF/aEMklxhaGGpnFy6FSVZh5tFSzODeuK4AJv2wDnDxA==".to_string()
        ];
        let tx = Transaction {
            signature: Signature::new_unique(),
            logs,
            blocktime: 0,
        };
        let events = parse_transaction(&tx).order_events;

        let OrderEvent::Created(order) = events.first().unwrap() else {
            panic!("expected created");
        };

        assert_eq!(events.len(), 1);
        assert_eq!(
            [
                197, 253, 161, 12, 146, 92, 97, 104, 97, 169, 156, 92, 186, 21, 37, 89, 135, 155,
                69, 75, 51, 131, 122, 226, 184, 0, 155, 246, 192, 57, 195, 196
            ],
            order.order_id
        );
        assert_eq!(order.give_amount, 25_959_612);
        assert_eq!(
            order.give_token.to_string(),
            "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v"
        );
    }

    #[test]
    fn empty_events_in_empty_logs() {
        let logs = vec![];

        let tx = Transaction {
            signature: Signature::new_unique(),
            logs,
            blocktime: 0,
        };
        let events = parse_transaction(&tx).order_events;
        assert!(events.is_empty());
    }

    #[test]
    fn spam_logs_without_program_data() {
        let logs = vec![" data: ==".to_string(), "==".to_string()];
        let tx = Transaction {
            signature: Signature::new_unique(),
            logs,
            blocktime: 0,
        };
        let events = parse_transaction(&tx).order_events;
        assert!(events.is_empty());
    }

    #[test]
    fn created_order_without_following_order_id() {
        let logs = vec![
            "Program data: aldK5iB4R9JjVIcInwEAACAAAABtpy2XFQT8HHppHw5pew3wrUrAZlBlxPm2CJoZnTzj0gAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAc29sIAAAAMb6evO+2606PWXzaqvJdDGxu+TC0vbg5HymAgNFL11hAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAGMHLwAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAiRQAAADCEy0F0xyRSofGYRwQdIrrBLWOjwAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAABgECRFAAAAGpPw5oHq3kyBJZkESLbLbPBTTCLIAAAAG2nLZcVBPwcemkfDml7DfCtSsBmUGXE+bYImhmdPOPSFAAAAAdG5+TRXzCIVha0rD0nQ5M1ToDAARQAAABVXOI2wCIGlbaDQbxIxo1SIQzDWwEgAAAAbactlxUE/Bx6aR8OaXsN8K1KwGZQZcT5tgiaGZ0849IAwOHkAAAAAACUKAAAAAAAAA==".to_string(),
            "Program data: ".to_string()
        ];
        let tx = Transaction {
            signature: Signature::new_unique(),
            logs,
            blocktime: 0,
        };
        let events = parse_transaction(&tx).order_events;
        assert!(events.is_empty());
    }

    #[test]
    fn overflow_u256() {
        let mut amount = [0u8; 32];
        amount[0] = 1; // upper bytes non-zero → overflow

        let event = CreatedOrderEvent {
            order: Order {
                make_order_nonce: 0,
                maker_src: vec![],
                give: Offer {
                    chain_id: [0; 32],
                    mint: vec![0; 32],
                    amount,
                },
                take: Offer {
                    chain_id: [0; 32],
                    mint: vec![0; 32],
                    amount: [0; 32],
                },
                receiver_dst: vec![],
                give_patch_authority_src: vec![],
                order_authority_address_dst: vec![],
                allowed_taker_dst: None,
                allowed_cancel_beneficiary_src: None,
                external_call: None,
            },
            fix_fee: 0,
            percent_fee: 0,
        };

        let result = try_build_created(&event, [0u8; 32]);
        assert!(result.is_none());
    }
}
