use std::{
    collections::HashMap,
    sync::RwLock,
    time::{Duration, Instant},
};

use async_trait::async_trait;
use serde::Deserialize;

use crate::application::ports::{PriceError, PriceProvider, TokenPriceInfo};

pub struct JupiterClient {
    http: reqwest::Client,
    base_url: String,
    cache: RwLock<HashMap<String, CachedPrice>>,
    cache_ttl: Duration,
}

impl JupiterClient {
    pub fn new(base_url: String, cache_ttl: Duration) -> Self {
        Self {
            http: reqwest::Client::new(),
            base_url,
            cache: RwLock::new(HashMap::new()),
            cache_ttl,
        }
    }
}

#[derive(Clone, Copy)]
struct CachedPrice {
    info: TokenPriceInfo,
    fetched_at: Instant,
}

#[derive(Deserialize)]
struct JupiterTokenPrice {
    #[serde(rename = "usdPrice")]
    usd_price: f64,
    decimals: u8,
}

#[async_trait]
impl PriceProvider for JupiterClient {
    async fn get_price(&self, mint: &str) -> Result<Option<TokenPriceInfo>, PriceError> {
        let cached = {
            let cache = self.cache.read().unwrap_or_else(|e| e.into_inner());
            cache.get(mint).cloned()
        };

        if let Some(cached) = cached
            && (cached.fetched_at + self.cache_ttl) > Instant::now()
        {
            return Ok(Some(cached.info));
        }

        let url = format!("{}/price/v3?ids={}", self.base_url, mint);
        let resp = self
            .http
            .get(&url)
            .send()
            .await
            .map_err(|e| PriceError::Transport(e.into()))?;

        let body: HashMap<String, JupiterTokenPrice> = resp
            .json()
            .await
            .map_err(|e| PriceError::Transport(e.into()))?;

        match body.get(mint) {
            Some(data) => {
                let info = TokenPriceInfo {
                    usd_price: data.usd_price,
                    decimals: data.decimals,
                };
                self.cache.write().unwrap_or_else(|e| e.into_inner()).insert(
                    mint.to_string(),
                    CachedPrice {
                        info,
                        fetched_at: Instant::now(),
                    },
                );
                Ok(Some(info))
            }
            None => Ok(None),
        }
    }
}
