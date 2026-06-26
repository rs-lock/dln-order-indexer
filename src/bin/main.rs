#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    // TODO: wire config, RPC client, indexer and run until shutdown
    Ok(())
}
