# MasterHttpRelayVPN

سازنده: Amin Mahmoudi (masterking32) · لایسنس: MIT · ریسک: **بحرانی** · سورس: پوشه‌ی `vendor/MasterHttpRelayVPN`

مرورگر ← پروکسی محلی ← سرورهای Google ← اسکریپت Apps Script خودتان ← سایت مقصد.

## خطرها را اول بخوانید

> این ابزار یک **گواهی CA ریشه** روی ویندوز نصب می‌کند و ترافیک HTTPS شما را رمزگشایی می‌کند. هر کس به کلید این گواهی (پوشه‌ی `ca`) دسترسی پیدا کند، می‌تواند هر سایتی، حتی بانک شما، را جعل کند.

- از حساب Google اصلی‌تان استفاده **نکنید**. ممکن است حساب تعلیق شود.
- فایل `config.json`، مقدار `auth_key` و پوشه‌ی `ca` را به هیچ‌کس ندهید.
- پس از استفاده، گواهی را حذف کنید (بخش آخر).

## قدم ۱: رله را روی Google بسازید

1. به [script.google.com](https://script.google.com/) بروید و **New project** را بزنید.
2. کد پیش‌فرض را پاک کنید. محتوای `vendor/MasterHttpRelayVPN/apps_script/Code.gs` را کامل بچسبانید.
3. مقدار `AUTH_KEY` را به یک رمز بلند و تصادفی خودتان عوض کنید.
4. **Deploy** ← **New deployment** ← **Web app**.
5. **Execute as** را **Me** و **Who has access** را **Anyone** بگذارید.
6. **Deployment ID** را کپی کنید.

## قدم ۲: نصب روی دستگاه

```powershell
winget install Python.Python.3.12
```

```powershell
powershell -ExecutionPolicy Bypass -File scripts\build-tool.ps1 master-http-relay
```

اسکریپت یک نسخه‌ی کاری در `%LOCALAPPDATA%\NetDesk\tools\master-http-relay` می‌سازد تا کلیدها وارد مخزن نشوند.

## قدم ۳: تنظیم

```powershell
cd "$env:LOCALAPPDATA\NetDesk\tools\master-http-relay"; .\.venv\Scripts\python.exe setup.py
```

`Deployment ID` و `AUTH_KEY` را وارد کنید. مقدار `AUTH_KEY` باید دقیقاً با مقدار داخل `Code.gs` یکی باشد. در `config.json` مطمئن شوید که برنامه فقط روی `127.0.0.1` گوش می‌دهد.

## قدم ۴: اجرا

1. «ابزارهای پرخطر» ← **MasterHttpRelayVPN** ← **پیدا کردن خودکار**.
2. ثبت مسیر و هش، تأیید هشدارها، **اجرا**.
3. پروکسی مرورگر: HTTP روی `127.0.0.1:8085`، یا SOCKS5 روی `127.0.0.1:1080`.

## حذف گواهی (حتماً)

```powershell
cd "$env:LOCALAPPDATA\NetDesk\tools\master-http-relay"; .\.venv\Scripts\python.exe main.py --uninstall-cert
```

بعد `certmgr.msc` را باز کنید و مطمئن شوید گواهی آن در **Trusted Root Certification Authorities** نمانده است.

## مشکلات رایج

- **`unauthorized`:** مقدار `AUTH_KEY` در دو طرف یکی نیست.
- **خطای گواهی در مرورگر:** راهنمای `vendor/MasterHttpRelayVPN/docs/TROUBLESHOOTING.md` را بخوانید.
