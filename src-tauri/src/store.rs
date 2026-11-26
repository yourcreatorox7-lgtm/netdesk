//! ذخیره‌سازی محلی: profiles.json، servers.json، history.json در پوشه‌ی داده‌ی برنامه.
//! نوشتن اتمیک (فایل موقت + rename) تا قطع برق فایل را خراب نکند.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tauri::{AppHandle, Manager};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct StoredProfile {
    pub fingerprint: String,
    pub protocol: String,
    pub name: String,
    pub host: String,
    pub port: u16,
    pub params: std::collections::BTreeMap<String, String>,
    pub folder: Option<String>,
    pub tags: Vec<String>,
    pub favorite: bool,
    pub standard_proxy: bool,
    pub has_secret: bool,
    pub added_at: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct Server {
    pub id: String,
    pub name: String,
    pub host: String,
    pub ssh_port: u16,
    pub ssh_user: String,
    /// مسیر کلید خصوصی SSH روی دستگاه (خود کلید کپی نمی‌شود)
    pub ssh_key_path: Option<String>,
    pub rdp_port: Option<u16>,
    pub rdp_user: Option<String>,
    pub tags: Vec<String>,
    pub provider: Option<String>,
    pub notes: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HistoryEntry {
    pub at: u64,
    pub kind: String,
    pub target: String,
    pub ok: bool,
    pub detail: String,
}

pub fn now() -> u64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0)
}

pub(crate) fn path(app: &AppHandle, name: &str) -> Result<PathBuf, String> {
    let dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    Ok(dir.join(name))
}

pub fn load<T: for<'de> Deserialize<'de> + Default>(app: &AppHandle, name: &str) -> T {
    path(app, name).ok().and_then(|p| std::fs::read_to_string(p).ok()).and_then(|s| serde_json::from_str(&s).ok()).unwrap_or_default()
}

pub fn save<T: Serialize>(app: &AppHandle, name: &str, v: &T) -> Result<(), String> {
    let p = path(app, name)?;
    let tmp = p.with_extension("tmp");
    std::fs::write(&tmp, serde_json::to_vec_pretty(v).map_err(|e| e.to_string())?).map_err(|e| e.to_string())?;
    std::fs::rename(tmp, p).map_err(|e| e.to_string())
}

pub fn log(app: &AppHandle, kind: &str, target: &str, ok: bool, detail: &str) {
    let mut h: Vec<HistoryEntry> = load(app, "history.json");
    h.push(HistoryEntry { at: now(), kind: kind.into(), target: target.into(), ok, detail: detail.chars().take(500).collect() });
    let keep = h.len().saturating_sub(2000); // سقف تاریخچه
    let _ = save(app, "history.json", &h[keep..].to_vec());
}

/// پشتیبان: فراداده‌ی پروفایل‌ها و سرورها، بدون رمز. رمزها در keyring می‌مانند
/// و کاربر باید آن‌ها را جداگانه منتقل کند — آگاهانه، نه پنهانی.
#[tauri::command]
pub fn backup_export(app: AppHandle) -> Result<String, String> {
    let profiles: Vec<StoredProfile> = load(&app, "profiles.json");
    let servers: Vec<Server> = load(&app, "servers.json");
    serde_json::to_string_pretty(&serde_json::json!({
        "format": "netdesk-backup",
        "version": 1,
        "createdAt": now(),
        "containsSecrets": false,
        "profiles": profiles,
        "servers": servers,
    }))
    .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn backup_import(app: AppHandle, json: String) -> Result<(usize, usize), String> {
    let v: serde_json::Value = serde_json::from_str(&json).map_err(|e| format!("فایل پشتیبان معتبر نیست: {e}"))?;
    if v["format"] != "netdesk-backup" {
        return Err("این فایل پشتیبان NetDesk نیست".into());
    }
    let p: Vec<StoredProfile> = serde_json::from_value(v["profiles"].clone()).unwrap_or_default();
    let s: Vec<Server> = serde_json::from_value(v["servers"].clone()).unwrap_or_default();
    let mut cur_p: Vec<StoredProfile> = load(&app, "profiles.json");
    let mut cur_s: Vec<Server> = load(&app, "servers.json");
    let (mut np, mut ns) = (0, 0);
    for x in p {
        if !cur_p.iter().any(|c| c.fingerprint == x.fingerprint) {
            cur_p.push(StoredProfile { has_secret: false, ..x });
            np += 1;
        }
    }
    for x in s {
        if !cur_s.iter().any(|c| c.id == x.id) {
            cur_s.push(x);
            ns += 1;
        }
    }
    save(&app, "profiles.json", &cur_p)?;
    save(&app, "servers.json", &cur_s)?;
    Ok((np, ns))
}
