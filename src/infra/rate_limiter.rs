use std::{num::NonZeroU32, sync::Arc};

use governor::Quota;

#[derive(Clone)]
pub struct RateLimiter {
    limiter: Arc<governor::DefaultDirectRateLimiter>,
}

impl RateLimiter {
    pub fn new(per_second: NonZeroU32) -> Self {
        Self {
            limiter: Arc::new(governor::RateLimiter::direct(Quota::per_second(per_second))),
        }
    }

    pub async fn until_ready(&self) {
        self.limiter.until_ready().await
    }
}
