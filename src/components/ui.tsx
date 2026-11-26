import { useEffect, useState, type ReactNode } from "react";

/** نشان منشأ قابلیت — الزام سند: تفکیک واضح محلی/خارجی */
export function Loc({ kind }: { kind: "local" | "public" | "account" }) {
  const label = { local: "محلی و رایگان", public: "سرور عمومی، بدون حساب", account: "نیازمند سرور یا حساب شما" }[kind];
  return <span className={`loc ${kind}`} title={label}>{label}</span>;
}

export function PageHead({ title, lead, children }: { title: string; lead?: string; children?: ReactNode }) {
  return (
    <div className="page-head">
      <div>
        <h1>{title}</h1>
        {lead && <p>{lead}</p>}
      </div>
      {children && <div className="row">{children}</div>}
    </div>
  );
}

/** خط سیگنال: نمودار SVG بدون کتابخانه. null یعنی بسته‌ی گم‌شده (نقطه‌ی قرمز). */
export function Trace({ samples, height = 180, max }: { samples: (number | null)[]; height?: number; max?: number }) {
  const w = 1000;
  const vals = samples.filter((x): x is number => x != null);
  const top = max ?? Math.max(60, ...vals) * 1.15;
  const step = samples.length > 1 ? w / (samples.length - 1) : w;
  const y = (v: number) => height - (v / top) * height;
  // نمونه‌های پیوسته را به بخش‌های جدا می‌شکنیم؛ بسته‌ی گم‌شده خط را قطع می‌کند
  const segs: [number, number][][] = [];
  samples.forEach((v, i) => {
    if (v == null) return;
    const pt: [number, number] = [i * step, y(v)];
    if (i > 0 && samples[i - 1] != null && segs.length) segs[segs.length - 1].push(pt);
    else segs.push([pt]);
  });
  const path = (s: [number, number][]) => s.map(([x, yy], k) => `${k ? "L" : "M"}${x.toFixed(1)},${yy.toFixed(1)}`).join("");
  const lines = [0.25, 0.5, 0.75].map((f) => height * f);
  return (
    <svg viewBox={`0 0 ${w} ${height}`} preserveAspectRatio="none" role="img" aria-label="نمودار تأخیر">
      {lines.map((ly) => <line key={ly} className="gridline" x1="0" x2={w} y1={ly} y2={ly} />)}
      {segs.map((s, k) => (
        <g key={k}>
          <path className="area" opacity=".5" d={`${path(s)}L${s[s.length - 1][0]},${height}L${s[0][0]},${height}Z`} />
          <path className="line" d={s.length > 1 ? path(s) : `${path(s)}h1`} vectorEffect="non-scaling-stroke" />
        </g>
      ))}
      {samples.map((v, i) => v == null && <rect key={i} className="lost" x={i * step - 2} y={height - 10} width="4" height="10" />)}
    </svg>
  );
}

export function Meter({ label, value }: { label: string; value: number | null }) {
  const v = Math.max(0, Math.min(100, value ?? 0));
  const cls = v > 90 ? "bad" : v > 75 ? "warn" : "";
  return (
    <div className="meter">
      <span className="muted">{label}</span>
      <span className="bar"><i className={cls} style={{ width: `${v}%` }} /></span>
      <span>{value == null ? "—" : `${Math.round(v).toLocaleString("fa-IR")}٪`}</span>
    </div>
  );
}

let pushToast: (m: string) => void = () => {};
export const toast = (m: string) => pushToast(m);
export function Toaster() {
  const [msg, setMsg] = useState<string | null>(null);
  useEffect(() => {
    pushToast = (m) => { setMsg(m); setTimeout(() => setMsg(null), 3200); };
  }, []);
  return msg ? <div className="toast" role="status">{msg}</div> : null;
}

export function useAsync<T>(fn: () => Promise<T>, deps: unknown[] = []) {
  const [data, setData] = useState<T | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(false);
  const run = async () => {
    setLoading(true); setError(null);
    try { setData(await fn()); } catch (e) { setError(String(e)); } finally { setLoading(false); }
  };
  // eslint-disable-next-line react-hooks/exhaustive-deps
  useEffect(() => { run(); }, deps);
  return { data, error, loading, run, setData };
}
