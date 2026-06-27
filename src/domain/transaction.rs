use solana_sdk::signature::Signature;

pub struct Transaction {
    pub signature: Signature,
    pub logs: Vec<String>,
    pub blocktime: i64,
}
