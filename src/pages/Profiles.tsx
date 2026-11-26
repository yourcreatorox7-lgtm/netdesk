import { useMemo, useState } from "react";
import { api, type ImportResult, type StoredProfile } from "../lib/api";
import { useAuto } from "../lib/useAuto";
import { Loc, PageHead, toast, useAsync } from "../components/ui";

/** مدیر کانفیگ: ورود، پوشه، برچسب، علاقه‌مندی، تکراری، خروجی */
export default function Profiles() {
  const { data, run: reload, setData } = useAsync(api.listProfiles);
  const [text, setText] = useState("");
  const [folder, setFolder] = useState("");
  const [result, setResult] = useState<ImportResult | null>(null);
  const [q, setQ] = useState("");
  const [activeFolder, setActiveFolder] = useState<string | null>(null);
  const { status, refresh } = useAuto();
  const [connecting, setConnecting] = useState<string | null>(null);
  const conn = status.connected;

  const connect = async (p: StoredProfile) => {
    setConnecting(p.fingerprint);
    try {
      const c = await api.singboxConnect(p.fingerprint);
      toast(`وصل شد — تأخیر تونل ${c.latencyMs == null ? "نامعلوم" : Math.round(c.latencyMs) + " ms"}`);
    } catch (e) { toast(String(e)); } finally { setConnecting(null); refresh(); }
  };
  const disconnect = async () => {
    await api.autoDisconnect().catch((e) => toast(String(e)));
    refresh();
    toast("اتصال قطع شد");
  };

  const folders = useMemo(() => [...new Set((data ?? []).map((p) => p.folder ?? "بدون پوشه"))], [data]);
  const shown = (data ?? [])
    .filter((p) => !activeFolder || (p.folder ?? "بدون پوشه") === activeFolder)
    .filter((p) => !q || `${p.name} ${p.host} ${p.tags.join(" ")}`.toLowerCase().includes(q.toLowerCase()))
    .sort((a, b) => Number(b.favorite) - Number(a.favorite));

  const doImport = async () => {
    try {
      const r = await api.importProfiles(text, folder || undefined);
      setResult(r);
      toast(`${r.added.toLocaleString("fa-IR")} پروفایل افزوده شد`);
      setText("");
      reload();
    } catch (e) { toast(String(e)); }
  };

  const patch = async (p: StoredProfile, change: Partial<StoredProfile>) => {
    setData((d) => d?.map((x) => (x.fingerprint === p.fingerprint ? { ...x, ...change } : x)) ?? null);
    await api.updateProfileMeta(p.fingerprint, change).catch((e) => toast(String(e)));
  };

  const exportAll = async (redacted: boolean) => {
    const out = await api.exportProfiles(shown.map((p) => p.fingerprint), redacted);
    await navigator.clipboard.writeText(out);
    toast(redacted ? "نسخه‌ی بدون رمز کپی شد" : "نسخه‌ی کامل (با رمز) کپی شد — آن را جایی امن نگه دارید");
  };

  return (
    <>
      <PageHead title="پروفایل‌ها" lead="vless، vmess، trojan، shadowsocks، hysteria2، tuic، socks5 و http — هر قالب تجزیه‌گر جداگانه دارد.">
        <Loc kind="local" />
      </PageHead>

      <section className="panel">
        <h2>وارد کردن</h2>
        <textarea placeholder="هر خط یک لینک، یا کل متن Base64 یک اشتراک را این‌جا بچسبانید" value={text} onChange={(e) => setText(e.target.value)} />
        <div className="row" style={{ marginTop: 10 }}>
          <input placeholder="پوشه (اختیاری)" value={folder} onChange={(e) => setFolder(e.target.value)} />
          <button className="primary" onClick={doImport} disabled={!text.trim()}>اعتبارسنجی و وارد کردن</button>
          <span className="muted" style={{ fontSize: "var(--t-xs)" }}>
            رمزها و UUIDها فقط در انبار امن سیستم‌عامل ذخیره می‌شوند. برای گرفتن خودکار سرورها، لینک اشتراک را در صفحه‌ی «خانه» اضافه کنید.
          </span>
        </div>
        {result && (
          <div style={{ marginTop: 12 }}>
            <p>
              {result.wasBase64 && "متن Base64 تشخیص داده شد. "}
              افزوده: {result.added.toLocaleString("fa-IR")}، تکراری (رد شد): {result.skippedDuplicates.toLocaleString("fa-IR")}
            </p>
            <table>
              <tbody>
                {result.lines.filter((l) => !l.ok || l.issues?.length || l.duplicate).map((l) => (
                  <tr key={l.line}>
                    <td>خط {l.line.toLocaleString("fa-IR")}</td>
                    <td>
                      {l.error && <><span className="dot bad" />{l.error}</>}
                      {l.duplicate && <><span className="dot warn" />تکراری: «{l.name}»</>}
                      {l.issues?.map((i, k) => <div key={k}><span className={`dot ${i.severity === "Error" ? "bad" : "warn"}`} />{i.message}</div>)}
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        )}
      </section>

      {conn && (
        <section className="panel conn-bar" style={{ marginTop: 16 }}>
          <div>
            <span className={`dot ${conn.healthy ? "ok live" : "warn"}`} />متصل به «{conn.name}» — پروکسی محلی <b className="ltr">127.0.0.1:{conn.port}</b>
            <div className="muted conn">همه‌ی ترافیک عبوری برای سرور این پروفایل قابل مشاهده است.</div>
          </div>
          <button onClick={disconnect}>قطع اتصال</button>
        </section>
      )}

      <section className="panel" style={{ marginTop: 16 }}>
        <div className="row" style={{ justifyContent: "space-between" }}>
          <div className="row">
            <button aria-pressed={!activeFolder} onClick={() => setActiveFolder(null)}>همه</button>
            {folders.map((f) => <button key={f} aria-pressed={activeFolder === f} onClick={() => setActiveFolder(f)}>{f}</button>)}
          </div>
          <div className="row">
            <input placeholder="جستجو در نام، میزبان، برچسب" value={q} onChange={(e) => setQ(e.target.value)} />
            <button onClick={() => exportAll(true)}>کپی بدون رمز</button>
            <button onClick={() => exportAll(false)}>کپی کامل</button>
          </div>
        </div>
        <table style={{ marginTop: 10 }}>
          <thead><tr><th /><th>نام</th><th>پروتکل</th><th>مقصد</th><th>برچسب‌ها</th><th /></tr></thead>
          <tbody>
            {shown.map((p) => (
              <tr key={p.fingerprint}>
                <td><button aria-label={p.favorite ? "حذف از علاقه‌مندی" : "افزودن به علاقه‌مندی"} onClick={() => patch(p, { favorite: !p.favorite })}>{p.favorite ? "★" : "☆"}</button></td>
                <td>
                  <input value={p.name} onChange={(e) => setData((d) => d?.map((x) => x.fingerprint === p.fingerprint ? { ...x, name: e.target.value } : x) ?? null)} onBlur={(e) => patch(p, { name: e.target.value })} aria-label="نام پروفایل" />
                </td>
                <td>{p.protocol}{p.standardProxy && <span className="muted"> (استاندارد)</span>}</td>
                <td className="ltr">{p.host}:{p.port}</td>
                <td>
                  <input value={p.tags.join("، ")} placeholder="برچسب" aria-label="برچسب‌ها"
                    onChange={(e) => setData((d) => d?.map((x) => x.fingerprint === p.fingerprint ? { ...x, tags: e.target.value.split(/[،,]\s*/) } : x) ?? null)}
                    onBlur={() => patch(p, { tags: p.tags.map((t) => t.trim()).filter(Boolean) })} />
                </td>
                <td>
                  <div className="row" style={{ flexWrap: "nowrap" }}>
                    {conn?.fingerprint === p.fingerprint
                      ? <button onClick={disconnect}>قطع</button>
                      : <button onClick={() => connect(p)} disabled={!!connecting || !p.hasSecret} title={p.hasSecret ? "اتصال با هسته‌ی sing-box (باید در «ابزارهای پرخطر» تنظیم و تأیید شده باشد)" : "متن کامل این پروفایل در انبار امن نیست"}>
                          {connecting === p.fingerprint ? "در حال اتصال…" : "اتصال"}
                        </button>}
                    <button onClick={async () => { await api.deleteProfile(p.fingerprint); reload(); toast("پروفایل حذف شد"); }}>حذف</button>
                  </div>
                </td>
              </tr>
            ))}
            {data == null && <tr><td colSpan={6} className="muted">بارگذاری…</td></tr>}
            {data != null && !shown.length && <tr><td colSpan={6} className="muted">{data.length ? "پروفایلی با این جستجو پیدا نشد." : "پروفایلی نیست. یک لینک کانفیگ را در کادر بالا بچسبانید."}</td></tr>}
          </tbody>
        </table>
      </section>
    </>
  );
}
