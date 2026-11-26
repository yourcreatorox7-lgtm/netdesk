import { useCallback, useEffect, useRef, useState } from "react";
import { api, type AiAction, type ChatMessage } from "../lib/api";
import { PageHead, toast } from "../components/ui";
import { copy } from "../components/markdown";
import { num } from "../lib/format";
import type { Route } from "../App";

const suggested = [
  { name: "gemma3:4b", note: "حدود ۳٫۳ گیگابایت؛ فارسی نسبتاً خوب" },
  { name: "qwen2.5:3b", note: "حدود ۱٫۹ گیگابایت؛ سبک‌تر، فارسی ضعیف‌تر" },
];

const actionLabel: Record<Exclude<AiAction, "none">, string> = {
  smart_connect: "اتصال هوشمند",
  disconnect: "قطع اتصال",
  update_subscriptions: "به‌روزرسانی اشتراک‌ها",
  run_diagnostics: "تست شبکه",
  open_profiles: "رفتن به پروفایل‌ها",
  open_tutorials: "رفتن به آموزش‌ها",
  open_tools: "رفتن به تنظیم هسته",
};

/** بیشترین قدم‌های پشت‌سرهم خلبان خودکار برای هر درخواست */
const MAX_AUTO_STEPS = 3;

interface Entry { who: "user" | "ai" | "result"; text: string; action?: AiAction; done?: boolean }

const quick = ["وصلم کن به بهترین سرور", "چرا وصل نمی‌شوم؟", "اینترنتم را تست کن", "چطور شروع کنم؟"];

// تاریخچه با جابه‌جایی بین صفحه‌ها حفظ می‌شود (فقط در حافظه)
let saved: Entry[] = [];

export default function Assistant({ go }: { go: (r: Route) => void }) {
  const [ai, setAi] = useState<{ running: boolean; models: string[] } | null>(null);
  const [model, setModel] = useState(() => { try { return localStorage.getItem("ai-model") ?? ""; } catch { return ""; } });
  const [autopilot, setAutopilot] = useState(false);
  const [entries, setEntries] = useState<Entry[]>(saved);
  const [text, setText] = useState("");
  const [busy, setBusy] = useState<string | null>(null);
  const [pull, setPull] = useState<{ running: boolean; ok: boolean | null; line: string } | null>(null);
  const autoRef = useRef(autopilot);
  autoRef.current = autopilot;
  const end = useRef<HTMLDivElement>(null);

  const refresh = useCallback(() => api.aiStatus().then(setAi).catch(() => setAi({ running: false, models: [] })), []);
  useEffect(() => { refresh(); }, [refresh]);
  useEffect(() => { saved = entries; end.current?.scrollIntoView({ block: "nearest" }); }, [entries, busy]);

  const active = ai?.models.includes(model) ? model : ai?.models[0] ?? "";

  useEffect(() => {
    if (!pull?.running) return;
    const t = window.setInterval(async () => {
      const s = await api.aiPullStatus().catch(() => null);
      if (!s) return;
      setPull(s);
      if (!s.running) { refresh(); toast(s.ok ? "مدل دانلود شد" : "دانلود ناموفق بود؛ دوباره امتحان کنید"); }
    }, 1500);
    return () => window.clearInterval(t);
  }, [pull?.running, refresh]);

  const download = async (name: string) => {
    try { await api.aiPull(name); setPull({ running: true, ok: null, line: "شروع دانلود…" }); } catch (e) { toast(String(e)); }
  };

  /** کارهای مجاز؛ هیچ‌کدام ابزار پرخطر اجرا نمی‌کند. خروجی null یعنی صفحه عوض شد. */
  const perform = async (a: Exclude<AiAction, "none">): Promise<string | null> => {
    switch (a) {
      case "smart_connect": {
        const c = await api.smartConnect();
        return `وصل شد. تأخیر تونل: ${c.latencyMs == null ? "نامعلوم" : `${Math.round(c.latencyMs)} ms`}، حالت: ${c.tweak}`;
      }
      case "disconnect": await api.autoDisconnect(); return "اتصال قطع شد.";
      case "update_subscriptions": {
        const r = await api.subUpdate();
        return `${r.added} سرور تازه. خطاها: ${r.errors.length}`;
      }
      case "run_diagnostics": {
        const [dns, tcp] = await Promise.all([api.dnsCompare("example.com"), api.tcpProbe("1.1.1.1:443", 5)]);
        const good = dns.filter((d) => d.summary.received > 0).length;
        const warned = dns.filter((d) => d.warning).length;
        return `DNS: ${good} از ${dns.length} پاسخ دادند، ${warned} پاسخ مشکوک. TCP به 1.1.1.1:443: ${tcp.summary.received} از ${tcp.summary.sent} موفق، میانگین ${tcp.summary.avgMs == null ? "—" : Math.round(tcp.summary.avgMs) + " ms"}.`;
      }
      case "open_profiles": go("profiles"); return null;
      case "open_tutorials": go("tutorials"); return null;
      case "open_tools": go("tools"); return null;
    }
  };

  const history = (list: Entry[]): ChatMessage[] =>
    list.map((e) => (e.who === "ai" ? { role: "assistant", content: e.text } : { role: "user", content: e.who === "result" ? `[نتیجه‌ی کار] ${e.text}` : e.text }));

  const runAction = async (list: Entry[], a: Exclude<AiAction, "none">): Promise<Entry[] | null> => {
    setBusy(`در حال انجام: ${actionLabel[a]}…`);
    let result: string | null;
    try { result = await perform(a); } catch (e) { result = `ناموفق: ${String(e).split("sing-box اجرا نشد:")[0] || "هسته اجرا نشد"}`; }
    if (result === null) return null;
    const next: Entry[] = [...list, { who: "result", text: result }];
    setEntries(next);
    return next;
  };

  /** مدل پاسخ می‌دهد؛ در خلبان خودکار کار را انجام می‌دهد، نتیجه را می‌بیند و ادامه می‌دهد */
  const think = async (start: Entry[]) => {
    let list = start;
    const done = new Set<AiAction>();
    try {
      for (let step = 0; step <= MAX_AUTO_STEPS; step++) {
        setBusy("مدل در حال فکر کردن است…");
        const r = await api.aiChat(active, history(list));
        const repeat = done.has(r.action);
        const auto = autoRef.current && r.action !== "none" && !repeat && step < MAX_AUTO_STEPS;
        list = [...list, { who: "ai", text: r.reply, action: repeat ? "none" : r.action, done: auto }];
        setEntries(list);
        if (!auto || r.action === "none") break;
        done.add(r.action);
        const next = await runAction(list, r.action);
        if (!next) break;
        list = next;
      }
    } catch (e) {
      setEntries([...list, { who: "result", text: String(e) }]);
    } finally { setBusy(null); }
  };

  const send = (message: string) => {
    const m = message.trim();
    if (!m || busy || !active) return;
    const list: Entry[] = [...entries, { who: "user", text: m }];
    setEntries(list);
    setText("");
    think(list);
  };

  const confirm = async (index: number, a: Exclude<AiAction, "none">) => {
    const marked = entries.map((e, i) => (i === index ? { ...e, done: true } : e));
    setEntries(marked);
    await runAction(marked, a);
    setBusy(null);
  };

  const report = async () => {
    try { await copy(await api.aiReport(), "گزارش بدون اطلاعات حساس کپی شد"); } catch (e) { toast(String(e)); }
  };

  return (
    <>
      <PageHead title="دستیار هوشمند" lead="مدل هوش مصنوعی روی همین دستگاه اجرا می‌شود. رمز، نشانی سرور و لینک اشتراک به آن داده نمی‌شود.">
        <button onClick={report}>کپی گزارش بدون اطلاعات حساس</button>
        <button onClick={() => go("tutorials")}>آموزش</button>
      </PageHead>

      {ai && !ai.running && (
        <section className="panel">
          <h2>Ollama در حال اجرا نیست</h2>
          <p>برنامه‌ی Ollama را باز کنید. اگر نصب نیست، این دستور را در PowerShell بزنید:</p>
          <div className="code"><button className="copy" onClick={() => copy("winget install Ollama.Ollama")}>کپی</button><pre className="ltr">winget install Ollama.Ollama</pre></div>
          <button className="primary" onClick={refresh}>بررسی دوباره</button>
        </section>
      )}

      {ai?.running && !ai.models.length && (
        <section className="panel">
          <h2>یک مدل دانلود کنید</h2>
          <p className="muted">فقط یک بار لازم است. بعد از آن، دستیار بدون اینترنت هم کار می‌کند.</p>
          {suggested.map((s) => (
            <div key={s.name} className="switch-row">
              <span><b className="ltr">{s.name}</b><span className="muted">{s.note}</span></span>
              <button className="primary" disabled={pull?.running} onClick={() => download(s.name)}>دانلود</button>
            </div>
          ))}
          {pull && <p className="ltr muted">{pull.line}</p>}
        </section>
      )}

      {ai?.running && !!ai.models.length && (
        <section className="panel chat">
          <div className="row chat-bar">
            <label>مدل{" "}
              <select className="ltr" value={active} onChange={(e) => { setModel(e.target.value); try { localStorage.setItem("ai-model", e.target.value); } catch { /* حالت خصوصی */ } }}>
                {ai.models.map((m) => <option key={m}>{m}</option>)}
              </select>
            </label>
            <label className="switch-row inline" title={`دستیار خودش کارهای مجاز را انجام می‌دهد (حداکثر ${num(MAX_AUTO_STEPS, 0)} قدم)`}>
              <span><b>خلبان خودکار</b></span>
              <input type="checkbox" role="switch" className="switch" checked={autopilot} onChange={(e) => setAutopilot(e.target.checked)} />
            </label>
            <button onClick={() => setEntries([])} disabled={!!busy || !entries.length}>گفتگوی تازه</button>
          </div>

          <div className="chat-log" aria-live="polite">
            {!entries.length && (
              <div className="chat-empty">
                <p className="muted">چه کاری انجام بدهم؟</p>
                <div className="row">{quick.map((q) => <button key={q} onClick={() => send(q)}>{q}</button>)}</div>
              </div>
            )}
            {entries.map((e, i) => (
              <div key={i} className={`bubble ${e.who}`}>
                {e.text}
                {e.who === "ai" && e.action && e.action !== "none" && !e.done && (
                  <button className="primary" disabled={!!busy} onClick={() => confirm(i, e.action as Exclude<AiAction, "none">)}>انجام بده: {actionLabel[e.action]}</button>
                )}
              </div>
            ))}
            {busy && <div className="bubble result pending">{busy}</div>}
            <div ref={end} />
          </div>

          <form className="row chat-input" onSubmit={(e) => { e.preventDefault(); send(text); }}>
            <input value={text} onChange={(e) => setText(e.target.value)} placeholder="سؤال یا درخواست خود را بنویسید" aria-label="پیام" />
            <button className="primary" type="submit" disabled={!text.trim() || !!busy}>بفرست</button>
          </form>
        </section>
      )}
    </>
  );
}
