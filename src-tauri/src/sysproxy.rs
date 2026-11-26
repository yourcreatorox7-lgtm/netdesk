//! تنظیم خودکار پروکسی سیستم ویندوز (فقط کاربر فعلی، بدون دسترسی مدیر).
//!
//! پیش از اولین تغییر، تنظیم قبلی در `sysproxy-backup.json` ذخیره می‌شود و
//! هنگام قطع اتصال، بستن برنامه یا (پس از کرش) اجرای بعدی برگردانده می‌شود.
//! اگر کاربر یا برنامه‌ی دیگری در این فاصله پروکسی را عوض کرده باشد، دست نمی‌زنیم.

use crate::store;
use netcore::winreg;
use serde::{Deserialize, Serialize};
use tauri::AppHandle;

const FILE: &str = "sysproxy-backup.json";

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct Backup {
    /// مقدارهایی که پیش از NetDesk بوده‌اند؛ None یعنی وجود نداشته
    enable: Option<String>,
    server: Option<String>,
    bypass: Option<String>,
    /// آدرسی که NetDesk تنظیم کرده
    applied: String,
}

#[cfg(target_os = "windows")]
mod os {
    use std::os::windows::process::CommandExt;
    use std::process::Command;

    pub const KEY: &str = r"HKCU\Software\Microsoft\Windows\CurrentVersion\Internet Settings";
    const BYPASS: &str = "localhost;127.*;10.*;192.168.*;172.16.*;172.17.*;172.18.*;172.19.*;172.20.*;172.21.*;172.22.*;172.23.*;172.24.*;172.25.*;172.26.*;172.27.*;172.28.*;172.29.*;172.30.*;172.31.*;<local>";

    fn run(program: &str, args: &[&str]) -> Result<std::process::Output, String> {
        Command::new(program).args(args).creation_flags(0x0800_0000).output().map_err(|e| e.to_string())
    }

    pub fn query() -> Result<String, String> {
        Ok(String::from_utf8_lossy(&run("reg", &["query", KEY])?.stdout).into_owned())
    }

    pub fn set(name: &str, kind: &str, value: Option<&str>) -> Result<(), String> {
        let out = match value {
            Some(v) => run("reg", &["add", KEY, "/v", name, "/t", kind, "/d", v, "/f"])?,
            None => run("reg", &["delete", KEY, "/v", name, "/f"])?,
        };
        // حذف مقداری که وجود ندارد خطا نیست
        if value.is_some() && !out.status.success() {
            return Err(format!("تغییر رجیستری ناموفق بود: {}", String::from_utf8_lossy(&out.stderr)));
        }
        Ok(())
    }

    pub fn bypass() -> &'static str {
        BYPASS
    }

    /// به برنامه‌های باز خبر می‌دهد که تنظیم پروکسی عوض شده (بدون راه‌اندازی دوباره)
    pub fn notify() {
        const PS: &str = "$s='[DllImport(\"wininet.dll\")] public static extern bool InternetSetOption(IntPtr h, int o, IntPtr b, int l);';\
$t=Add-Type -MemberDefinition $s -Name W -Namespace N -PassThru;\
$null=$t::InternetSetOption([IntPtr]::Zero,39,[IntPtr]::Zero,0);$null=$t::InternetSetOption([IntPtr]::Zero,37,[IntPtr]::Zero,0)";
        let _ = run("powershell", &["-NoProfile", "-NonInteractive", "-Command", PS]);
    }
}

#[cfg(target_os = "windows")]
pub fn enable(app: &AppHandle, port: u16) -> Result<(), String> {
    let server = format!("127.0.0.1:{port}");
    let now = os::query()?;
    let old: Backup = store::load(app, FILE);
    let backup = if old.applied.is_empty() {
        Backup {
            enable: winreg::value(&now, "ProxyEnable"),
            server: winreg::value(&now, "ProxyServer"),
            bypass: winreg::value(&now, "ProxyOverride"),
            applied: server.clone(),
        }
    } else {
        Backup { applied: server.clone(), ..old }
    };
    // اول پشتیبان روی دیسک، بعد تغییر
    store::save(app, FILE, &backup)?;
    os::set("ProxyServer", "REG_SZ", Some(&server))?;
    os::set("ProxyOverride", "REG_SZ", Some(os::bypass()))?;
    os::set("ProxyEnable", "REG_DWORD", Some("1"))?;
    os::notify();
    store::log(app, "sysproxy", &server, true, "enabled");
    Ok(())
}

#[cfg(target_os = "windows")]
pub fn restore(app: &AppHandle) -> Result<(), String> {
    let b: Backup = store::load(app, FILE);
    if b.applied.is_empty() {
        return Ok(());
    }
    let now = os::query()?;
    if winreg::value(&now, "ProxyServer").as_deref() == Some(b.applied.as_str()) {
        // اول خاموش، تا لحظه‌ای با آدرس نیمه‌کاره روشن نماند
        let enable = b.enable.as_deref().map(|v| if v == "0x1" || v == "1" { "1" } else { "0" }).unwrap_or("0");
        os::set("ProxyEnable", "REG_DWORD", Some(enable))?;
        os::set("ProxyServer", "REG_SZ", b.server.as_deref())?;
        os::set("ProxyOverride", "REG_SZ", b.bypass.as_deref())?;
        os::notify();
        store::log(app, "sysproxy", &b.applied, true, "restored");
    }
    store::save(app, FILE, &Backup::default())
}

#[cfg(target_os = "windows")]
pub fn is_ours(app: &AppHandle) -> bool {
    let b: Backup = store::load(app, FILE);
    !b.applied.is_empty()
}

#[cfg(not(target_os = "windows"))]
pub fn enable(_: &AppHandle, _: u16) -> Result<(), String> {
    Err("تنظیم خودکار پروکسی سیستم فعلاً فقط روی ویندوز پشتیبانی می‌شود".into())
}
#[cfg(not(target_os = "windows"))]
pub fn restore(_: &AppHandle) -> Result<(), String> {
    Ok(())
}
#[cfg(not(target_os = "windows"))]
pub fn is_ours(_: &AppHandle) -> bool {
    false
}
