# NetDesk — Ultimate Network Desktop Suite

برنامه‌ی دسکتاپ متن‌باز برای عیب‌یابی شبکه، مدیریت کانفیگ‌ها و مدیریت سرورهای شخصی.
Tauri 2 + Rust + React/TypeScript. رابط کاملاً فارسی و راست‌به‌چپ.

> این برنامه به‌تنهایی اتصال اینترنتی «رایگان» یا «نامحدود» نمی‌سازد. ابزارهای حالت رایگان شبکه‌ی فعلی شما را می‌سنجند؛ هر اتصال پروکسی/VPN به سروری نیاز دارد که خودتان یا ارائه‌دهنده‌تان فراهم می‌کند.

## شروع سریع

آموزش کامل: **[از صفر تا اتصال](docs/tutorials/start.fa.md)**

```powershell
git clone https://github.com/yourcreatorox7-lgtm/netdesk.git
cd netdesk
powershell -ExecutionPolicy Bypass -File scripts\build-tool.ps1 sing-box
npm install
npm run tauri dev
```

## قابلیت‌ها

**اتصال هوشمند (یک دکمه):** سنجش هم‌زمان همه‌ی سرورها، تست واقعی از داخل تونل، امتحان TLS fragment در صورت فیلتر، انتخاب سریع‌ترین، تنظیم خودکار پروکسی ویندوز و جابه‌جایی خودکار هنگام قطعی.

**اشتراک‌ها:** افزودن لینک https و به‌روزرسانی خودکار هنگام باز شدن برنامه.

**دستیار هوشمند محلی:** با Ollama روی دستگاه خودتان؛ با «خلبان خودکار» کارهای مجاز را خودش انجام می‌دهد. داده‌ی حساس به مدل داده نمی‌شود.

**مدیر کانفیگ:** vless، vmess، trojan، ss، hysteria2، tuic، socks5، http؛ تشخیص Base64؛ هشدار امنیتی؛ تشخیص تکراری؛ پوشه و برچسب؛ خروجی کامل یا بدون رمز.

**ابزارهای شبکه:** پایش تأخیر، Jitter و Packet loss؛ مقایسه‌ی resolverهای DNS؛ تست TCP/UDP و پروکسی؛ گزارش Markdown/JSON؛ مدیریت VPS و RDP.

**ابزارهای پرخطر (اختیاری):** سورس آن‌ها در `vendor/` است و با یک اسکریپت ساخته می‌شود. هر کدام آموزش فارسی، هشدار جداگانه، هش SHA-256 سنجاق‌شده و تأیید چندمرحله‌ای دارد.

## اجرا

```bash
# فقط هسته (آفلاین)
cargo test
cargo run -p netcore --bin netcore-cli -- quick
cargo run -p netcore --bin netcore-cli -- import subscription.txt

# برنامه‌ی دسکتاپ (نیاز به Node 20+، Rust 1.77+ و پیش‌نیازهای Tauri سیستم‌عامل)
npm install
npm run tauri dev
npm run tauri build      # نصب‌کننده‌ی Windows/macOS/Linux

# ساخت دوباره‌ی پیش‌نمایش مرورگر
node preview/build.mjs
```

پیش‌نیازهای Tauri: https://v2.tauri.app/start/prerequisites/

## آموزش‌ها

همین آموزش‌ها داخل برنامه هم هستند.

- [از صفر تا اتصال](docs/tutorials/start.fa.md)
- [sing-box (هسته‌ی اتصال)](docs/tutorials/sing-box.fa.md)
- [دستیار هوش مصنوعی](docs/tutorials/ai.fa.md)
- [Xray-core](docs/tutorials/xray.fa.md)
- ابزارهای پرخطر: [MasterHttpRelayVPN](docs/tutorials/master-http-relay.fa.md) · [mhr-cfw](docs/tutorials/mhr-cfw.fa.md) · [FlowDriver](docs/tutorials/flowdriver.fa.md) · [SNI-Spoofing](docs/tutorials/sni-spoofing-py.fa.md) · [sni-spoofing-rust](docs/tutorials/sni-spoofing-rust.fa.md) · [SNI-Spoofing-Go](docs/tutorials/sni-spoofing-go.fa.md)

## مستندات فنی
- [معماری و تصمیم‌های امنیتی](docs/ARCHITECTURE.fa.md)
- [گزارش بررسی اولیه‌ی مخازن](docs/RESEARCH.fa.md)

## لایسنس
کد خود NetDesk تحت MIT است. **پوشه‌ی `vendor/` شامل این لایسنس نیست:** هر زیرپوشه کار سازنده‌ی خودش است و لایسنس خودش را دارد (GPL-3.0، MPL-2.0 یا MIT). [فهرست سازنده‌ها و لایسنس‌ها](vendor/README.md)
