type Props = {
  from: string;
  to: string;
  onChange: (from: string, to: string) => void;
};

const inputStyle: React.CSSProperties = {
  backgroundColor: "#1a1a2e",
  border: "1px solid #333",
  borderRadius: 8,
  padding: "8px 12px",
  color: "#fff",
  fontSize: 14,
};

export function DateRangePicker({ from, to, onChange }: Props) {
  return (
    <div style={{ display: "flex", gap: 12, alignItems: "center", marginBottom: 24 }}>
      <label style={{ color: "#888", fontSize: 13 }}>From</label>
      <input
        type="date"
        value={from}
        onChange={(e) => onChange(e.target.value, to)}
        style={inputStyle}
      />
      <label style={{ color: "#888", fontSize: 13 }}>To</label>
      <input
        type="date"
        value={to}
        onChange={(e) => onChange(from, e.target.value)}
        style={inputStyle}
      />
    </div>
  );
}
