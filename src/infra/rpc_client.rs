use std::str::FromStr;

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

use solana_transaction_status_client_types::EncodedConfirmedTransactionWithStatusMeta;

use crate::{
    application::ports::{Rpc, RpcError},
    domain::transaction::Transaction,
};

pub struct SolClient {
    rpc: RpcClient,
}

impl SolClient {
    pub fn new(rpc_url: String) -> Self {
        Self {
            rpc: RpcClient::new(rpc_url),
        }
    }
}

fn map_transaction(
    tx: EncodedConfirmedTransactionWithStatusMeta,
    signature: Signature,
) -> Option<Transaction> {
    let blocktime = tx.block_time?;
    let tx = Transaction {
        signature,
        logs: tx
            .transaction
            .meta
            .and_then(|meta| Option::from(meta.log_messages))
            .unwrap_or_default(),
        blocktime,
    };
    Some(tx)
}

#[async_trait]
impl Rpc for SolClient {
    async fn get_signatures(
        &self,
        program: &str,
        cursor_tx: Option<&str>,
        limit: Option<usize>,
    ) -> Result<Vec<String>, RpcError> {
        let before = cursor_tx
            .map(Signature::from_str)
            .transpose()
            .map_err(|e| RpcError::InvalidCursor(e.to_string()))?;

        let program =
            Pubkey::from_str(program).map_err(|e| RpcError::InvalidProgramId(e.to_string()))?;

        let res = self
            .rpc
            .get_signatures_for_address_with_config(
                &program,
                GetConfirmedSignaturesForAddress2Config {
                    before,
                    limit,
                    commitment: Some(CommitmentConfig::finalized()),
                    ..Default::default()
                },
            )
            .await
            .map_err(anyhow::Error::from)?
            .into_iter()
            .map(|sign| sign.signature)
            .collect();

        Ok(res)
    }

    async fn get_transaction(&self, sig: &str) -> Result<Option<Transaction>, RpcError> {
        let signature = Signature::from_str(sig).expect("Signatures comes valid from rpc");
        let tx_res = self
            .rpc
            .get_transaction_with_config(
                &signature,
                RpcTransactionConfig {
                    commitment: Some(CommitmentConfig {
                        commitment: CommitmentLevel::Finalized,
                    }),
                    max_supported_transaction_version: Some(0),
                    ..Default::default()
                },
            )
            .await;

        match tx_res {
            Ok(tx) => Ok(map_transaction(tx, signature)),
            Err(e) => {
                tracing::info!(%e, " get tx request");
                Err(RpcError::Transport(anyhow::Error::from(e)))
            }
        }
    }
}
