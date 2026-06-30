"use client";

import { useState, useEffect } from "react";
import { SummaryCards } from "./components/SummaryCards";
import { DateRangePicker } from "./components/DateRangePicker";
import { VolumeChart } from "./components/VolumeChart";

function defaultFrom() {
  const d = new Date();
  d.setDate(d.getDate() - 30);
  return d.toISOString().split("T")[0];
}

function defaultTo() {
  return new Date().toISOString().split("T")[0];
}

export default function Home() {
  const [from, setFrom] = useState(defaultFrom());
  const [to, setTo] = useState(defaultTo());
  const [stats, setStats] = useState<any>(null);
  const [chartData, setChartData] = useState<any[]>([]);

  useEffect(() => {
    Promise.all([
      fetch("/api/stats").then((r) => r.json()),
      fetch(`/api/volume/created?from=${from}&to=${to}`).then((r) => r.json()),
      fetch(`/api/volume/fulfilled?from=${from}&to=${to}`).then((r) => r.json()),
    ]).then(([statsData, createdRows, fulfilledRows]) => {
      setStats(statsData);

      const byDate: Record<string, { created: number; fulfilled: number }> = {};

      for (const row of createdRows) {
        byDate[row.date] = { created: Number(row.total_amount), fulfilled: 0 };
      }
      for (const row of fulfilledRows) {
        if (!byDate[row.date]) byDate[row.date] = { created: 0, fulfilled: 0 };
        byDate[row.date].fulfilled = Number(row.total_orders);
      }

      const merged = Object.entries(byDate)
        .map(([date, vals]) => ({ date, ...vals }))
        .sort((a, b) => a.date.localeCompare(b.date));

      setChartData(merged);
    });
  }, [from, to]);

  return (
    <main style={{
      padding: "40px 48px",
      minHeight: "100vh",
      backgroundColor: "#0d0d1a",
      color: "#fff",
    }}>
      <h1 style={{ fontSize: 32, fontWeight: 700, marginBottom: 24 }}>
        DLN Order Dashboard
      </h1>
      <DateRangePicker from={from} to={to} onChange={(f, t) => { setFrom(f); setTo(t); }} />
      {stats && (
        <SummaryCards
          createdTotal={stats.created.total}
          fulfilledTotal={stats.fulfilled.total}
          minDate={stats.created.min_date}
          maxDate={stats.created.max_date}
        />
      )}
      <VolumeChart data={chartData} />
    </main>
  );
}
