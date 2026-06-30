import { clickhouse } from "@/lib/clickhouse";

export async function GET(request: Request) {

    const { searchParams } = new URL(request.url);

    const now = new Date();
    const thirtyDaysAgo = new Date();
    thirtyDaysAgo.setDate(now.getDate() - 30);

    const from = searchParams.get("from") ?? thirtyDaysAgo.toISOString().split("T")[0];
    const to = searchParams.get("to") ?? now.toISOString().split("T")[0];


    const result = await clickhouse.query({
        query: `
                SELECT date, count() AS total_orders
                FROM fulfilled
                WHERE date >= {from:Date} AND date <= {to:Date}
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