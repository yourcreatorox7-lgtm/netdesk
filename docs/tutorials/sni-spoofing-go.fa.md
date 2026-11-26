# SNI-Spoofing-Go

سازنده: aleskxyz (بر پایه‌ی روش patterniha) · لایسنس: GPL-3.0 · ریسک: **بحرانی** · سورس: پوشه‌ی `vendor/SNI-Spoofing-Go`

## خطرها

> باید **Administrator** باشد و با WinDivert بسته‌ها را دست‌کاری می‌کند.

## ساخت

```powershell
winget install GoLang.Go
```

```powershell
powershell -ExecutionPolicy Bypass -File scripts\build-tool.ps1 sni-spoofing-go
```

خروجی: `%LOCALAPPDATA%\NetDesk\bin\sni-spoofing-go.exe` (نسخه‌ی خط فرمان).

## فایل‌های WinDivert

فایل‌های `WinDivert.dll` و `WinDivert64.sys` را از [سایت رسمی WinDivert](https://reqrypt.org/windivert.html) بگیرید و کنار فایل exe بگذارید.

## اجرا در NetDesk

1. «ابزارهای پرخطر» ← **SNI-Spoofing-Go** ← **پیدا کردن خودکار**.
2. آرگومان‌ها (IP سرور خودتان را بگذارید):

```bash
-listen 127.0.0.1:40443 -connect 104.19.229.21:443 -fake-sni hcaptcha.com -utls firefox
```

3. ثبت، تأیید، **اجرا**، و پنجره‌ی UAC را تأیید کنید.
4. در کانفیگ کلاینت فقط آدرس را `127.0.0.1` و پورت را `40443` کنید.

## تنظیم‌های مفید

- `-enable-fragment`: تکه‌تکه کردن ClientHello واقعی.
- `-fake-repeat 2`: فرستادن دو بسته‌ی جعلی.
- `-test`: آزمایش خودکار روش‌ها.
- `-h`: فهرست همه‌ی گزینه‌ها.
