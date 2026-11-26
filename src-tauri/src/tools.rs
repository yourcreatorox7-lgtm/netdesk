//! اجرای ابزارهای خارجی پرخطر و هسته‌ی sing-box.
//!
//! قواعد امنیتی (در Rust، نه در رابط کاربری):
//! - فقط شناسه‌های موجود در `netcore::tools::catalog()` اجرا می‌شوند.
//! - مسیر فایل اجرایی را کاربر انتخاب می‌کند؛ هش SHA-256 آن سنجاق می‌شود و
//!   پیش از هر اجرا دوباره محاسبه می‌شود. اگر فرق کند، تأیید باطل می‌شود.
//! - بدون تأیید معتبر (همه‌ی هشدارها + عبارت تأیید + نسخه‌ی فعلی هشدارها) اجرا ممکن نیست.
//! - هیچ shellی در کار نیست؛ آرگومان‌ها مستقیم به پردازه داده می‌شوند.
//! - ابزارهای نیازمند دسترسی مدیر با UAC ویندوز (یا pkexec لینوکس) اجرا
//!   می‌شوند؛ خود NetDesk هرگز دسترسی مدیر ندارد.

use crate::{auto, store};
use netcore::{config::Profile, proxy, sha256, singbox, tools as cat};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::Mutex;
use std::time::{Duration, Instant};
use tauri::{AppHandle, Manager};

const FILE: &str = "tools.json";

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ToolSetup {
    pub id: String,
    pub path: String,
    pub args: String,
    pub cwd: Option<String>,
    pub sha256: String,
    pub consent_version: Option<u32>,
    pub consented_at: Option<u64>,
}

pub struct RunInfo {
    child: Option<Child>,
    elevated: bool,
    started_at: u64,
    port: Option<u16>,
    profile: Option<String>,
}

#[derive(Default)]
pub struct Running(pub Mutex<HashMap<String, RunInfo>>);

impl Running {
    /// هنگام بستن برنامه همه‌ی پردازه‌های فرزند بسته می‌شوند
    pub fn kill_all(&self) {
        if let Ok(mut m) = self.0.lock() {
            for (_, r) in m.iter_mut() {
                if let Some(c) = r.child.as_mut() {
                    let _ = c.kill();
                    let _ = c.wait();
                }
            }
            m.clear();
        }
    }
}

pub(crate) async fn blocking<T: Send + 'static>(f: impl FnOnce() -> T + Send + 'static) -> Result<T, String> {
    tauri::async_runtime::spawn_blocking(f).await.map_err(|e| e.to_string())
}

fn spec(id: &str) -> Result<&'static cat::ToolSpec, String> {
    cat::find(id).ok_or_else(|| format!("ابزار ناشناخته: {id}"))
}

fn load(app: &AppHandle) -> Vec<ToolSetup> {
    store::load(app, FILE)
}

fn save_one(app: &AppHandle, t: ToolSetup) -> Result<(), String> {
    let mut v = load(app);
    v.retain(|x| x.id != t.id);
    v.push(t);
    store::save(app, FILE, &v)
}

fn consent_valid(t: &ToolSetup) -> bool {
    t.consent_version == Some(cat::CONSENT_VERSION) && t.consented_at.is_some()
}

/// پیش از هر اجرا: تأیید معتبر و هش بدون تغییر
fn verified(app: &AppHandle, id: &str) -> Result<(&'static cat::ToolSpec, ToolSetup), String> {
    let s = spec(id)?;
    let mut t = load(app).into_iter().find(|x| x.id == id).ok_or("ابتدا مسیر فایل اجرایی را تنظیم کنید")?;
    if !consent_valid(&t) {
        return Err("هشدارهای این ابزار هنوز تأیید نشده (یا متن هشدارها به‌روز شده و باید دوباره تأیید کنید)".into());
    }
    let now_hash = sha256::file_hex(Path::new(&t.path))?;
    if now_hash != t.sha256 {
        t.consent_version = None;
        t.consented_at = None;
        let _ = save_one(app, t.clone());
        store::log(app, "tool", id, false, "hash-mismatch");
        return Err(format!(
            "فایل اجرایی تغییر کرده است!\nهش سنجاق‌شده: {}\nهش فعلی: {now_hash}\nتأیید باطل شد. اگر خودتان به‌روزش کرده‌اید، دوباره تنظیم و تأیید کنید.",
            t.sha256
        ));
    }
    Ok((s, t))
}

pub(crate) fn log_path(app: &AppHandle, id: &str) -> Result<PathBuf, String> {
    let dir = app.path().app_data_dir().map_err(|e| e.to_string())?.join("logs");
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    Ok(dir.join(format!("{id}.log")))
}

pub(crate) fn tail(path: &Path, max: usize) -> String {
    let b = std::fs::read(path).unwrap_or_default();
    let start = b.len().saturating_sub(max);
    String::from_utf8_lossy(&b[start..]).into_owned()
}

pub(crate) fn hide_window(cmd: &mut Command) {
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x0800_0000); // CREATE_NO_WINDOW
    }
    let _ = cmd;
}

fn spawn_logged(app: &AppHandle, id: &str, path: &str, args: &[String], cwd: Option<&str>) -> Result<Child, String> {
    let lp = log_path(app, id)?;
    let out = std::fs::File::create(&lp).map_err(|e| e.to_string())?;
    let err = out.try_clone().map_err(|e| e.to_string())?;
    let mut cmd = Command::new(path);
    cmd.args(args).stdin(Stdio::null()).stdout(out).stderr(err);
    if let Some(c) = cwd {
        cmd.current_dir(c);
    }
    hide_window(&mut cmd);
    cmd.spawn().map_err(|e| format!("اجرای «{path}» ممکن نشد: {e}"))
}

/// اجرا با دسترسی مدیر از طریق تأیید سیستم‌عامل (UAC / pkexec).
/// NetDesk به این پردازه دسترسی ندارد و نمی‌تواند متوقفش کند.
fn spawn_elevated(path: &str, args: &[String], cwd: Option<&str>) -> Result<(), String> {
    #[cfg(target_os = "windows")]
    {
        let ps = |s: &str| format!("'{}'", s.replace('\'', "''"));
        let winq = |a: &String| {
            if a.is_empty() || a.contains([' ', '\t', '"']) { format!("\"{}\"", a.replace('"', "\\\"")) } else { a.clone() }
        };
        let mut script = format!("Start-Process -FilePath {} -Verb RunAs", ps(path));
        let al: Vec<String> = args.iter().map(winq).collect();
        if !al.is_empty() {
            script.push_str(&format!(" -ArgumentList {}", ps(&al.join(" "))));
        }
        if let Some(c) = cwd {
            script.push_str(&format!(" -WorkingDirectory {}", ps(c)));
        }
        let mut cmd = Command::new("powershell");
        cmd.args(["-NoProfile", "-NonInteractive", "-Command", &script]);
        hide_window(&mut cmd);
        let st = cmd.status().map_err(|e| e.to_string())?;
        if !st.success() {
            return Err("اجرا با دسترسی مدیر لغو شد (پنجره‌ی UAC رد شد یا فایل اجرا نشد)".into());
        }
        Ok(())
    }
    #[cfg(target_os = "linux")]
    {
        let mut cmd = Command::new("pkexec");
        cmd.arg(path).args(args);
        if let Some(c) = cwd {
            cmd.current_dir(c);
        }
        cmd.spawn().map_err(|e| format!("pkexec پیدا نشد: {e}"))?;
        Ok(())
    }
    #[cfg(not(any(target_os = "windows", target_os = "linux")))]
    {
        let _ = (path, args, cwd);
        Err("اجرای ابزار با دسترسی مدیر روی این سیستم‌عامل پشتیبانی نمی‌شود".into())
    }
}

fn status_of(r: &mut RunInfo) -> Value {
    let alive = match r.child.as_mut() {
        Some(c) => matches!(c.try_wait(), Ok(None)),
        None => r.elevated,
    };
    json!({ "alive": alive, "elevated": r.elevated, "startedAt": r.started_at, "port": r.port, "profile": r.profile })
}

// ---------------------------------------------------------------- فرمان‌ها

#[tauri::command]
pub fn tool_list(app: AppHandle) -> Vec<Value> {
    let setups = load(&app);
    let running = app.state::<Running>();
    let mut run = running.0.lock().unwrap_or_else(|e| e.into_inner());
    cat::catalog()
        .iter()
        .map(|s| {
            let t = setups.iter().find(|x| x.id == s.id);
            json!({
                "id": s.id, "name": s.name, "url": s.url, "license": s.license, "summary": s.summary,
                "risk": s.risk.as_str(), "confirmPhrase": s.risk.confirm_phrase(),
                "requiresAdmin": s.requires_admin, "installsRootCa": s.installs_root_ca, "unreviewed": s.unreviewed,
                "warnings": s.warnings, "setupHint": s.setup_hint,
                "setup": t, "consented": t.map(consent_valid).unwrap_or(false),
                "running": run.get_mut(s.id).map(status_of),
            })
        })
        .collect()
}

/// تنظیم مسیر. هر تغییری تأیید قبلی را باطل می‌کند.
#[tauri::command]
pub async fn tool_configure(app: AppHandle, id: String, path: String, args: String, cwd: Option<String>) -> Result<ToolSetup, String> {
    spec(&id)?;
    let p = PathBuf::from(path.trim());
    if !p.is_absolute() || !p.is_file() {
        return Err("مسیر باید کامل و به یک فایل موجود باشد".into());
    }
    cat::split_args(&args)?;
    let cwd = cwd.map(|c| c.trim().to_string()).filter(|c| !c.is_empty());
    if let Some(c) = &cwd {
        if !Path::new(c).is_dir() {
            return Err("پوشه‌ی کاری وجود ندارد".into());
        }
    }
    let pc = p.clone();
    let hash = blocking(move || sha256::file_hex(&pc)).await??;
    let t = ToolSetup { id: id.clone(), path: p.display().to_string(), args, cwd, sha256: hash, consent_version: None, consented_at: None };
    save_one(&app, t.clone())?;
    store::log(&app, "tool-config", &id, true, &t.sha256);
    Ok(t)
}

#[tauri::command]
pub fn tool_consent(app: AppHandle, id: String, acknowledged: Vec<usize>, phrase: String) -> Result<(), String> {
    let s = spec(&id)?;
    let mut t = load(&app).into_iter().find(|x| x.id == id).ok_or("ابتدا مسیر فایل اجرایی را تنظیم کنید")?;
    cat::check_consent(s, &acknowledged, &phrase)?;
    t.consent_version = Some(cat::CONSENT_VERSION);
    t.consented_at = Some(store::now());
    save_one(&app, t)?;
    store::log(&app, "tool-consent", &id, true, s.risk.as_str());
    Ok(())
}

#[tauri::command]
pub fn tool_forget(app: AppHandle, id: String) -> Result<(), String> {
    let _ = tool_stop(app.clone(), id.clone());
    let mut v = load(&app);
    v.retain(|x| x.id != id);
    store::save(&app, FILE, &v)
}

#[tauri::command]
pub async fn tool_start(app: AppHandle, id: String) -> Result<Value, String> {
    if id == "sing-box" {
        return Err("sing-box از صفحه‌ی پروفایل‌ها با دکمه‌ی «اتصال» اجرا می‌شود".into());
    }
    let a = app.clone();
    blocking(move || -> Result<Value, String> {
        let (s, t) = verified(&a, &id)?;
        let args = cat::split_args(&t.args)?;
        let state = a.state::<Running>();
        // قفل پیش از اجرا آزاد می‌شود: پنجره‌ی UAC نباید رابط کاربری را منتظر نگه دارد
        {
            let mut run = state.0.lock().map_err(|e| e.to_string())?;
            if let Some(r) = run.get_mut(&id) {
                if status_of(r)["alive"].as_bool() == Some(true) && !r.elevated {
                    return Err("این ابزار در حال اجراست".into());
                }
            }
        }
        let info = if s.requires_admin {
            spawn_elevated(&t.path, &args, t.cwd.as_deref())?;
            RunInfo { child: None, elevated: true, started_at: store::now(), port: None, profile: None }
        } else {
            let c = spawn_logged(&a, &id, &t.path, &args, t.cwd.as_deref())?;
            RunInfo { child: Some(c), elevated: false, started_at: store::now(), port: None, profile: None }
        };
        let mut info = info;
        let st = status_of(&mut info);
        state.0.lock().map_err(|e| e.to_string())?.insert(id.clone(), info);
        store::log(&a, "tool-start", &id, true, if s.requires_admin { "elevated" } else { "normal" });
        Ok(st)
    })
    .await?
}

#[tauri::command]
pub fn tool_stop(app: AppHandle, id: String) -> Result<String, String> {
    if id == "sing-box" {
        auto::disconnect_inner(&app);
        return Ok("اتصال قطع شد".into());
    }
    let state = app.state::<Running>();
    let mut run = state.0.lock().map_err(|e| e.to_string())?;
    let Some(mut r) = run.remove(&id) else { return Ok("در حال اجرا نبود".into()) };
    store::log(&app, "tool-stop", &id, true, "");
    match r.child.as_mut() {
        Some(c) => {
            let _ = c.kill();
            let _ = c.wait();
            Ok("متوقف شد".into())
        }
        None => Ok("این ابزار با دسترسی مدیر اجرا شده و NetDesk نمی‌تواند متوقفش کند؛ پنجره‌ی خودش را ببندید.".into()),
    }
}

#[tauri::command]
pub fn tool_log(app: AppHandle, id: String) -> Result<String, String> {
    spec(&id)?;
    Ok(tail(&log_path(&app, &id)?, 16 * 1024))
}

// ------------------------------------------------------------ هسته‌ی sing-box

/// پیش از شروع تست‌ها: تنظیم، تأیید و هش هسته درست باشد
pub(crate) fn core_ready(app: &AppHandle) -> Result<(), String> {
    verified(app, "sing-box").map(|_| ()).map_err(|e| format!("sing-box آماده نیست: {e} (آموزش sing-box را ببینید)"))
}

pub(crate) fn core_kill(app: &AppHandle) {
    let state = app.state::<Running>();
    let mut run = state.0.lock().unwrap_or_else(|e| e.into_inner());
    if let Some(mut r) = run.remove("sing-box") {
        if let Some(c) = r.child.as_mut() {
            let _ = c.kill();
            let _ = c.wait();
        }
    }
}

pub(crate) fn core_alive(app: &AppHandle, fingerprint: &str) -> bool {
    let state = app.state::<Running>();
    let mut run = state.0.lock().unwrap_or_else(|e| e.into_inner());
    run.get_mut("sing-box")
        .filter(|r| r.profile.as_deref() == Some(fingerprint))
        .and_then(|r| r.child.as_mut())
        .is_some_and(|c| matches!(c.try_wait(), Ok(None)))
}

/// مسیر و هش هسته‌ی تأییدشده (برای ساخت کیت سرور)
pub(crate) fn core_binary(app: &AppHandle) -> Result<(PathBuf, String), String> {
    core_ready(app)?;
    let (_, t) = verified(app, "sing-box")?;
    Ok((PathBuf::from(t.path), t.sha256))
}

/// هسته را اجرا می‌کند و تا باز شدن پورت محلی صبر می‌کند. فراخواننده باید
/// پیش از این مطمئن شده باشد که پورت آزاد است.
pub(crate) fn core_start(app: &AppHandle, p: &Profile, fingerprint: &str, port: u16, tweak: singbox::Tweak) -> Result<(), String> {
    let (_, t) = verified(app, "sing-box")?;
    let cfg = singbox::build_config_with(p, port, tweak)?;
    let dir = app.path().app_data_dir().map_err(|e| e.to_string())?.join("run");
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let file = dir.join(format!("sb-{fingerprint}.json"));
    std::fs::write(&file, &cfg).map_err(|e| e.to_string())?;
    let args = vec!["run".to_string(), "-c".to_string(), file.display().to_string()];
    let mut child = match spawn_logged(app, "sing-box", &t.path, &args, None) {
        Ok(c) => c,
        Err(e) => {
            let _ = std::fs::remove_file(&file);
            return Err(e);
        }
    };
    let local: std::net::SocketAddr = ([127, 0, 0, 1], port).into();
    let started = Instant::now();
    let mut listening = false;
    while started.elapsed() < Duration::from_secs(8) && matches!(child.try_wait(), Ok(None)) {
        std::thread::sleep(Duration::from_millis(150));
        if std::net::TcpStream::connect_timeout(&local, Duration::from_millis(250)).is_ok() {
            listening = true;
            break;
        }
    }
    // هسته کانفیگ را خوانده (یا شکست خورده)؛ نسخه‌ی روی دیسک که رمز دارد پاک می‌شود
    let _ = std::fs::remove_file(&file);
    if !listening || !matches!(child.try_wait(), Ok(None)) {
        let _ = child.kill();
        let _ = child.wait();
        let log = tail(&log_path(app, "sing-box")?, 1500);
        store::log(app, "singbox", fingerprint, false, &log);
        return Err(format!("sing-box اجرا نشد: {}", log.trim().lines().last().unwrap_or("بدون پیام")));
    }
    let state = app.state::<Running>();
    let mut run = state.0.lock().map_err(|e| e.to_string())?;
    run.insert("sing-box".into(), RunInfo { child: Some(child), elevated: false, started_at: store::now(), port: Some(port), profile: Some(fingerprint.to_string()) });
    Ok(())
}

/// تست انتها-به-انتها از داخل تونل؛ خروجی تأخیر بر حسب میلی‌ثانیه
pub(crate) fn core_health(port: u16) -> Result<f64, String> {
    let local: std::net::SocketAddr = ([127, 0, 0, 1], port).into();
    proxy::check_socks5_http(local, "www.gstatic.com", "/generate_204", Duration::from_secs(8))
}

/// اتصال دستی به یک پروفایل
#[tauri::command]
pub async fn singbox_connect(app: AppHandle, fingerprint: String) -> Result<auto::Connected, String> {
    blocking(move || auto::connect_one_blocking(&app, &fingerprint)).await?
}

/// فایل‌هایی که اسکریپت scripts/build-tool.ps1 ساخته (یا روی PATH هستند) پیدا می‌کند.
/// فقط فیلدها را پر می‌کند؛ ثبت هش و تأیید هشدارها همچنان با کاربر است.
#[tauri::command]
pub fn tool_detect(id: String) -> Result<Option<Value>, String> {
    spec(&id)?;
    let Some(local) = std::env::var_os("LOCALAPPDATA").map(PathBuf::from) else { return Ok(None) };
    let base = local.join("NetDesk");
    let exe = match id.as_str() {
        "sing-box" => Some("sing-box.exe"),
        "xray" => Some("xray.exe"),
        "flowdriver" => Some("flowdriver-client.exe"),
        "sni-spoofing-go" => Some("sni-spoofing-go.exe"),
        "sni-spoofing-rust" => Some("sni-spoof-rs.exe"),
        _ => None,
    };
    if let Some(name) = exe {
        let mut candidates = vec![base.join("bin").join(name)];
        if let Some(path) = std::env::var_os("PATH") {
            candidates.extend(std::env::split_paths(&path).filter(|d| d.is_absolute()).map(|d| d.join(name)));
        }
        return Ok(candidates.into_iter().find(|c| c.is_file()).map(|c| json!({ "path": c.display().to_string(), "args": "", "cwd": null })));
    }
    let dir = base.join("tools").join(&id);
    let python = dir.join(".venv").join("Scripts").join("python.exe");
    Ok(python.is_file().then(|| json!({ "path": python.display().to_string(), "args": "main.py", "cwd": dir.display().to_string() })))
}
