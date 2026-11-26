import { useCallback, useEffect, useState } from "react";
import { api, type AutoSettings } from "../lib/api";
import { useAuto } from "../lib/useAuto";
import { num } from "../lib/format";
import { toast, useAsync } from "../components/ui";
import { copy } from "../components/markdown";
import type { Route } from "../App";

const toggles: { key: "systemProxy" | "watchdog" | "connectOnStart" | "updateSubsOnStart"; title: string; hint: string }[] = [
  { key: "systemProxy", title: "پروکسی سیستم", hint: "مرورگر و بیشتر برنامه‌ها خودکار از تونل رد شوند؛ هنگام قطع برمی‌گردد" },
  { key: "watchdog", title: "جابه‌جایی خودکار", hint: "اگر سرور قطع شد، سراغ بهترین سرور بعدی برو" },
  { key: "connectOnStart", title: "اتصال هنگام باز شدن", hint: "با باز شدن برنامه، اتصال هوشمند اجرا شود" },
  { key: "updateSubsOnStart", title: "تازه کردن اشتراک‌ها", hint: "با باز شدن برنامه، سرورهای تازه گرفته شوند" },
];

/** صفحه‌ی خانه: یک دکمه برای همه‌چیز */
export default function Home({ go, tutorial }: { go: (r: Route) => void; tutorial: (id: string) => void }) {
  const { status, refresh } = useAuto();
  const { data: tools } = useAsync(api.toolList);
  const { data: profiles, run: reloadProfiles } = useAsync(api.listProfiles);
  const testing = status.phase === "testing";
  const conn = status.connected;

  const core = tools?.find((t) => t.id === "sing-box");
  // xhttp فقط با Xray کار می‌کند؛ sing-box آن را اجرا نمی‌کند
  const xrayReady = !!tools?.find((t) => t.id === "xray")?.consented;
  const xhttpAll = profiles?.filter((p) => ["xhttp", "splithttp"].includes(p.params.type ?? p.params.net ?? "")).length ?? 0;
  const xhttp = xrayReady ? 0 : xhttpAll; // با Xray آماده، xhttp هم قابل اتصال است
  const usable = (profiles?.filter((p) => p.hasSecret).length ?? 0) - xhttp;
  const [warnings, setWarnings] = useState<string[]>([]);
  useEffect(() => { api.netEnv().then(setWarnings).catch(() => {}); }, [status.phase === "failed"]);
  const ready = !!core?.consented && usable > 0;

  const connect = async () => {
    window.setTimeout(refresh, 100);
    try { await api.smartConnect(); } catch (e) { if (!String(e).includes("لغو شد")) toast(String(e)); }
    refresh();
  };
  const disconnect = async () => {
    await api.autoDisconnect().catch((e) => toast(String(e)));
    refresh();
  };

  const state = testing ? "testing" : conn ? (conn.healthy ? "on" : "weak") : status.phase === "failed" ? "failed" : "off";
  const title = { testing: "در حال پیدا کردن بهترین سرور…", on: "متصل", weak: "اتصال ضعیف است", failed: "اتصال برقرار نشد", off: "قطع" }[state];

  return (
    <>
      <section className={`hero ${state}`} aria-live="polite">
        <button
          className="power"
          onClick={conn || testing ? disconnect : connect}
          aria-label={conn ? "قطع اتصال" : testing ? "لغو" : "اتصال هوشمند"}
          disabled={!conn && !testing && !ready}
        >
          <svg viewBox="0 0 24 24" aria-hidden="true"><path d="M12 3v8M7 6.3a8 8 0 1 0 10 0" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" /></svg>
          <span>{conn ? "قطع اتصال" : testing ? "لغو" : "اتصال هوشمند"}</span>
        </button>
        <div className="hero-text">
          <h1>{title}</h1>
          {conn ? (
            <>
              <p className="hero-server">{conn.name}</p>
              <dl className="stats">
                <div><dt>تأخیر تونل</dt><dd>{conn.latencyMs == null ? "—" : `${num(conn.latencyMs, 0)} ms`}</dd></div>
                <div><dt>حالت</dt><dd>{conn.tweak}</dd></div>
                <div><dt>پروکسی محلی</dt><dd><button className="linklike ltr" onClick={() => copy(`127.0.0.1:${conn.port}`)}>127.0.0.1:{conn.port} ⧉</button></dd></div>
                <div><dt>پروکسی سیستم</dt><dd>{status.systemProxy ? "روشن" : "خاموش"}</dd></div>
              </dl>
            </>
          ) : (
            <p className="muted">
              {ready
                ? `${num(usable, 0)} پروفایل آماده است${xhttp ? `؛ ${num(xhttp, 0)} پروفایل xhttp فقط با Xray کار می‌کند؛ Xray را در «هسته و ابزارها» ثبت کنید` : ""}. برنامه سریع‌ترین سرور سالم را پیدا و وصل می‌کند.`
                : "پیش از اولین اتصال، دو قدم پایین را کامل کنید."}
            </p>
          )}
        </div>
      </section>

      {!ready && tools && profiles && !conn && (
        <section className="panel" style={{ marginTop: 16 }}>
          <h2>راه‌اندازی</h2>
          <ol className="checklist">
            <li className={core?.consented ? "done" : ""}>
              <div><b>هسته‌ی sing-box</b><span className="muted">نصب از سورس داخل مخزن و تأیید هشدارها</span></div>
              {!core?.consented && <div className="row"><button onClick={() => tutorial("sing-box")}>آموزش</button><button className="primary" onClick={() => go("tools")}>تنظیم هسته</button></div>}
            </li>
            <li className={usable ? "done" : ""}>
              <div><b>سرور شما</b><span className="muted">لینک کانفیگ یا اشتراک از سرور خودتان یا ارائه‌دهنده‌تان</span></div>
              {!usable && <button className="primary" onClick={() => go("profiles")}>وارد کردن لینک</button>}
            </li>
          </ol>
        </section>
      )}

      {warnings.length > 0 && (
        <section className="panel" role="note" style={{ marginTop: 16 }}>
          <h2>نکته‌ی محیط</h2>
          <ul className="steps">{warnings.map((w, i) => <li key={i} className="bad">{w}</li>)}</ul>
        </section>
      )}

      {status.steps.length > 0 && (
        <details className="panel" style={{ marginTop: 16 }} open={testing || status.phase === "failed"}>
          <summary>گزارش لحظه‌ای</summary>
          <ul className="steps">
            {status.steps.slice(-40).map((s, i) => <li key={i} className={s.startsWith("✓") ? "ok" : s.startsWith("✗") || s.startsWith("⚠") ? "bad" : ""}>{s}</li>)}
          </ul>
          {status.phase === "failed" && <button onClick={() => go("assistant")}>از دستیار هوشمند کمک بگیر</button>}
        </details>
      )}

      <div className="grid g2" style={{ marginTop: 16 }}>
        <Automation refresh={refresh} />
        <Subscriptions onUpdated={reloadProfiles} />
      </div>
    </>
  );
}

function Automation({ refresh }: { refresh: () => void }) {
  const [s, setS] = useState<AutoSettings | null>(null);
  useEffect(() => { api.autoSettingsGet().then(setS).catch((e) => toast(String(e))); }, []);
  if (!s) return <section className="panel"><h2>کارهای خودکار</h2><p className="muted">بارگذاری…</p></section>;
  const save = async (next: AutoSettings) => {
    const previous = s;
    setS(next);
    try { await api.autoSettingsSet(next); } catch (e) { setS(previous); toast(String(e)); }
    refresh();
  };
  return (
    <section className="panel">
      <h2>کارهای خودکار</h2>
      {toggles.map((t) => (
        <label key={t.key} className="switch-row">
          <span><b>{t.title}</b><span className="muted">{t.hint}</span></span>
          <input type="checkbox" role="switch" className="switch" checked={s[t.key]} onChange={(e) => save({ ...s, [t.key]: e.target.checked })} />
        </label>
      ))}
    </section>
  );
}

function Subscriptions({ onUpdated }: { onUpdated: () => void }) {
  const [url, setUrl] = useState("");
  const [list, setList] = useState<string[]>([]);
  const [busy, setBusy] = useState(false);
  const reload = useCallback(() => api.subList().then(setList).catch(() => {}), []);
  useEffect(() => { reload(); }, [reload]);

  const update = async () => {
    setBusy(true);
    try {
      const r = await api.subUpdate();
      toast(r.errors.length ? r.errors.join(" | ") : `${num(r.added, 0)} سرور تازه اضافه شد`);
      onUpdated();
    } catch (e) { toast(String(e)); } finally { setBusy(false); }
  };
  const add = async () => {
    try { await api.subAdd(url); setUrl(""); await reload(); await update(); } catch (e) { toast(String(e)); }
  };

  return (
    <section className="panel">
      <h2>اشتراک‌ها</h2>
      <div className="row" style={{ flexWrap: "nowrap", gap: 8 }}>
        <input className="ltr" style={{ flex: 1, minWidth: 0 }} type="password" placeholder="https://…" aria-label="نشانی اشتراک" value={url} onChange={(e) => setUrl(e.target.value)} />
        <button className="primary" onClick={add} disabled={!url.trim() || busy}>افزودن</button>
      </div>
      <ul className="subs">
        {list.map((h, i) => (
          <li key={i}><span className="ltr">{h}</span><button onClick={async () => { await api.subRemove(i); reload(); }}>حذف</button></li>
        ))}
      </ul>
      {list.length > 0 && <button onClick={update} disabled={busy}>{busy ? "در حال دریافت…" : "به‌روزرسانی همه"}</button>}
      <p className="muted" style={{ fontSize: "var(--t-xs)", marginBottom: 0 }}>
        فقط https. نشانی در انبار امن ویندوز می‌ماند و برنامه فقط هنگام به‌روزرسانی به آن وصل می‌شود.
      </p>
    </section>
  );
}
