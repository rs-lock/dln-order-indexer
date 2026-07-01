# DLN Order Indexer

Backfill indexer for [deBridge DLN](https://docs.dln.trade/) cross-chain orders on Solana. Reads on-chain transactions, parses `OrderCreated` and `OrderFulfilled` events, enriches them with USD prices via Jupiter, stores them in ClickHouse, and serves a dashboard.

## Architecture

```
  Solana RPC (Helius)
        │
        │  getSignaturesForAddress / getTransaction
        │
  ┌─────▼──────────────────────────────────┐
  │  Ingestion Process (Rust)              │
  │                                        │
  │  Worker(SRC) ──┬─► mpsc ──► Writer ──► ClickHouse
  │  Worker(DST) ──┘                       │
  │       │                                │
  │  Jupiter API (price oracle)            │
  │                                        │
  │  SafeRpc = RateLimiter + CircuitBreaker│
  └────────────────────────────────────────┘
        │
  ┌─────▼──────────────────────────────────┐
  │  Dashboard (Next.js + TypeScript)      │
  │                                        │
  │  API Routes ──► ClickHouse (read-only) │
  │  React + Recharts (charts, filters)    │
  └────────────────────────────────────────┘
```

Workers are sharded by program (`DlnSrc`, `DlnDst`), each performing concurrent RPC calls via `buffer_unordered`. A single Writer merges both streams into batched inserts — ClickHouse performs best with infrequent bulk inserts, so a single consumer consolidating the fan-in is a deliberate choice.

## Quick Start

```bash
# 1. Start ClickHouse
docker compose up -d clickhouse

# 2. Apply migrations
cat migrations/init.sql | curl http://localhost:8123 --data-binary @-

# 3. Configure
cp .env.example .env
# Set SOL_RPC to a Helius (or other provider) URL

# 4. Run indexer
cargo run --bin ingestion

# 5. Run dashboard (separate terminal)
cd dashboard
npm install
npm run dev
# Open http://localhost:3000
```

## Design Decisions

### Error Classification

Errors at both I/O boundaries (RPC reads, CH writes) are classified as transient or permanent. Transient (network, timeout, 429) → retry with capped backoff. Permanent (schema mismatch, auth) → fail fast and let the orchestrator restart the process.

Panics inside spawned Tokio tasks do not propagate to the runtime, leaving the system in a degraded state. Errors are returned as `Err` values to `main` instead.

### Delivery Semantics

**at-least-once + idempotent writes.** Data is durable before the cursor advances. A crash mid-page causes replay on restart; `ReplacingMergeTree ORDER BY order_id` collapses duplicates. A duplicate is cheap — a gap requires a full rescan.

### Checkpoint Coordination

Workers send `Data` messages followed by a `Checkpoint` through the same bounded `mpsc` channel. FIFO ordering guarantees all data precedes its checkpoint. The Writer persists the checkpoint after a successful flush — no acknowledgment channel required.

### Cursor Lifecycle

The cursor is loaded from ClickHouse once at startup and tracked in memory. The persisted cursor is a restart-only checkpoint. The Writer flushes with lag, so re-reading the cursor on every fetch would cause repeated processing of the same page.

### Backpressure

Bounded `mpsc::channel(N)`. When CH is unavailable: flush fails → Writer stops consuming → channel fills → workers block on `send().await` → RPC load drops. Memory is bounded by `channel_capacity + batch_size`. The Writer retries flush in-place rather than returning to the recv loop, which would drain the channel into an unbounded buffer.

### Price Oracle

Workers enrich each `OrderCreated` event with a USD price by querying the Jupiter Price API for the `give_token_mint` at ingestion time. Prices are cached in-memory with a TTL to avoid redundant requests for the same token within a short window. The `PriceProvider` trait abstracts the oracle behind a port — the same boundary pattern used for RPC and ClickHouse. If the price lookup fails or the token is unknown, the order is stored with `price_usd = NULL` rather than blocking ingestion.

### Retry Strategy

The process retries in-place rather than crashing when CH is unavailable. The cursor store and the data store are the same CH instance — a restart would fail to read the cursor, resulting in equivalent recovery latency, but with process restart overhead and loss of in-memory batching state.

## Module Structure

```
domain/          — OrderEvent, parser (Borsh deserialization)
application/     — Indexer, Worker, Writer, port traits (Rpc, OrdersRepo, CursorRepo)
infra/           — SolClient, ClickhouseRepo, SafeRpc, RateLimiter, CircuitBreaker, JupiterClient
bin/ingestion.rs — composition root (wiring, config, signal handling)
dashboard/       — Next.js app (API routes + React + Recharts)
```

Port traits live in `application` with neutral error types (`RpcError`, `WriteError`). Infrastructure types do not cross into the core — adapters translate at the boundary. The `Indexer` depends on `Arc<dyn Trait>`, enabling testing with mocks.

## Assumptions

- **Best-effort pricing.** USD prices come from Jupiter at ingestion time, not at order execution time. Price may differ from the actual swap rate. Orders with unknown tokens are stored without a price.
- **`give_amount` as `u128`.** The on-chain field is `u256`; orders exceeding `u128` are not supported by the current implementation.
- **No ClickHouse auth.** Local development only. Production would need network restrictions and credentials.
- **Backfill only.** No live tail — would require WebSocket subscription.
- **Fulfilled volume as count.** The `fulfilled` table carries no `give_amount`; the two programs index opposite sides of cross-chain orders.

## Potential Improvements

- **Adaptive rate limiting (AIMD):** adjust request rate based on 429 responses to converge on actual server capacity.
- **Health-aware RPC routing:** score endpoints by success rate, route to the healthiest, probe unresponsive nodes.
- **Poison signature handling:** cap retries per transaction, dead-letter persistently failing signatures with alerting.
- **Deterministic time:** `Clock` trait in `CircuitBreaker` for time-controlled tests.
- **Structured CH error parsing:** classify error codes numerically rather than via string matching.
- **Live indexing:** WebSocket subscription merged with the backfill cursor.
