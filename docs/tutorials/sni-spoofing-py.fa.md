# SNI-Spoofing (نسخه‌ی Python)

سازنده‌ی روش: patterniha · لایسنس: GPL-3.0 · ریسک: **بحرانی** · سورس: پوشه‌ی `vendor/SNI-Spoofing`

یک رله‌ی محلی است. پیش از دست‌دهی واقعی، یک ClientHello جعلی با نام سایت دیگری می‌فرستد تا فیلتر فریب بخورد. فقط برای کانفیگ‌هایی کار می‌کند که پشت CDN (مثل Cloudflare) هستند.

## خطرها

> باید با دسترسی **Administrator** اجرا شود و با درایور WinDivert بسته‌های شبکه را در هسته‌ی ویندوز دست‌کاری می‌کند.

## نصب

```powershell
winget install Python.Python.3.12
```

```powershell
powershell -ExecutionPolicy Bypass -File scripts\build-tool.ps1 sni-spoofing-py
```

بسته‌ی `pydivert` هنگام نصب، درایور WinDivert را همراه خود می‌آورد. آنتی‌ویروس ممکن است درباره‌ی آن هشدار بدهد.

## تنظیم

فایل `%LOCALAPPDATA%\NetDesk\tools\sni-spoofing-py\config.json` را باز کنید:

```json
{
  "LISTEN_HOST": "127.0.0.1",
  "LISTEN_PORT": 40443,
  "CONNECT_IP": "IP سرور CDN شما",
  "CONNECT_PORT": 443,
  "FAKE_SNI": "auth.vercel.com"
}
```

> مقدار پیش‌فرض `LISTEN_HOST` در سورس `0.0.0.0` است. **حتماً** آن را به `127.0.0.1` تغییر دهید.

## اتصال کانفیگ

در کانفیگ vless یا trojan خود فقط **آدرس** را `127.0.0.1` و **پورت** را `40443` کنید. بقیه (UUID، SNI، host، path) بدون تغییر بماند. بعد آن را در «پروفایل‌ها» وارد کنید.

## اجرا

1. «ابزارهای پرخطر» ← **SNI-Spoofing (Python)** ← **پیدا کردن خودکار** ← ثبت ← تأیید.
2. **اجرا** را بزنید. ویندوز پنجره‌ی UAC نشان می‌دهد و خودتان باید تأیید کنید.
3. در «پروفایل‌ها» روی پروفایل `127.0.0.1:40443` دکمه‌ی «اتصال» را بزنید.

برای توقف، پنجره‌ی خود ابزار را ببندید. NetDesk به پردازه‌ای که دسترسی مدیر دارد دسترسی ندارد.
