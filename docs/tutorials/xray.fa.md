# Xray-core

سازنده: تیم Project X (XTLS) · لایسنس: MPL-2.0 · ریسک: متوسط · سورس: پوشه‌ی `vendor/Xray-core`

هسته‌ی جایگزین برای کسانی که کانفیگ Xray را خودشان می‌نویسند. «اتصال هوشمند» از sing-box استفاده می‌کند، نه Xray.

## ساخت از سورس

```powershell
winget install GoLang.Go
```

```powershell
powershell -ExecutionPolicy Bypass -File scripts\build-tool.ps1 xray
```

خروجی: `%LOCALAPPDATA%\NetDesk\bin\xray.exe`

## کانفیگ

فایل `config.json` خود را در پوشه‌ای **بیرون از مخزن** بسازید تا رمزهایتان در گیت نرود. نمونه‌ی ورودی امن (فقط همین دستگاه):

```json
{
  "inbounds": [
    { "listen": "127.0.0.1", "port": 10810, "protocol": "socks" }
  ],
  "outbounds": [ { "protocol": "freedom" } ]
}
```

بخش `outbounds` را با مشخصات سرور خودتان جایگزین کنید.

> `listen` را روی `0.0.0.0` نگذارید. در آن صورت همه‌ی دستگاه‌های شبکه می‌توانند از پروکسی شما استفاده کنند.

## اجرا در NetDesk

1. «ابزارهای پرخطر» ← کارت **Xray-core** ← **پیدا کردن خودکار**.
2. آرگومان‌ها: `run -c C:\path\to\config.json`
3. ثبت مسیر و هش، تأیید هشدارها، و **اجرا**.
4. برنامه‌ی دلخواه را روی `127.0.0.1:10810` (SOCKS5) تنظیم کنید.
