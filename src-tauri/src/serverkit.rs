//! «کیت سرور» برای سرور ویندوزِ خود کاربر. NetDesk به سرور وصل نمی‌شود و رمز
//! RDP نمی‌گیرد؛ فقط پوشه‌ای می‌سازد که کاربر روی سرور کپی و اجرا می‌کند.

use crate::{commands, store, tools};
use netcore::serverkit;
use serde_json::{json, Value};
use std::path::{Path, PathBuf};
use std::process::Command;
use tauri::{AppHandle, Manager};

const SETUP: &str = include_str!("../../scripts/server/setup-server.ps1");
const PREFIX: &str = "NetDesk-Server-Kit-";

fn launcher(extra: &str) -> String {
    // مسیر از متغیر محیطی خوانده می‌شود تا نویسه‌های خاص در نام پوشه فرمان را تغییر ندهند
    let head = "@echo off\r\nset \"KIT=%~dp0\"\r\n";
    let run = r#"powershell -NoProfile -ExecutionPolicy Bypass -Command "Start-Process powershell -Verb RunAs -ArgumentList ('-NoProfile -ExecutionPolicy Bypass -NoExit -File \"' + $env:KIT + 'setup-server.ps1\"EXTRA')""#;
    format!("{head}{}\r\n", run.replace("EXTRA", extra))
}

/// خروجی `sing-box generate …` با همان هسته‌ی تأییدشده
fn generate(exe: &Path, args: &[&str]) -> Result<String, String> {
    let mut cmd = Command::new(exe);
    cmd.arg("generate").args(args);
    tools::hide_window(&mut cmd);
    let out = cmd.output().map_err(|e| format!("اجرای sing-box ممکن نشد: {e}"))?;
    let text = String::from_utf8_lossy(&out.stdout).trim().to_string();
    if !out.status.success() || text.is_empty() {
        return Err("sing-box نتوانست کلید بسازد".into());
    }
    Ok(text)
}

#[tauri::command]
pub async fn server_kit_create(app: AppHandle, host: String, port: u16, sni: String) -> Result<Value, String> {
    let host = host.trim().trim_matches(['[', ']']).to_string();
    let sni = sni.trim().to_string();
    serverkit::valid_host(&host)?;
    serverkit::valid_port(port)?;
    serverkit::valid_host(&sni)?;
    if sni.parse::<std::net::IpAddr>().is_ok() {
        return Err("سایت SNI باید نام دامنه باشد، نه IP".into());
    }
    tools::blocking(move || -> Result<Value, String> {
        let (exe, sha) = tools::core_binary(&app)?;
        let (private_key, public_key) =
            serverkit::parse_keypair(&generate(&exe, &["reality-keypair"])?).ok_or("خروجی کلید نامعتبر بود")?;
        let keys = serverkit::Keys {
            uuid: generate(&exe, &["uuid"])?,
            private_key,
            public_key,
            short_id: generate(&exe, &["rand", "8", "--hex"])?,
        };
        let desktop: PathBuf = app.path().desktop_dir().map_err(|e| e.to_string())?;
        let dir = desktop.join(format!("{PREFIX}{}", store::now()));
        std::fs::create_dir(&dir).map_err(|e| format!("ساخت پوشه‌ی کیت ممکن نشد: {e}"))?;
        let write = |name: &str, body: &str| std::fs::write(dir.join(name), body).map_err(|e| e.to_string());
        std::fs::copy(&exe, dir.join("sing-box.exe")).map_err(|e| e.to_string())?;
        write("config.json", &serverkit::server_config(&keys, port, &sni))?;
        write("kit.json", &json!({ "port": port, "sni": sni, "sha256": sha }).to_string())?;
        write("setup-server.ps1", SETUP)?;
        write("INSTALL.cmd", &launcher(""))?;
        write("UNINSTALL.cmd", &launcher(" -Uninstall"))?;
        write(
            "README.txt",
            "NetDesk server kit\r\n\r\n1) Copy this whole folder to your Windows server (through Remote Desktop).\r\n2) On the server, double-click INSTALL.cmd and click Yes.\r\n3) When you see DONE, go back to NetDesk and press the connect button.\r\n4) Delete this folder from both computers. config.json contains the server's private key.\r\n",
        )?;

        let link = serverkit::client_link(&keys, &host, port, &sni, &format!("سرور من ({host})"));
        let report = commands::import_profiles(app.clone(), link, Some("سرور من".into()))?;
        let fingerprint = report["lines"][0]["fingerprint"].as_str().ok_or("افزودن پروفایل ناموفق بود")?.to_string();
        store::log(&app, "server-kit", &host, true, &format!("port {port}"));
        Ok(json!({ "dir": dir.display().to_string(), "fingerprint": fingerprint, "port": port }))
    })
    .await?
}

/// فقط پوشه‌هایی که خود برنامه روی دسکتاپ ساخته باز می‌شوند
#[tauri::command]
pub fn server_kit_open(app: AppHandle, dir: String) -> Result<(), String> {
    let desktop = app.path().desktop_dir().map_err(|e| e.to_string())?;
    let p = PathBuf::from(&dir);
    let ours = p.parent() == Some(desktop.as_path())
        && p.file_name().and_then(|n| n.to_str()).is_some_and(|n| n.starts_with(PREFIX))
        && p.is_dir();
    if !ours {
        return Err("پوشه‌ی کیت پیدا نشد".into());
    }
    Command::new("explorer").arg(&p).spawn().map_err(|e| e.to_string())?;
    Ok(())
}
