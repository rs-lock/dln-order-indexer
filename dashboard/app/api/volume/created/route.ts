import { clickhouse } from "@/lib/clickhouse";

export async function GET(request: Request) {

    const { searchParams } = new URL(request.url);

    const now = new Date();
    const thirtyDaysAgo = new Date(now);
    thirtyDaysAgo.setDate(now.getDate() - 30);


    const from = searchParams.get("from") ?? thirtyDaysAgo.toISOString().split("T")[0];
    const to = searchParams.get("to") ?? now.toISOString().split("T")[0];

    const result = await clickhouse.query({
        query: `SELECT date,
                SUM(give_amount / pow(10, decimals) * price_usd) AS total_usd
                FROM created FINAL
                WHERE date >= {from:Date} AND date <= {to:Date}
                AND price_usd IS NOT NULL
                GROUP BY date
                ORDER BY date
        `,
        query_params: {
            from,
            to
        },
        format: "JSONEachRow",
    });

    const rows = await result.json();
    return Response.json(rows);
}