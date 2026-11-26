import { useState } from "react";
import { api, isMock, type Tool } from "../lib/api";
import { PageHead, toast, useAsync } from "../components/ui";

const riskLabel = { medium: "ریسک متوسط", high: "پرخطر", critical: "بحرانی" } as const;

async function pick(directory = false): Promise<string | null> {
  if (isMock) return null;
  const { open } = await import("@tauri-apps/plugin-dialog");
  const r = await open({ multiple: false, directory });
  return typeof r === "string" ? r : null;
}

async function askUser(message: string): Promise<boolean> {
  if (isMock) return window.confirm(message);
  const { ask } = await import("@tauri-apps/plugin-dialog");
  return ask(message, { title: "تأیید اجرای ابزار پرخطر", kind: "warning", okLabel: "بله، اجرا کن", cancelLabel: "انصراف" });
}

const GATE_KEY = "advanced-tools-gate";
const readGate = () => { try { return localStorage.getItem(GATE_KEY) === "1"; } catch { return false; } };

/** ابزارهای خارجی اختیاری و پرخطر. همه‌ی قواعد تأیید در Rust هم بررسی می‌شوند. */
export default function Tools({ tutorial }: { tutorial: (id: string) => void }) {
  const [gate, setGate] = useState(readGate);
  const { data, run: reload, error } = useAsync(api.toolList, []);
  const [consentFor, setConsentFor] = useState<Tool | null>(null);

  if (!gate) {
    return (
      <>
        <PageHead title="هسته و ابزارها">
          <button onClick={() => tutorial("sing-box")}>آموزش sing-box</button>
        </PageHead>
        <section className="danger-box" style={{ maxWidth: 760 }}>
          <h2>پیش از ادامه بخوانید</h2>
          <p>این بخش برنامه‌هایی را اجرا می‌کند که <b>جزو NetDesk نیستند</b>: هسته‌های پروکسی (sing-box، Xray) و ابزارهای دور زدن فیلترینگ که بررسی امنیتی آن‌ها در «مخازن مرجع» آمده است.</p>
          <ul>
            <li>برخی از آن‌ها <b>گواهی CA ریشه</b> نصب می‌کنند و همه‌ی ترافیک HTTPS شما را باز می‌کنند.</li>
            <li>برخی <b>دسترسی مدیر سیستم</b> می‌خواهند و بسته‌های شبکه را دست‌کاری می‌کنند.</li>
            <li>برخی از سرویس‌های Google خارج از کاربرد مجاز استفاده می‌کنند و ممکن است <b>حساب شما تعلیق شود</b>.</li>
            <li>NetDesk هیچ‌کدام را خودش اجرا یا دانلود نمی‌کند. طبق آموزش، فایل را خودتان می‌سازید؛ برنامه هش آن را ثبت می‌کند و اگر فایل عوض شود، اجرا را متوقف می‌کند.</li>
            <li>برای هر ابزار باید تک‌تک هشدارها را تأیید کنید و عبارت تأیید را تایپ کنید.</li>
          </ul>
          <div className="row" style={{ marginTop: 12 }}>
            <button className="primary" onClick={() => { try { localStorage.setItem(GATE_KEY, "1"); } catch { /* حالت خصوصی */ } setGate(true); }}>
              متوجه شدم، فهرست را نشان بده
            </button>
          </div>
        </section>
      </>
    );
  }

  return (
    <>
      <PageHead title="هسته و ابزارها" lead="sing-box برای اتصال لازم است؛ بقیه اختیاری و پرخطرند. هر کدام با آموزش، هشدار جداگانه، هش سنجاق‌شده و تأیید صریح.">
        <button onClick={reload}>تازه‌سازی</button>
        <button onClick={() => { try { localStorage.removeItem(GATE_KEY); } catch { /* */ } setGate(false); }}>پنهان کردن</button>
      </PageHead>
      {error && <p style={{ color: "var(--bad)" }}>{error}</p>}
      <div className="grid g2">
        {(data ?? []).map((t) => <ToolCard key={t.id} t={t} reload={reload} tutorial={tutorial} onConsent={() => setConsentFor(t)} />)}
      </div>
      {consentFor && <ConsentDialog t={consentFor} close={() => setConsentFor(null)} done={() => { setConsentFor(null); reload(); }} />}
    </>
  );
}

function ToolCard({ t, reload, onConsent, tutorial }: { t: Tool; reload: () => void; onConsent: () => void; tutorial: (id: string) => void }) {
  const [path, setPath] = useState(t.setup?.path ?? "");
  const [args, setArgs] = useState(t.setup?.args ?? "");
  const [cwd, setCwd] = useState(t.setup?.cwd ?? "");
  const [log, setLog] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const dirty = path !== (t.setup?.path ?? "") || args !== (t.setup?.args ?? "") || cwd !== (t.setup?.cwd ?? "");
  const alive = t.running?.alive ?? false;

  const wrap = (f: () => Promise<unknown>) => async () => {
    setBusy(true);
    try { await f(); } catch (e) { toast(String(e)); } finally { setBusy(false); reload(); }
  };

  const save = wrap(async () => {
    await api.toolConfigure(t.id, path, args, cwd || null);
    toast("مسیر و هش ثبت شد؛ حالا هشدارها را تأیید کنید");
  });

  const detect = wrap(async () => {
    const found = await api.toolDetect(t.id);
    if (!found) { toast("پیدا نشد. اول طبق آموزش آن را بسازید."); return; }
    setPath(found.path); setArgs((a) => a || found.args); setCwd((c) => c || (found.cwd ?? ""));
    toast("پیدا شد؛ حالا «ثبت مسیر و هش» را بزنید");
  });

  const start = wrap(async () => {
    const extra = [
      t.installsRootCa && "این ابزار گواهی CA ریشه نصب می‌کند و ترافیک HTTPS را باز می‌کند.",
      t.requiresAdmin && "این ابزار با دسترسی مدیر سیستم اجرا می‌شود (پنجره‌ی UAC).",
      t.unreviewed && "این پروژه هرگز بررسی نشده است.",
    ].filter(Boolean).join("\n");
    if (!(await askUser(`«${t.name}» اجرا شود؟\n\n${extra}\n\nهش فایل: ${t.setup?.sha256.slice(0, 16)}…`))) return;
    const r = await api.toolStart(t.id);
    toast(r.elevated ? "درخواست اجرا با دسترسی مدیر فرستاده شد" : "اجرا شد");
  });

  return (
    <article className="panel tool">
      <header>
        <div>
          <h3>{t.name}</h3>
          <button className="linklike" onClick={() => tutorial(t.id)}>آموزش قدم‌به‌قدم</button>
        </div>
        <span className={`risk ${t.risk}`}>{riskLabel[t.risk]}</span>
      </header>
      <div className="flags">
        {t.installsRootCa && <span className="risk high">نصب CA ریشه / رهگیری HTTPS</span>}
        {t.requiresAdmin && <span className="risk high">دسترسی مدیر سیستم</span>}
        {t.unreviewed && <span className="risk critical">بررسی‌نشده</span>}
        <span className="muted" style={{ fontSize: "var(--t-xs)" }}>لایسنس: <span className="ltr">{t.license}</span></span>
      </div>
      <p style={{ margin: 0 }}>{t.summary}</p>

      <details>
        <summary>هشدارها ({t.warnings.length.toLocaleString("fa-IR")})</summary>
        <div className="danger-box" style={{ marginTop: 8 }}>
          <ul>{t.warnings.map((w, i) => <li key={i}>{w}</li>)}</ul>
        </div>
      </details>

      <label>فایل اجرایی
        <div className="row">
          <input className="ltr" style={{ flex: 1 }} value={path} onChange={(e) => setPath(e.target.value)} placeholder="C:\Tools\program.exe" />
          <button onClick={detect} disabled={busy}>پیدا کردن خودکار</button>
          {!isMock && <button onClick={async () => { const p = await pick(); if (p) setPath(p); }}>انتخاب…</button>}
        </div>
      </label>
      {t.id !== "sing-box" && (
        <div className="grid g2">
          <label>آرگومان‌ها <input className="ltr" value={args} onChange={(e) => setArgs(e.target.value)} /></label>
          <label>پوشه‌ی کاری
            <div className="row">
              <input className="ltr" style={{ flex: 1 }} value={cwd} onChange={(e) => setCwd(e.target.value)} />
              {!isMock && <button onClick={async () => { const p = await pick(true); if (p) setCwd(p); }}>…</button>}
            </div>
          </label>
        </div>
      )}
      <span className="muted" style={{ fontSize: "var(--t-xs)" }}>{t.setupHint}</span>

      {t.setup && (
        <div className="muted" style={{ fontSize: "var(--t-xs)" }}>
          SHA-256 سنجاق‌شده (با checksum صفحه‌ی Releases مقایسه کنید):
          <div className="hash ltr">{t.setup.sha256}</div>
        </div>
      )}

      <div>
        {!t.setup ? <><span className="dot warn" />تنظیم نشده</>
          : !t.consented ? <><span className="dot bad" />هشدارها تأیید نشده</>
          : alive ? <><span className="dot live" />در حال اجرا{t.running?.elevated && " (با دسترسی مدیر، خارج از کنترل NetDesk)"}{t.running?.port && <span className="ltr"> — 127.0.0.1:{t.running.port}</span>}</>
          : <><span className="dot ok" />آماده‌ی اجرا</>}
      </div>

      <div className="row">
        <button onClick={save} disabled={busy || !path.trim() || (!dirty && !!t.setup)}>ثبت مسیر و هش</button>
        {t.setup && !dirty && !t.consented && <button className="primary" onClick={onConsent}>خواندن و تأیید هشدارها</button>}
        {t.consented && t.id !== "sing-box" && !alive && <button className="primary" onClick={start} disabled={busy}>اجرا</button>}
        {t.consented && t.id === "sing-box" && <span className="muted" style={{ fontSize: "var(--t-xs)" }}>اتصال از صفحه‌ی «پروفایل‌ها» ← دکمه‌ی «اتصال»</span>}
        {alive && <button onClick={wrap(async () => toast(await api.toolStop(t.id)))}>توقف</button>}
        {t.setup && <button onClick={async () => setLog(log == null ? await api.toolLog(t.id).catch(String) : null)}>{log == null ? "لاگ" : "بستن لاگ"}</button>}
        {t.setup && <button onClick={wrap(async () => { await api.toolForget(t.id); setPath(""); setArgs(""); setCwd(""); toast("تنظیمات و تأیید حذف شد"); })}>فراموش کن</button>}
      </div>
      {log != null && <pre>{log || "(لاگ خالی است)"}</pre>}
    </article>
  );
}

function ConsentDialog({ t, close, done }: { t: Tool; close: () => void; done: () => void }) {
  const [ack, setAck] = useState<boolean[]>(() => t.warnings.map(() => false));
  const [phrase, setPhrase] = useState("");
  const all = ack.every(Boolean);

  const submit = async () => {
    try {
      await api.toolConsent(t.id, ack.flatMap((a, i) => (a ? [i] : [])), phrase);
      toast(`تأیید «${t.name}» ثبت شد`);
      done();
    } catch (e) { toast(String(e)); }
  };

  return (
    <div className="modal-back" role="presentation" onClick={close}>
      <div className="modal" role="dialog" aria-modal="true" aria-labelledby="consent-title" onClick={(e) => e.stopPropagation()}>
        <h2 id="consent-title">تأیید خطرهای «{t.name}»</h2>
        <p>هر مورد را بخوانید و جداگانه تیک بزنید. این تأیید فقط برای همین فایل با همین هش معتبر است.</p>
        {t.warnings.map((w, i) => (
          <label key={i} className="ack">
            <input type="checkbox" checked={ack[i]} onChange={(e) => setAck((a) => a.map((x, k) => (k === i ? e.target.checked : x)))} />
            <span>{w}</span>
          </label>
        ))}
        <p style={{ marginTop: 14 }}>برای ادامه دقیقاً بنویسید: <span className="phrase">{t.confirmPhrase}</span></p>
        <input style={{ width: "100%" }} value={phrase} onChange={(e) => setPhrase(e.target.value)} disabled={!all} aria-label="عبارت تأیید" />
        <div className="row" style={{ marginTop: 14 }}>
          <button className="primary" onClick={submit} disabled={!all || !phrase.trim()}>ثبت تأیید</button>
          <button onClick={close}>انصراف</button>
        </div>
      </div>
    </div>
  );
}
