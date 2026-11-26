# FlowDriver

سازنده: NullLatency · لایسنس: MIT · ریسک: پرخطر · سورس: پوشه‌ی `vendor/FlowDriver`

یک تونل SOCKS5 که داده‌ها را از راه فایل‌های Google Drive جابه‌جا می‌کند. دو طرف دارد: **client** روی دستگاه شما و **server** روی سروری که اینترنت آزاد دارد.

## خطرها

> توکن حساب Google در یک فایل ساده روی دیسک ذخیره می‌شود. استفاده‌ی سنگین ممکن است حساب را محدود یا تعلیق کند. از حساب اصلی استفاده نکنید. خود سازنده هم استفاده‌ی عملیاتی را توصیه نمی‌کند.

## قدم ۱: دریافت credentials.json

1. در [console.cloud.google.com](https://console.cloud.google.com/) پروژه بسازید و **Google Drive API** را فعال کنید.
2. **OAuth consent screen** را پر کنید.
3. **Credentials** ← **Create Credentials** ← **OAuth client ID** ← **Desktop App**.
4. فایل JSON را دانلود کنید و نامش را `credentials.json` بگذارید.

> اگر وضعیت برنامه‌ی OAuth روی Testing باشد، توکن هر ۷ روز منقضی می‌شود.

## قدم ۲: ساخت

```powershell
winget install GoLang.Go
```

```powershell
powershell -ExecutionPolicy Bypass -File scripts\build-tool.ps1 flowdriver
```

خروجی: `flowdriver-client.exe` و `flowdriver-server.exe` در `%LOCALAPPDATA%\NetDesk\bin`.

## قدم ۳: تنظیم

یک پوشه **بیرون از مخزن** بسازید. `vendor/FlowDriver/client_config.json.example` را در آن کپی کنید و نامش را `client_config.json` بگذارید. فایل `credentials.json` را هم همان‌جا بگذارید.

- `listen_addr` را روی `127.0.0.1:1080` نگه دارید.
- `refresh_rate_ms` را ۲۰۰ یا بیشتر بگذارید تا سهمیه‌ی API زود تمام نشود.
- `google_folder_id` در هر دو طرف باید یکی باشد.

روی سرور، با `server_config.json` اجرا کنید:

```bash
./flowdriver-server -c server_config.json -gc credentials.json
```

## قدم ۴: اجرا

1. «ابزارهای پرخطر» ← **FlowDriver** ← **پیدا کردن خودکار**.
2. آرگومان‌ها: `-c client_config.json -gc credentials.json`
3. پوشه‌ی کاری: همان پوشه‌ای که ساختید.
4. ثبت، تأیید، **اجرا**. برنامه‌ها را روی SOCKS5 `127.0.0.1:1080` بگذارید.

بار اول ممکن است برای ورود به Google نیاز باشد برنامه را یک بار در ترمینال اجرا کنید.
