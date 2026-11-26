import { useState } from "react";
import { api, type Metrics, type Server } from "../lib/api";
import { uptime } from "../lib/format";
import { Loc, Meter, PageHead, toast, useAsync } from "../components/ui";

const empty: Server = { id: "", name: "", host: "", sshPort: 22, sshUser: "root", sshKeyPath: null, rdpPort: null, rdpUser: null, tags: [], provider: null, notes: null };

/** VPS و RDP — جدا از هر قابلیت VPN/پروکسی */
export default function Servers() {
  const { data, run: reload } = useAsync(api.listServers);
  const [metrics, setMetrics] = useState<Record<string, Metrics | string>>({});
  const [form, setForm] = useState<Server | null>(null);

  const refresh = async (s: Server) => {
    setMetrics((m) => ({ ...m, [s.id]: "…" }));
    try {
      const r = await api.serverMetrics(s.id);
      setMetrics((m) => ({ ...m, [s.id]: r }));
    } catch (e) { setMetrics((m) => ({ ...m, [s.id]: String(e) })); }
  };

  const saveForm = async () => {
    if (!form) return;
    try { await api.saveServer(form); setForm(null); reload(); toast("سرور ذخیره شد"); } catch (e) { toast(String(e)); }
  };

  return (
    <>
      <PageHead title="سرورها" lead="مانیتورینگ با SSH و کلید شخصی شما؛ Remote Desktop با کلاینت بومی سیستم.">
        <Loc kind="account" />
        <button className="primary" onClick={() => setForm({ ...empty })}>افزودن سرور</button>
      </PageHead>

      {form && (
        <section className="panel" style={{ marginBottom: 16 }}>
          <h2>{form.id ? "ویرایش سرور" : "سرور جدید"}</h2>
          <div className="grid g3">
            <label>نام <input value={form.name} onChange={(e) => setForm({ ...form, name: e.target.value })} /></label>
            <label>میزبان / IP <input className="ltr" value={form.host} onChange={(e) => setForm({ ...form, host: e.target.value })} /></label>
            <label>ارائه‌دهنده <input value={form.provider ?? ""} onChange={(e) => setForm({ ...form, provider: e.target.value || null })} /></label>
            <label>کاربر SSH <input className="ltr" value={form.sshUser} onChange={(e) => setForm({ ...form, sshUser: e.target.value })} /></label>
            <label>پورت SSH <input type="number" value={form.sshPort} onChange={(e) => setForm({ ...form, sshPort: +e.target.value })} /></label>
            <label>مسیر کلید خصوصی <input className="ltr" placeholder="~/.ssh/id_ed25519" value={form.sshKeyPath ?? ""} onChange={(e) => setForm({ ...form, sshKeyPath: e.target.value || null })} /></label>
            <label>پورت RDP (اختیاری) <input type="number" value={form.rdpPort ?? ""} onChange={(e) => setForm({ ...form, rdpPort: e.target.value ? +e.target.value : null })} /></label>
            <label>کاربر RDP <input className="ltr" value={form.rdpUser ?? ""} onChange={(e) => setForm({ ...form, rdpUser: e.target.value || null })} /></label>
          </div>
          <p className="muted" style={{ fontSize: "var(--t-xs)" }}>
            برنامه رمز SSH نمی‌گیرد و فقط با کلید وصل می‌شود. رمز RDP را کلاینت بومی ویندوز/مک هنگام اتصال می‌پرسد.
          </p>
          <div className="row"><button className="primary" onClick={saveForm}>ذخیره‌ی سرور</button><button onClick={() => setForm(null)}>انصراف</button></div>
        </section>
      )}

      <div className="grid g2">
        {(data ?? []).map((s) => {
          const m = metrics[s.id];
          const mm = typeof m === "object" ? m : null;
          return (
            <article key={s.id} className="server">
              <header>
                <div>
                  <h3>{s.name || s.host}</h3>
                  <span className="muted ltr">{s.sshUser}@{s.host}:{s.sshPort}</span>
                  {s.provider && <span className="muted">، {s.provider}</span>}
                </div>
                <span>{mm ? <><span className={`dot ${mm.health === "Ok" ? "ok" : mm.health === "Warning" ? "warn" : "bad"}`} />{mm.health === "Ok" ? "سالم" : mm.health === "Warning" ? "پرفشار" : "بحرانی"}</> : <span className="muted">وضعیت نامعلوم</span>}</span>
              </header>
              <Meter label="CPU" value={mm?.cpuPct ?? null} />
              <Meter label="RAM" value={mm?.memUsedPct ?? null} />
              <Meter label="دیسک" value={mm?.diskUsedPct ?? null} />
              {typeof m === "string" && m !== "…" && <p style={{ color: "var(--bad)", fontSize: "var(--t-sm)", margin: 0 }}>{m}</p>}
              {mm && <span className="muted" style={{ fontSize: "var(--t-xs)" }}>روشن از {uptime(mm.uptimeS)} پیش، {mm.cpus?.toLocaleString("fa-IR")} هسته</span>}
              <div className="row">
                <button onClick={() => refresh(s)} disabled={m === "…"}>{m === "…" ? "در حال اتصال…" : "به‌روزرسانی وضعیت"}</button>
                {s.rdpPort && <button onClick={() => api.rdpLaunch(s.id).then(() => toast("کلاینت Remote Desktop باز شد")).catch((e) => toast(String(e)))}>اتصال RDP</button>}
                <button onClick={() => setForm(s)}>ویرایش</button>
                <button onClick={async () => { await api.deleteServer(s.id); reload(); }}>حذف</button>
              </div>
            </article>
          );
        })}
      </div>
      {data && !data.length && !form && <section className="panel"><p>هنوز سروری اضافه نکرده‌اید. اگر VPS ندارید، بخش‌های «وضعیت شبکه» و «عیب‌یابی» بدون آن کامل کار می‌کنند.</p></section>}
    </>
  );
}
