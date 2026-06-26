use std::{collections::VecDeque, str::FromStr};

use async_trait::async_trait;
use solana_client::{
    nonblocking::rpc_client::RpcClient, rpc_client::GetConfirmedSignaturesForAddress2Config,
    rpc_config::RpcTransactionConfig,
};
use solana_sdk::{
    commitment_config::{CommitmentConfig, CommitmentLevel},
    pubkey::Pubkey,
    signature::Signature,
};

use solana_transaction_status_client_types::EncodedTransactionWithStatusMeta;

use crate::{application::indexer::Rpc, domain::transaction::Transaction};

pub struct SolClient {
    rpc: RpcClient,
}

impl SolClient {
    pub fn new() -> Self {
        Self {
            rpc: RpcClient::new(crate::SOL_RPC.to_string()),
        }
    }

    async fn get_transactions_batch(
        &self,
        signatures: VecDeque<String>,
    ) -> anyhow::Result<Vec<Transaction>> {
        let mut txs = vec![];
        let signature = signatures.clone().pop_front().unwrap();
        let tx = self
            .rpc
            .get_transaction_with_config(
                &Signature::from_str(&signature)?,
                RpcTransactionConfig {
                    commitment: Some(CommitmentConfig {
                        commitment: CommitmentLevel::Confirmed,
                    }),
                    max_supported_transaction_version: Some(0),
                    ..Default::default()
                },
            )
            .await;

        match tx {
            Ok(tx) => txs.push(map_transaction(tx.transaction, signature)),
            Err(err) => tracing::warn!(%signature, %err, "failed to fetch transaction"),
        }
        Ok(txs)
    }
}

fn map_transaction(tx: EncodedTransactionWithStatusMeta, signature: String) -> Transaction {
    Transaction {
        signature,
        logs: tx
            .meta
            .and_then(|meta| Option::from(meta.log_messages))
            .unwrap_or_default(),
    }
}

#[async_trait]
impl Rpc for SolClient {
    async fn get_signatures(
        &self,
        program: String,
        cursor_tx: &str,
        limit: u32,
    ) -> anyhow::Result<Vec<String>> {
        let res = self
            .rpc
            .get_signatures_for_address_with_config(
                &Pubkey::from_str(&program).unwrap(),
                GetConfirmedSignaturesForAddress2Config {
                    limit: Some(1),
                    ..Default::default()
                },
            )
            .await?
            .into_iter()
            .map(|sign| sign.signature)
            .collect();

        Ok(res)
    }

    async fn get_transaction_batch(&self, sig: &str) -> anyhow::Result<Vec<String>> {
        Ok(vec![])
    }
}
