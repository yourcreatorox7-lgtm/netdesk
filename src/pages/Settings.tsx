import { useState } from "react";
import { api } from "../lib/api";
import { PageHead, toast } from "../components/ui";

export default function Settings({ theme, setTheme }: { theme: string; setTheme: (t: "dark" | "light" | "system") => void }) {
  const [restore, setRestore] = useState("");
  return (
    <>
      <PageHead title="تنظیمات" />
      <div className="grid g2">
        <section className="panel">
          <h2>ظاهر</h2>
          <div className="row" role="radiogroup" aria-label="تم">
            {(["system", "dark", "light"] as const).map((t) => (
              <button key={t} role="radio" aria-checked={theme === t} className={theme === t ? "primary" : ""} onClick={() => setTheme(t)}>
                {{ system: "مطابق سیستم", dark: "تیره", light: "روشن" }[t]}
              </button>
            ))}
          </div>
          <h2 style={{ marginTop: 20 }}>میان‌برهای صفحه‌کلید</h2>
          <table><tbody>
            {[["Ctrl+1 تا Ctrl+6", "رفتن به بخش‌ها"], ["Ctrl+R", "اجرای عیب‌یابی کامل"], ["Ctrl+I", "وارد کردن پروفایل"], ["Ctrl+Shift+L", "تغییر تم"]].map(([k, v]) => (
              <tr key={k}><td className="ltr">{k}</td><td>{v}</td></tr>
            ))}
          </tbody></table>
        </section>
        <section className="panel">
          <h2>پشتیبان و بازیابی</h2>
          <p className="muted">پشتیبان فقط فراداده‌ی پروفایل‌ها و سرورها را دارد. رمزها در انبار امن سیستم‌عامل می‌مانند و به فایل منتقل نمی‌شوند.</p>
          <button onClick={async () => { await navigator.clipboard.writeText(await api.backupExport()); toast("پشتیبان در کلیپ‌بورد کپی شد"); }}>کپی فایل پشتیبان</button>
          <textarea style={{ marginTop: 12, minHeight: 90 }} placeholder="محتوای پشتیبان را برای بازیابی بچسبانید" value={restore} onChange={(e) => setRestore(e.target.value)} />
          <button disabled={!restore} onClick={async () => {
            try { const [p, s] = await api.backupImport(restore); toast(`بازیابی شد: ${p.toLocaleString("fa-IR")} پروفایل، ${s.toLocaleString("fa-IR")} سرور`); setRestore(""); }
            catch (e) { toast(String(e)); }
          }}>بازیابی</button>
        </section>
      </div>
    </>
  );
}
