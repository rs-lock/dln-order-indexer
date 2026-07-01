use std::{num::NonZeroU32, sync::Arc, time::Duration};

use anyhow::Context;
use tokio_util::sync::CancellationToken;
use tracing_subscriber::EnvFilter;

use debridge::{
    DST, SOL_RPC, SRC,
    application::{
        indexer::Indexer,
        ports::{CursorRepo, OrdersRepo, Rpc},
    },
    infra::{
        circuit_breaker::CircuitBreaker, clickhouse::repo::ClickhouseRepo,
        jupiter_client::JupiterClient, rate_limiter::RateLimiter, rpc_client::SolClient,
        safe_rpc::SafeRpc,
    },
};

struct Config {
    sol_rpc: String,
    clickhouse_url: String,
    clickhouse_user: String,
    clickhouse_password: String,
    clickhouse_database: String,
    batch_size: usize,
    flush_interval: Duration,
    rpc_rate_per_sec: u32,
    channel_capacity: usize,
    page_size: usize,
    jupiter_url: String,
    price_cache_ttl: Duration,
}

impl Config {
    fn from_env() -> Self {
        let var = |key: &str| std::env::var(key).ok();
        Self {
            sol_rpc: var("SOL_RPC").unwrap_or_else(|| SOL_RPC.to_string()),
            clickhouse_url: var("CLICKHOUSE_URL").unwrap_or_else(|| "http://localhost:8123".into()),
            clickhouse_user: var("CLICKHOUSE_USER").unwrap_or_else(|| "default".into()),
            clickhouse_password: var("CLICKHOUSE_PASSWORD").unwrap_or_default(),
            clickhouse_database: var("CLICKHOUSE_DATABASE").unwrap_or_else(|| "orders".into()),
            batch_size: var("BATCH_SIZE")
                .and_then(|v| v.parse().ok())
                .unwrap_or(1000),
            flush_interval: Duration::from_secs(
                var("FLUSH_INTERVAL_SECONDS")
                    .and_then(|v| v.parse().ok())
                    .unwrap_or(5),
            ),
            rpc_rate_per_sec: var("RPC_RATE_LIMIT")
                .and_then(|v| v.parse().ok())
                .unwrap_or(10),
            channel_capacity: var("CHANNEL_CAPACITY")
                .and_then(|v| v.parse().ok())
                .unwrap_or(10_000),
            page_size: var("PAGE_SIZE")
                .and_then(|v| v.parse().ok())
                .unwrap_or(1000),
            jupiter_url: var("JUPITER_URL").unwrap_or_else(|| "https://api.jup.ag".into()),
            price_cache_ttl: Duration::from_secs(
                var("PRICE_CACHE_TTL_SEC")
                    .and_then(|v| v.parse().ok())
                    .unwrap_or(60),
            ),
        }
    }
}

async fn shutdown_signal() -> anyhow::Result<()> {
    use tokio::signal::unix::{SignalKind, signal};
    let mut sigterm = signal(SignalKind::terminate())?;
    tokio::select! {
        res = tokio::signal::ctrl_c() => {
            res?;
            tracing::info!("received Ctrl-C");
        }
        _ = sigterm.recv() => {
            tracing::info!("received SIGTERM");
        }
    }
    Ok(())
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();

    let cfg = Config::from_env();

    let client = clickhouse::Client::default()
        .with_url(&cfg.clickhouse_url)
        .with_user(&cfg.clickhouse_user)
        .with_password(&cfg.clickhouse_password)
        .with_database(&cfg.clickhouse_database);
    let repo = Arc::new(ClickhouseRepo::new(client));

    let rate_limiter = RateLimiter::new(
        NonZeroU32::new(cfg.rpc_rate_per_sec).context("RPC_RATE_LIMIT must be > 0")?,
    );
    let circuit_breaker = CircuitBreaker::new(5, Duration::from_secs(30));
    let rpc: Arc<dyn Rpc> = Arc::new(SafeRpc::new(
        SolClient::new(cfg.sol_rpc.clone()),
        rate_limiter,
        circuit_breaker,
    ));

    let price_provider = Arc::new(JupiterClient::new(cfg.jupiter_url, cfg.price_cache_ttl));

    let cancel = CancellationToken::new();

    let signal_cancel = cancel.clone();
    tokio::spawn(async move {
        if shutdown_signal().await.is_ok() {
            tracing::info!("shutdown requested");
            signal_cancel.cancel();
        }
    });

    let indexer = Indexer::new(
        rpc,
        repo.clone() as Arc<dyn OrdersRepo>,
        repo as Arc<dyn CursorRepo>,
        price_provider,
        vec![SRC.to_string(), DST.to_string()],
        cfg.channel_capacity,
        cfg.page_size,
        cfg.batch_size,
        cfg.flush_interval,
    );

    tracing::info!("ingestion started; press Ctrl-C or send SIGTERM to stop");
    let result = indexer.run(cancel).await;
    tracing::info!("ingestion stopped");
    result
}
