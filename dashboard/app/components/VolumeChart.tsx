"use client";

import {
  LineChart, Line, XAxis, YAxis, Tooltip, Legend,
  ResponsiveContainer, CartesianGrid,
} from "recharts";

type DataPoint = {
  date: string;
  created: number;
  fulfilled: number;
};

type Props = {
  data: DataPoint[];
};

function formatUsd(value: number) {
  return `$${value.toLocaleString("en-US", { maximumFractionDigits: 0 })}`;
}

export function VolumeChart({ data }: Props) {
  return (
    <ResponsiveContainer width="100%" height={400}>
      <LineChart data={data} margin={{ top: 20, right: 60, left: 20, bottom: 20 }}>
        <CartesianGrid strokeDasharray="3 3" stroke="#333" />
        <XAxis
          dataKey="date"
          tick={{ fill: "#aaa", fontSize: 12 }}
          tickFormatter={(d: string) => {
            const [, m, day] = d.split("-");
            return `${day}.${m}`;
          }}
        />
        <YAxis
          yAxisId="left"
          tick={{ fill: "#8884d8", fontSize: 12 }}
          tickFormatter={formatUsd}
          width={90}
        />
        <YAxis
          yAxisId="right"
          orientation="right"
          tick={{ fill: "#82ca9d", fontSize: 12 }}
          width={50}
        />
        <Tooltip
          contentStyle={{ backgroundColor: "#1a1a2e", border: "1px solid #333", borderRadius: 8 }}
          labelStyle={{ color: "#fff" }}
          formatter={(value, name) => {
            const v = Number(value);
            if (name === "Volume (USD)") return [formatUsd(v), name];
            return [v.toLocaleString(), name];
          }}
        />
        <Legend wrapperStyle={{ paddingTop: 16 }} />
        <Line
          yAxisId="left"
          type="monotone"
          dataKey="created"
          stroke="#8884d8"
          strokeWidth={2}
          dot={{ r: 4 }}
          name="Volume (USD)"
        />
        <Line
          yAxisId="right"
          type="monotone"
          dataKey="fulfilled"
          stroke="#82ca9d"
          strokeWidth={2}
          dot={{ r: 4 }}
          name="Fulfilled (count)"
        />
      </LineChart>
    </ResponsiveContainer>
  );
}
