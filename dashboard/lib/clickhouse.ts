import { createClient } from "@clickhouse/client-web";

export const clickhouse = createClient({
    url: process.env.CLICKHOUSE_URL ?? "http://localhost:8123",
    database: "orders",
});