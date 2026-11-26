# mhr-cfw

سازنده: denuitt1 · لایسنس: MIT · ریسک: **بحرانی** · سورس: پوشه‌ی `vendor/mhr-cfw`

نسخه‌ای از MasterHttpRelay که خروجی آن از Cloudflare Worker است.

## خطرها

> این ابزار هم **گواهی CA ریشه** نصب می‌کند و ترافیک HTTPS را رمزگشایی می‌کند.

- از حساب‌های اصلی Google و Cloudflare استفاده نکنید.
- اگر تنظیمی به اسم `UPSTREAM_FAIL_MODE` می‌بینید، آن را روی `closed` بگذارید. در حالت `open` ممکن است رله‌ی باز ساخته شود که دیگران از آن سوءاستفاده کنند.

## قدم ۱: Cloudflare Worker

1. وارد [dash.cloudflare.com](https://dash.cloudflare.com/) شوید و به **Compute** ← **Workers & Pages** بروید.
2. **Create Application** ← **Start with Hello World** ← **Deploy**.
3. **Edit code** را بزنید و کد پیش‌فرض را پاک کنید.
4. محتوای `vendor/mhr-cfw/deploy/cloudflare-worker/worker.js` را در ویرایشگر Worker بچسبانید.
5. مقدار `WORKER_URL` را به نشانی Worker خودتان تغییر دهید و **Deploy** بزنید.

## قدم ۲: رله‌ی Google

1. در [script.google.com](https://script.google.com/) پروژه‌ی تازه بسازید.
2. محتوای `vendor/mhr-cfw/deploy/gas/Code.gs` را بچسبانید.
3. `AUTH_KEY` (رمز بلند خودتان) و `WORKER_URL` (با `https://`) را تنظیم کنید.
4. **Deploy** ← **Web app**، با **Execute as: Me** و **Who has access: Anyone**.
5. **Deployment ID** را کپی کنید.

## قدم ۳: نصب و تنظیم

```powershell
powershell -ExecutionPolicy Bypass -File scripts\build-tool.ps1 mhr-cfw
```

```powershell
cd "$env:LOCALAPPDATA\NetDesk\tools\mhr-cfw"; .\.venv\Scripts\python.exe setup.py
```

## قدم ۴: اجرا

1. «ابزارهای پرخطر» ← **mhr-cfw** ← **پیدا کردن خودکار** ← ثبت ← تأیید ← **اجرا**.
2. پروکسی HTTP: `127.0.0.1:8085`
3. در `ipleak.net` بررسی کنید که IP شما عوض شده باشد.

## حذف گواهی (حتماً)

```powershell
cd "$env:LOCALAPPDATA\NetDesk\tools\mhr-cfw"; .\.venv\Scripts\python.exe main.py --uninstall-cert
```

بعد `certmgr.msc` را هم بررسی کنید.
