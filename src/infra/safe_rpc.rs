use async_trait::async_trait;

use crate::{
    application::ports::{Rpc, RpcError},
    domain::transaction::Transaction,
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

    async fn with_guard<Fut, T>(&self, fut: Fut) -> Result<T, RpcError>
    where
        Fut: Future<Output = Result<T, RpcError>> + Send,
    {
        if !self.circuit_breaker.allow_request() {
            return Err(RpcError::CircuitOpen);
        }

        self.rate_limiter.until_ready().await;

        let res = fut.await;

        match &res {
            Ok(_) => self.circuit_breaker.handle_success(),
            Err(_) => {
                self.circuit_breaker.handle_failure();
            }
        }
        res
    }
}

#[async_trait]
impl<R> Rpc for SafeRpc<R>
where
    R: Rpc + Send + Sync,
{
    async fn get_signatures(
        &self,
        program: &str,
        cursor_tx: Option<&str>,
        limit: Option<usize>,
    ) -> Result<Vec<String>, RpcError> {
        self.with_guard(self.client.get_signatures(program, cursor_tx, limit))
            .await
    }

    async fn get_transaction(&self, sig: &str) -> Result<Option<Transaction>, RpcError> {
        self.with_guard(self.client.get_transaction(sig)).await
    }
}
