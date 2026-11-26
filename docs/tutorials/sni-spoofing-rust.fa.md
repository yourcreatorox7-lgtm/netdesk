# sni-spoofing-rust

سازنده: therealaleph (بر پایه‌ی روش patterniha) · لایسنس: MIT · ریسک: **بحرانی** · سورس: پوشه‌ی `vendor/sni-spoofing-rust`

> برنامه‌های آماده‌ی پوشه‌ی `releases` پروژه‌ی اصلی **عمداً** در این مخزن نیستند، چون بررسی نشده‌اند. خودتان از سورس بسازید.

## خطرها

> باید **Administrator** باشد و درایور WinDivert را در هسته‌ی ویندوز بارگذاری می‌کند.

## ساخت

```powershell
winget install Rustlang.Rustup
```

```powershell
powershell -ExecutionPolicy Bypass -File scripts\build-tool.ps1 sni-spoofing-rust
```

خروجی: `%LOCALAPPDATA%\NetDesk\bin\sni-spoof-rs.exe`. ساخت به Visual Studio Build Tools (بخش C++) نیاز دارد.

## فایل‌های WinDivert

فایل‌های `WinDivert.dll` و `WinDivert64.sys` را **فقط از سایت رسمی** [reqrypt.org/windivert.html](https://reqrypt.org/windivert.html) بگیرید و کنار فایل exe بگذارید.

## کانفیگ

در پوشه‌ای بیرون از مخزن، `config.json` بسازید:

```json
{
  "graceful_shutdown_sec": 0,
  "listeners": [
    {
      "listen": "127.0.0.1:40443",
      "connect": "CLOUDFLARE_IP:443",
      "fake_sni": "security.vercel.com",
      "conn_timeout_sec": 5,
      "handshake_timeout_sec": 2,
      "keepalive_time_sec": 11,
      "keepalive_interval_sec": 2
    }
  ]
}
```

برای پیدا کردن IP سرور خودتان:

```powershell
nslookup myserver.example.com
```

> نمونه‌ی داخل سورس روی `0.0.0.0` گوش می‌دهد. `127.0.0.1` را نگه دارید.

## اتصال و اجرا

1. در کانفیگ کلاینت فقط آدرس را `127.0.0.1` و پورت را `40443` کنید.
2. «ابزارهای پرخطر» ← **sni-spoofing-rust** ← **پیدا کردن خودکار**.
3. آرگومان: مسیر کامل `config.json`. پوشه‌ی کاری: همان پوشه.
4. ثبت، تأیید، **اجرا**، و پنجره‌ی UAC را تأیید کنید.

> دکمه‌ی «دانلود Xray» داخل رابط گرافیکی خود این پروژه را استفاده نکنید.
