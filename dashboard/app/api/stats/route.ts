import { clickhouse } from "@/lib/clickhouse"


export async function GET(request: Request) {

    const [created, fulfilled] = await Promise.all([
        clickhouse.query({
            query: `SELECT count() AS total, min(date) AS min_date, max(date) AS max_date
                    FROM created`,
            format: "JSONEachRow"
        }),
        clickhouse.query({
            query: `SELECT count() AS total, min(date) AS min_date, max(date) AS max_date
                    FROM fulfilled`,
            format: "JSONEachRow"
        })
    ]);

    const [createdStats] = await created.json();
    const [fulfilledStats] = await fulfilled.json();

    return Response.json({
        created: createdStats,
        fulfilled: fulfilledStats,
    });

}