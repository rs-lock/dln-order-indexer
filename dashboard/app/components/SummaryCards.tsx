type Props = {
  createdTotal: string;
  fulfilledTotal: string;
  minDate: string;
  maxDate: string;
};

const cardStyle: React.CSSProperties = {
  backgroundColor: "#1a1a2e",
  border: "1px solid #333",
  borderRadius: 12,
  padding: "20px 24px",
  minWidth: 180,
};

const labelStyle: React.CSSProperties = {
  fontSize: 13,
  color: "#888",
  marginBottom: 4,
};

const valueStyle: React.CSSProperties = {
  fontSize: 28,
  fontWeight: 700,
  color: "#fff",
};

export function SummaryCards(props: Props) {
  return (
    <div style={{ display: "flex", gap: 16, marginBottom: 32 }}>
      <div style={cardStyle}>
        <div style={labelStyle}>Created Orders</div>
        <div style={valueStyle}>{Number(props.createdTotal).toLocaleString()}</div>
      </div>
      <div style={cardStyle}>
        <div style={labelStyle}>Fulfilled Orders</div>
        <div style={valueStyle}>{Number(props.fulfilledTotal).toLocaleString()}</div>
      </div>
      <div style={cardStyle}>
        <div style={labelStyle}>Date Range</div>
        <div style={{ ...valueStyle, fontSize: 20 }}>{props.minDate} — {props.maxDate}</div>
      </div>
    </div>
  );
}
