//! netcore — هسته‌ی محلی «Ultimate Network Desktop Suite»
//!
//! همه‌ی قابلیت‌های این crate در «حالت رایگان» کار می‌کنند: هیچ سرور خارجی،
//! حساب کاربری یا VPS لازم ندارند. فقط بسته‌های استاندارد (DNS روی UDP،
//! اتصال TCP) به مقصدی که خود کاربر انتخاب کرده ارسال می‌شود.
//!
//! ماژول‌ها:
//! - [`dns`]      ساخت/تجزیه‌ی پرس‌وجوی DNS و سنجش پاسخ‌دهی resolverها
//! - [`probe`]    سنجش تأخیر TCP، Packet loss، Jitter
//! - [`stats`]    آمار نمونه‌ها
//! - [`config`]   تجزیه، اعتبارسنجی و تشخیص تکراری پروفایل‌ها
//! - [`proxy`]    اعتبارسنجی تنظیمات پروکسی استاندارد (HTTP/SOCKS5)
//! - [`server`]   تجزیه‌ی خروجی /proc برای مانیتورینگ VPS روی SSH
//! - [`rdp`]      ساخت فایل .rdp (مستقل از VPN/پروکسی)
//! - [`report`]   گزارش تشخیصی JSON/Markdown
//! - [`singbox`]  ساخت کانفیگ sing-box از پروفایل (اتصال واقعی با هسته‌ی خارجی)
//! - [`tools`]    فهرست ابزارهای خارجی پرخطر، هشدارها و قواعد تأیید کاربر
//! - [`sha256`]   هش فایل اجرایی ابزارهای خارجی
//! - [`b64`], [`json`] ابزارهای کوچک داخلی بدون وابستگی

pub mod b64;
pub mod config;
pub mod dns;
pub mod json;
pub mod localhttp;
pub mod netenv;
pub mod probe;
pub mod proxy;
pub mod rdp;
pub mod report;
pub mod server;
pub mod serverkit;
pub mod sha256;
pub mod singbox;
pub mod stats;
pub mod tools;
pub mod winreg;
pub mod xray;

/// برچسب منشأ هر قابلیت؛ رابط کاربری با این، «محلی و رایگان» را از
/// «نیازمند سرویس خارجی» جدا نشان می‌دهد (الزام بخش ۱ سند پروژه).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Locality {
    /// کاملاً روی دستگاه کاربر؛ بدون حساب و بدون محدودیت مصرف.
    Local,
    /// به سرور شخص ثالث (مثلاً resolver عمومی) بسته می‌فرستد، اما حساب لازم ندارد.
    PublicEndpoint,
    /// به حساب/سرور شخصی نیاز دارد (VPS، SSH، RDP، سرویس با سهمیه).
    RequiresAccount,
}
