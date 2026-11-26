import { useEffect, useRef, useState } from "react";
import { api, isMock, type ResolverHealth } from "../lib/api";
import { ms, num, pct } from "../lib/format";
import { Loc, PageHead, Trace } from "../components/ui";
import type { Route } from "../App";

const WINDOW = 60;

/** داشبورد: یک خط سیگنال زنده (TCP به 1.1.1.1:443 هر ۲ ثانیه) + خلاصه‌ها */
export default function Dashboard({ go }: { go: (r: Route) => void }) {
  const [samples, setSamples] = useState<(number | null)[]>([]);
  const [target, setTarget] = useState("1.1.1.1:443");
  const [running, setRunning] = useState(true);
  const [dns, setDns] = useState<ResolverHealth[] | null>(null);
  const timer = useRef<number>();

  useEffect(() => {
    if (!running) return;
    const tick = async () => {
      try {
        const r = await api.tcpProbe(target, 1);
        setSamples((s) => [...s, r.samples[0] ?? null].slice(-WINDOW));
      } catch {
        setSamples((s) => [...s, null].slice(-WINDOW));
      }
      timer.current = window.setTimeout(tick, 2000);
    };
    tick();
    return () => window.clearTimeout(timer.current);
  }, [running, target]);

  useEffect(() => { api.dnsCompare("example.com").then(setDns).catch(() => setDns([])); }, []);

  const ok = samples.filter((x): x is number => x != null);
  const last = ok.at(-1) ?? null;
  const avg = ok.length ? ok.reduce((a, b) => a + b, 0) / ok.length : null;
  const loss = samples.length ? ((samples.length - ok.length) * 100) / samples.length : null;
  const jitter = ok.length > 1 ? ok.slice(1).reduce((a, v, i) => a + Math.abs(v - ok[i]), 0) / (ok.length - 1) : null;
  const lastIsLost = samples.length > 0 && samples[samples.length - 1] == null;
  const status = loss == null ? "در حال سنجش" : lastIsLost ? "قطع" : loss > 20 ? "ناپایدار" : "متصل";

  // اعلان بومی سیستم‌عامل وقتی اتصال قطع یا وصل می‌شود
  const prev = useRef(status);
  useEffect(() => {
    if (isMock || prev.current === status) return;
    const was = prev.current;
    prev.current = status;
    if (status === "قطع" || (was === "قطع" && status === "متصل")) {
      import("@tauri-apps/plugin-notification").then(async (n) => {
        if (!(await n.isPermissionGranted()) && (await n.requestPermission()) !== "granted") return;
        n.sendNotification({ title: "NetDesk", body: status === "قطع" ? `اتصال به ${target} قطع شد` : "اتصال برقرار شد" });
      });
    }
  }, [status, target]);

  return (
    <>
      <PageHead title="وضعیت شبکه" lead={isMock ? "نمای طراحی با داده‌ی شبیه‌سازی‌شده. در برنامه‌ی دسکتاپ اعداد واقعی هستند." : undefined}>
        <Loc kind="local" />
      </PageHead>

      <section className="trace" aria-live="polite">
        <div className="trace-read">
          <div className="big">{last == null ? "—" : num(last, 0)}<small>ms</small></div>
          <dl>
            <div><dt>وضعیت</dt><dd><span className={`dot ${status === "متصل" ? "ok live" : status === "قطع" ? "bad" : "warn"}`} />{status}</dd></div>
            <div><dt>میانگین {num(WINDOW, 0)} نمونه</dt><dd>{ms(avg)}</dd></div>
            <div><dt>Jitter</dt><dd>{ms(jitter)}</dd></div>
            <div><dt>بسته‌ی گم‌شده</dt><dd>{pct(loss)}</dd></div>
          </dl>
          <div className="row" style={{ marginInlineStart: "auto" }}>
            <input className="ltr" value={target} onChange={(e) => setTarget(e.target.value)} aria-label="مقصد پایش" size={18} />
            <button onClick={() => setRunning((r) => !r)}>{running ? "توقف پایش" : "ادامه‌ی پایش"}</button>
          </div>
        </div>
        <Trace samples={samples.length ? samples : [null]} />
        <p className="muted" style={{ margin: "6px 0 0", fontSize: "var(--t-xs)" }}>
          زمان دست‌دهی TCP هر ۲ ثانیه؛ میله‌ی قرمز یعنی پاسخی نیامد. این روش به دسترسی مدیر نیاز ندارد.
        </p>
      </section>

      <div className="grid g2" style={{ marginTop: 16 }}>
        <section className="panel">
          <div className="row" style={{ justifyContent: "space-between" }}>
            <h2>سریع‌ترین DNSها برای شما</h2><Loc kind="public" />
          </div>
          {!dns ? <p className="muted">در حال پرس‌وجو…</p> : (
            <table>
              <thead><tr><th>Resolver</th><th>میانگین</th><th>گم‌شده</th><th>امتیاز</th></tr></thead>
              <tbody>
                {dns.slice(0, 5).map((r) => (
                  <tr key={r.address}>
                    <td>{r.name} <span className="muted ltr">{r.address.replace(":53", "")}</span></td>
                    <td>{ms(r.summary.avgMs)}</td>
                    <td>{pct(r.summary.lossPct)}</td>
                    <td>{num(r.summary.score, 0)}</td>
                  </tr>
                ))}
              </tbody>
            </table>
          )}
          <button style={{ marginTop: 12 }} onClick={() => go("diagnostics")}>همه‌ی تست‌های DNS</button>
        </section>

        <section className="panel">
          <h2>کارهای رایج</h2>
          <div className="grid" style={{ gap: 8 }}>
            <button onClick={() => go("analyzer")}>ساخت گزارش کامل عیب‌یابی</button>
            <button onClick={() => go("home")}>اتصال هوشمند و اشتراک‌ها</button>
            <button onClick={() => go("servers")}>افزودن سرور شخصی (SSH / RDP)</button>
          </div>
          <p className="muted" style={{ fontSize: "var(--t-sm)" }}>
            این برنامه به‌تنهایی اتصال اینترنتی «رایگان» نمی‌سازد. ابزارهای این صفحه شبکه‌ی فعلی شما را می‌سنجند؛
            اتصال از طریق پروکسی یا VPN همیشه به سروری نیاز دارد که خودتان یا ارائه‌دهنده‌تان فراهم می‌کند.
          </p>
        </section>
      </div>
    </>
  );
}
