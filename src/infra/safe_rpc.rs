use anyhow::Ok;
use async_trait::async_trait;

use crate::{
    application::indexer::Rpc,
    infra::{circuit_breaker::CircuitBreaker, rate_limiter::RateLimiter},
};

pub struct SafeRpc<R: Rpc> {
    client: R,
    rate_limiter: RateLimiter,
    circuit_breaker: CircuitBreaker,
}

impl<R> SafeRpc<R>
where
    R: Rpc,
{
    pub fn new(client: R, rate_limiter: RateLimiter, circuit_breaker: CircuitBreaker) -> Self {
        Self {
            client,
            rate_limiter,
            circuit_breaker,
        }
    }

    fn handle_error(&self, error: solana_client::client_error::ClientError) {
        match error.kind {
            solana_client::client_error::ClientErrorKind::RpcError(rpc_error) => match rpc_error {
                solana_client::rpc_request::RpcError::RpcResponseError {
                    code,
                    message,
                    data,
                } => if code == 429 {},
                _ => todo!(),
            },
            _ => todo!(),
        }
    }
}

#[async_trait]
impl<R> Rpc for SafeRpc<R>
where
    R: Rpc + Send + Sync,
{
    async fn get_signatures(
        &self,
        program: String,
        cursor_tx: &str,
        limit: u32,
    ) -> anyhow::Result<Vec<String>> {
        let true = self.circuit_breaker.allow_request() else {
            return Ok(vec![]);
        };

        self.rate_limiter.until_ready().await;

        self.client.get_signatures(program, cursor_tx, limit).await
    }

    async fn get_transaction_batch(&self, sig: &str) -> anyhow::Result<Vec<String>> {
        let true = self.circuit_breaker.allow_request() else {
            return Ok(vec![]);
        };

        self.rate_limiter.until_ready().await;
        self.client.get_transaction_batch(sig).await
    }
}
