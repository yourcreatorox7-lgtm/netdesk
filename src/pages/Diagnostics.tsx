import { useState } from "react";
import { api, type ResolverHealth, type TcpResult } from "../lib/api";
import { ms, num, pct } from "../lib/format";
import { Loc, PageHead, toast } from "../components/ui";

/** ابزارهای حالت رایگان: DNS، اتصال TCP، اعتبارسنجی پروکسی */
export default function Diagnostics() {
  const [domain, setDomain] = useState("example.com");
  const [extra, setExtra] = useState("");
  const [dns, setDns] = useState<ResolverHealth[] | null>(null);
  const [busy, setBusy] = useState<string | null>(null);
  const [tcpTarget, setTcpTarget] = useState("github.com:443");
  const [tcp, setTcp] = useState<TcpResult | null>(null);
  const [proxy, setProxy] = useState({ kind: "socks5" as "socks5" | "http", address: "127.0.0.1:1080" });
  const [proxyRes, setProxyRes] = useState<string | null>(null);

  const wrap = async (k: string, f: () => Promise<void>) => {
    setBusy(k);
    try { await f(); } catch (e) { toast(`خطا: ${e}`); } finally { setBusy(null); }
  };

  return (
    <>
      <PageHead title="عیب‌یابی" lead="همه‌ی تست‌ها از روی همین دستگاه اجرا می‌شوند و حساب کاربری لازم ندارند." />

      <section className="panel">
        <div className="row" style={{ justifyContent: "space-between" }}><h2>مقایسه‌ی resolverهای DNS</h2><Loc kind="public" /></div>
        <form className="row" onSubmit={(e) => { e.preventDefault(); wrap("dns", async () => setDns(await api.dnsCompare(domain, extra.split(/[\s,]+/).filter(Boolean)))); }}>
          <label>دامنه‌ی آزمایشی <input className="ltr" value={domain} onChange={(e) => setDomain(e.target.value)} /></label>
          <label>DNS سفارشی <input className="ltr" placeholder="مثلاً 10.0.0.1" value={extra} onChange={(e) => setExtra(e.target.value)} /></label>
          <button className="primary" disabled={busy === "dns"}>{busy === "dns" ? "در حال تست…" : "تست DNSها"}</button>
        </form>
        {dns && (
          <table style={{ marginTop: 14 }}>
            <thead><tr><th>Resolver</th><th>میانگین</th><th>p95</th><th>Jitter</th><th>گم‌شده</th><th>پاسخ</th><th>امتیاز</th></tr></thead>
            <tbody>
              {dns.map((r) => (
                <tr key={r.address}>
                  <td>{r.name} <span className="muted ltr">{r.address}</span></td>
                  <td>{ms(r.summary.avgMs)}</td><td>{ms(r.summary.p95Ms)}</td><td>{ms(r.summary.jitterMs)}</td>
                  <td>{pct(r.summary.lossPct)}</td>
                  <td className="ltr" title={r.warning ?? ""}>
                    {r.warning && <span className="dot bad" />}{r.answers.join(", ") || r.errors[0] || "—"}
                  </td>
                  <td>{num(r.summary.score, 0)}</td>
                </tr>
              ))}
            </tbody>
          </table>
        )}
        <p className="muted" style={{ fontSize: "var(--t-xs)" }}>
          نقطه‌ی قرمز کنار پاسخ یعنی آدرس خصوصی برگشته — نشانه‌ی احتمالی دست‌کاری یا مسدودسازی DNS، نه حکم قطعی.
        </p>
      </section>

      <div className="grid g2" style={{ marginTop: 16 }}>
        <section className="panel">
          <div className="row" style={{ justifyContent: "space-between" }}><h2>اتصال TCP و Packet loss</h2><Loc kind="local" /></div>
          <form className="row" onSubmit={(e) => { e.preventDefault(); wrap("tcp", async () => setTcp(await api.tcpProbe(tcpTarget, 10))); }}>
            <input className="ltr" value={tcpTarget} onChange={(e) => setTcpTarget(e.target.value)} aria-label="میزبان:پورت" />
            <button className="primary" disabled={busy === "tcp"}>{busy === "tcp" ? "۱۰ تلاش…" : "تست اتصال"}</button>
          </form>
          {tcp && (
            <dl className="grid g3" style={{ marginTop: 12 }}>
              <div><dt className="muted">پورت</dt><dd>{tcp.portOpen ? "باز" : "بسته یا فیلتر"}</dd></div>
              <div><dt className="muted">میانگین / بیشینه</dt><dd>{ms(tcp.summary.avgMs)} / {ms(tcp.summary.maxMs)}</dd></div>
              <div><dt className="muted">گم‌شده</dt><dd>{pct(tcp.summary.lossPct)} از {num(tcp.summary.sent, 0)}</dd></div>
            </dl>
          )}
        </section>

        <section className="panel">
          <div className="row" style={{ justifyContent: "space-between" }}><h2>اعتبارسنجی پروکسی</h2><Loc kind="local" /></div>
          <form className="row" onSubmit={(e) => { e.preventDefault(); wrap("px", async () => { const r = await api.proxyCheck(proxy.kind, proxy.address); setProxyRes(r.works ? "پروکسی کار می‌کند و به example.com:443 وصل شد." : r.verdict); }); }}>
            <select value={proxy.kind} onChange={(e) => setProxy({ ...proxy, kind: e.target.value as "socks5" | "http" })}>
              <option value="socks5">SOCKS5</option><option value="http">HTTP CONNECT</option>
            </select>
            <input className="ltr" value={proxy.address} onChange={(e) => setProxy({ ...proxy, address: e.target.value })} />
            <button className="primary" disabled={busy === "px"}>بررسی</button>
          </form>
          {proxyRes && <p>{proxyRes}</p>}
          <p className="muted" style={{ fontSize: "var(--t-xs)" }}>فقط دست‌دهی پروتکل بررسی می‌شود؛ ترافیکی رمزگشایی یا رهگیری نمی‌شود.</p>
        </section>
      </div>
    </>
  );
}
