import { useState } from "react";
import { api, type ServerKit } from "../lib/api";
import { PageHead, toast } from "../components/ui";
import { copy } from "../components/markdown";
import type { Route } from "../App";

// فقط در حافظه؛ با جابه‌جایی بین صفحه‌ها کیت ساخته‌شده گم نمی‌شود
let savedKit: ServerKit | null = null;

/** ساخت سرور اتصال روی سرور ویندوز خود کاربر */
export default function MyServer({ go, tutorial }: { go: (r: Route) => void; tutorial: (id: string) => void }) {
  const [host, setHost] = useState("");
  const [port, setPort] = useState("443");
  const [sni, setSni] = useState("www.microsoft.com");
  const [kit, setKitState] = useState<ServerKit | null>(savedKit);
  const [busy, setBusy] = useState<"create" | "connect" | null>(null);
  const [error, setError] = useState("");
  const [connected, setConnected] = useState("");
  const setKit = (k: ServerKit | null) => { savedKit = k; setKitState(k); };

  const address = host.trim();
  // «IP:پورت» یعنی پورت RDP همراه آدرس چسبانده شده
  const rdpPort = /^[^:]+:(\d+)$/.exec(address)?.[1];

  const create = async () => {
    setBusy("create"); setError(""); setConnected("");
    try { setKit(await api.serverKitCreate(address, Number(port), sni.trim())); }
    catch (e) { setError(String(e)); }
    finally { setBusy(null); }
  };

  const connect = async () => {
    if (!kit) return;
    setBusy("connect"); setError(""); setConnected("");
    try {
      const c = await api.singboxConnect(kit.fingerprint);
      setConnected(c.latencyMs == null ? "وصل شد." : `وصل شد. تأخیر تونل: ${Math.round(c.latencyMs)} ms`);
    } catch (e) { setError(String(e)); }
    finally { setBusy(null); }
  };

  return (
    <>
      <PageHead title="سرور خودم" lead="سرور ویندوزی که با Remote Desktop دارید را سرور اتصال خودتان کنید. برنامه رمز سرور را نمی‌گیرد.">
        <button onClick={() => tutorial("my-server")}>آموزش کامل</button>
        <button onClick={() => go("tools")}>تنظیم هسته</button>
      </PageHead>

      <section className="panel">
        <h2>۱. ساخت کیت</h2>
        <div className="grid g3">
          <label>IP سرور <input className="ltr" placeholder="203.0.113.10" value={host} onChange={(e) => setHost(e.target.value)} /></label>
          <label>پورت <input className="ltr" inputMode="numeric" value={port} onChange={(e) => setPort(e.target.value.replace(/\D/g, ""))} /></label>
          <label>سایت پوششی (SNI) <input className="ltr" value={sni} onChange={(e) => setSni(e.target.value)} /></label>
        </div>
        {rdpPort && <p className="muted">پورت را از آدرس حذف کنید. اگر RDP شما پورتی غیر از ۳۳۸۹ دارد ({rdpPort})، شاید IP مشترک باشد و سرور از بیرون در دسترس نباشد. آموزش را ببینید.</p>}
        <p className="muted">اگر دو مقدار آخر را نمی‌شناسید، تغییرشان ندهید. قوانین ارائه‌دهنده‌ی سرور درباره‌ی VPN را هم بررسی کنید.</p>
        <button className="primary" disabled={!address || !port || !sni.trim() || !!busy} onClick={create}>{busy === "create" ? "در حال ساخت…" : "ساخت کیت"}</button>
      </section>

      {kit && (
        <>
          <section className="panel">
            <h2>۲. اجرا روی سرور</h2>
            <p>کیت روی دسکتاپ ساخته شد و پروفایل «سرور من» هم اضافه شد.</p>
            <p className="ltr muted">{kit.dir}</p>
            <ol>
              <li>با Remote Desktop به سرور وصل شوید.</li>
              <li>کل پوشه‌ی کیت را کپی کنید و روی دسکتاپ سرور بچسبانید.</li>
              <li>روی سرور، <b className="ltr">INSTALL.cmd</b> را دوبار کلیک کنید و <b>Yes</b> را بزنید.</li>
              <li>صبر کنید تا <b className="ltr">DONE</b> را ببینید.</li>
            </ol>
            <div className="row">
              <button className="primary" onClick={() => api.serverKitOpen(kit.dir).catch((e) => toast(String(e)))}>باز کردن پوشه‌ی کیت</button>
              <button onClick={() => copy(kit.dir, "مسیر کپی شد")}>کپی مسیر</button>
            </div>
          </section>

          <section className="panel">
            <h2>۳. تست و اتصال</h2>
            <button className="primary" disabled={!!busy} onClick={connect}>{busy === "connect" ? "در حال اتصال…" : "تست و اتصال"}</button>
            {connected && <p role="status"><span className="dot ok" /> {connected} حالا پوشه‌ی کیت را از هر دو دستگاه پاک کنید.</p>}
          </section>
        </>
      )}

      {error && (
        <section className="panel" role="alert">
          <p style={{ color: "var(--bad)" }}>{error}</p>
          <p className="muted">اگر اتصال به سرور ناموفق بود، اول پورت TCP {kit?.port ?? port} را در فایروال پنل ارائه‌دهنده باز کنید.</p>
        </section>
      )}
    </>
  );
}
