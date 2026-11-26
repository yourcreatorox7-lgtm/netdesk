//! اجرای دستورهای ویندوز برای دیدن VPN یا پروکسی دیگری که پیش از NetDesk
//! ترافیک را می‌گیرد. منطق تحلیل متن در `netcore::netenv` است (قابل تست).

use netcore::netenv;
use tauri::AppHandle;

#[cfg(target_os = "windows")]
fn up_adapters() -> String {
    let mut cmd = std::process::Command::new("powershell");
    cmd.args([
        "-NoProfile",
        "-NonInteractive",
        "-Command",
        "Get-NetAdapter | Where-Object Status -eq 'Up' | ForEach-Object { $_.Name + '|' + $_.InterfaceDescription }",
    ]);
    crate::tools::hide_window(&mut cmd);
    cmd.output().map(|o| String::from_utf8_lossy(&o.stdout).into_owned()).unwrap_or_default()
}

#[cfg(target_os = "windows")]
fn proxy_settings() -> String {
    let mut cmd = std::process::Command::new("reg");
    cmd.args(["query", r"HKCU\Software\Microsoft\Windows\CurrentVersion\Internet Settings"]);
    crate::tools::hide_window(&mut cmd);
    cmd.output().map(|o| String::from_utf8_lossy(&o.stdout).into_owned()).unwrap_or_default()
}

/// هشدارهای فارسی؛ فهرست خالی یعنی چیزی مزاحم دیده نشد. مسدودکننده است (حدود یک ثانیه).
#[cfg(target_os = "windows")]
pub fn scan(app: &AppHandle) -> Vec<String> {
    let mut out = Vec::new();
    for name in netenv::tunnel_adapters(&up_adapters()) {
        out.push(format!("«{name}» (VPN/TUN) فعال است و ترافیک NetDesk هم از آن رد می‌شود. برای تست مستقیم، آن را موقتاً خاموش کنید."));
    }
    if !crate::sysproxy::is_ours(app) {
        if let Some(server) = netenv::system_proxy(&proxy_settings()) {
            out.push(format!("پروکسی سیستم روی {server} (برنامه‌ی دیگر) تنظیم است. NetDesk هنگام اتصال آن را عوض می‌کند و بعد برمی‌گرداند."));
        }
    }
    out
}

#[cfg(not(target_os = "windows"))]
pub fn scan(_: &AppHandle) -> Vec<String> {
    Vec::new()
}

#[tauri::command]
pub async fn net_env(app: AppHandle) -> Result<Vec<String>, String> {
    crate::tools::blocking(move || scan(&app)).await
}
