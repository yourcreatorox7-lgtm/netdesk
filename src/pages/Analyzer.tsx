import { useState } from "react";
import { save } from "@tauri-apps/plugin-dialog";
import { writeTextFile } from "@tauri-apps/plugin-fs";
import { api, isMock } from "../lib/api";
import { Loc, PageHead, toast } from "../components/ui";

/** مرکز تحلیل: گزارش کامل یک‌کلیکی و خروجی Markdown/JSON */
export default function Analyzer() {
  const [md, setMd] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);

  const run = async () => {
    setBusy(true);
    try { setMd(await api.quickReport("md")); } catch (e) { toast(String(e)); } finally { setBusy(false); }
  };

  const exportAs = async (fmt: "md" | "json") => {
    const text = fmt === "md" ? md ?? "" : await api.quickReport("json");
    if (isMock) { navigator.clipboard?.writeText(text); toast("گزارش در کلیپ‌بورد کپی شد"); return; }
    const path = await save({ defaultPath: `netdesk-report.${fmt}`, filters: [{ name: fmt.toUpperCase(), extensions: [fmt] }] });
    if (path) { await writeTextFile(path, text); toast("گزارش ذخیره شد"); }
  };

  return (
    <>
      <PageHead title="تحلیلگر شبکه" lead="ده resolver عمومی و سه مقصد TCP را همزمان می‌سنجد و نتیجه را به زبان ساده خلاصه می‌کند.">
        <Loc kind="public" />
        <button className="primary" onClick={run} disabled={busy}>{busy ? "در حال اجرا (حدود ۱۰ ثانیه)…" : "اجرای عیب‌یابی کامل"}</button>
      </PageHead>
      {!md ? (
        <section className="panel">
          <p>هنوز گزارشی ساخته نشده. «اجرای عیب‌یابی کامل» را بزنید؛ نتیجه را می‌توانید برای پشتیبانی یا ارائه‌دهنده‌ی اینترنت بفرستید.</p>
          <p className="muted">گزارش شامل آدرس IP resolverها و زمان‌های پاسخ است، نه اطلاعات حساب یا کانفیگ‌های شما.</p>
        </section>
      ) : (
        <section className="panel">
          <div className="row" style={{ justifyContent: "flex-end" }}>
            <button onClick={() => exportAs("md")}>ذخیره به Markdown</button>
            <button onClick={() => exportAs("json")}>ذخیره به JSON</button>
          </div>
          <pre style={{ whiteSpace: "pre-wrap", fontFamily: "inherit" }}>{md}</pre>
        </section>
      )}
    </>
  );
}
