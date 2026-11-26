//! اتصال خودکار: سنجش همه‌ی پروفایل‌ها، انتخاب بهترین، ترفند TLS fragment در
//! صورت فیلتر، تنظیم پروکسی سیستم، پایش اتصال و جابه‌جایی خودکار، و
//! به‌روزرسانی اشتراک‌ها.
//!
//! هر اتصال یا قطع، «نسل» (generation) را یکی بالا می‌برد؛ کارهای قدیمی
//! (تست در حال اجرا یا پایشگر) با دیدن نسل جدید خودشان متوقف می‌شوند.

use crate::{commands, secrets, store, sysproxy, tools};
use netcore::config::{self, Profile, Protocol};
use netcore::singbox::{self, Tweak};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::io::Write;
use std::net::{SocketAddr, TcpListener, TcpStream, ToSocketAddrs};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Mutex;
use std::time::{Duration, Instant};
use tauri::{AppHandle, Manager};

const SETTINGS: &str = "auto-settings.json";
const SUBS_KEY: &str = "subscriptions";
/// بیشترین تعداد پروفایلی که انتها-به-انتها تست می‌شود
const MAX_FULL_TESTS: usize = 10;
/// پس از این تعداد اتصال موفق، بهترین انتخاب می‌شود
const ENOUGH_SUCCESSES: usize = 3;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Settings {
    pub system_proxy: bool,
    pub watchdog: bool,
    pub connect_on_start: bool,
    pub update_subs_on_start: bool,
    pub port: u16,
}

impl Default for Settings {
    fn default() -> Self {
        Settings { system_proxy: true, watchdog: true, connect_on_start: false, update_subs_on_start: false, port: 10808 }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Connected {
    pub fingerprint: String,
    pub name: String,
    pub port: u16,
    pub latency_ms: Option<f64>,
    pub tweak: String,
    pub healthy: bool,
    pub since: u64,
}

#[derive(Debug, Clone, Serialize, Default)]
#[serde(rename_all = "camelCase")]
struct Info {
    /// idle | testing | connected | failed
    phase: String,
    steps: Vec<String>,
    connected: Option<Connected>,
}

#[derive(Default)]
pub struct AutoState {
    generation: AtomicU64,
    /// فقط یک دنباله‌ی اتصال هم‌زمان
    busy: Mutex<()>,
    /// «ثبت اتصال و روشن کردن پروکسی» و «قطع» نباید در هم بروند
    commit: Mutex<()>,
    info: Mutex<Info>,
}

fn state(app: &AppHandle) -> tauri::State<'_, AutoState> {
    app.state::<AutoState>()
}

fn with_info(app: &AppHandle, f: impl FnOnce(&mut Info)) {
    let s = state(app);
    let mut i = s.info.lock().unwrap_or_else(|e| e.into_inner());
    f(&mut i);
}

fn step(app: &AppHandle, text: String) {
    with_info(app, |i| {
        i.steps.push(text);
        let extra = i.steps.len().saturating_sub(60);
        i.steps.drain(..extra);
    });
}

fn bump(app: &AppHandle) -> u64 {
    state(app).generation.fetch_add(1, Ordering::SeqCst) + 1
}

fn current(app: &AppHandle, generation: u64) -> Result<(), String> {
    if state(app).generation.load(Ordering::SeqCst) == generation {
        Ok(())
    } else {
        Err("لغو شد".into())
    }
}

pub fn settings(app: &AppHandle) -> Settings {
    store::load(app, SETTINGS)
}

/// قطع کامل: توقف کارهای خودکار، بستن هسته و برگرداندن پروکسی سیستم
pub fn disconnect_inner(app: &AppHandle) {
    let s = state(app);
    let _commit = s.commit.lock().unwrap_or_else(|e| e.into_inner());
    bump(app);
    tools::core_kill(app);
    if let Err(e) = sysproxy::restore(app) {
        step(app, format!("برگرداندن پروکسی سیستم ناموفق بود: {e}"));
    }
    with_info(app, |i| {
        i.phase = "idle".into();
        i.connected = None;
    });
}

// ------------------------------------------------------------ نامزدها

struct Candidate {
    fingerprint: String,
    name: String,
    profile: Profile,
    tcp_ms: Option<f64>,
}

fn load_candidates(app: &AppHandle) -> Vec<Candidate> {
    let stored: Vec<store::StoredProfile> = store::load(app, "profiles.json");
    stored
        .into_iter()
        .filter_map(|s| {
            let raw = secrets::read(&format!("profile:{}", s.fingerprint))?;
            let profile = config::parse_line(&raw).ok()?;
            singbox::build_config(&profile, 10808).ok()?;
            Some(Candidate { fingerprint: s.fingerprint, name: s.name, profile, tcp_ms: None })
        })
        .collect()
}

fn is_udp(p: &Profile) -> bool {
    matches!(p.protocol, Protocol::Hysteria2 | Protocol::Tuic)
}

fn tcp_ms(host: &str, port: u16) -> Option<f64> {
    let addr = (host.trim_matches(['[', ']']), port).to_socket_addrs().ok()?.next()?;
    let t = Instant::now();
    TcpStream::connect_timeout(&addr, Duration::from_secs(3)).ok()?;
    Some(t.elapsed().as_secs_f64() * 1000.0)
}

/// سنجش موازی دسترسی TCP. خروجی: مرتب‌شده بر اساس تأخیر؛ پروتکل‌های UDP
/// (که با TCP سنجیده نمی‌شوند) در انتها می‌مانند و غیرقابل‌دسترس‌ها حذف می‌شوند.
fn rank(list: Vec<Candidate>) -> Vec<Candidate> {
    let mut out = Vec::new();
    let mut rest = list.into_iter().peekable();
    while rest.peek().is_some() {
        let batch: Vec<Candidate> = rest.by_ref().take(32).collect();
        std::thread::scope(|sc| {
            let handles: Vec<_> = batch
                .into_iter()
                .map(|mut c| {
                    sc.spawn(move || {
                        if !is_udp(&c.profile) {
                            c.tcp_ms = tcp_ms(&c.profile.host, c.profile.port);
                        }
                        c
                    })
                })
                .collect();
            out.extend(handles.into_iter().filter_map(|h| h.join().ok()));
        });
    }
    out.retain(|c| c.tcp_ms.is_some() || is_udp(&c.profile));
    out.sort_by(|a, b| a.tcp_ms.unwrap_or(f64::MAX).total_cmp(&b.tcp_ms.unwrap_or(f64::MAX)));
    out
}

/// اولین پورت آزاد از پورت دلخواه. اگر برنامه‌ی دیگری (مثلاً کلاینت پروکسی
/// دیگر) آن را گرفته باشد، تست سلامت نباید به‌اشتباه به آن برسد.
fn free_port(preferred: u16) -> Result<u16, String> {
    let start = preferred.max(1024);
    (start..start.saturating_add(40))
        .find(|p| TcpListener::bind(SocketAddr::from(([127, 0, 0, 1], *p))).is_ok())
        .ok_or_else(|| format!("پورت آزادی از {start} به بعد پیدا نشد"))
}

/// هسته را با این حالت اجرا و از داخل تونل تست می‌کند
fn attempt(app: &AppHandle, c: &Candidate, tweak: Tweak) -> Result<(u16, f64), String> {
    tools::core_kill(app);
    let port = free_port(settings(app).port)?;
    tools::core_start(app, &c.profile, &c.fingerprint, port, tweak)?;
    let ms = tools::core_health(port)?;
    Ok((port, ms))
}

fn publish(app: &AppHandle, generation: u64, c: &Candidate, port: u16, ms: Option<f64>, tweak: Tweak) -> Result<Connected, String> {
    let s = state(app);
    let _commit = s.commit.lock().unwrap_or_else(|e| e.into_inner());
    // اگر کاربر در همین فاصله «قطع» زده، پروکسی سیستم نباید روشن شود
    current(app, generation)?;
    let conn = Connected {
        fingerprint: c.fingerprint.clone(),
        name: c.name.clone(),
        port,
        latency_ms: ms,
        tweak: tweak.label().into(),
        healthy: ms.is_some(),
        since: store::now(),
    };
    let copy = conn.clone();
    with_info(app, |i| {
        i.phase = "connected".into();
        i.connected = Some(copy);
    });
    if settings(app).system_proxy {
        match sysproxy::enable(app, port) {
            Ok(()) => step(app, "پروکسی سیستم ویندوز تنظیم شد".into()),
            Err(e) => step(app, format!("تنظیم پروکسی سیستم ناموفق بود: {e}")),
        }
    }
    Ok(conn)
}

/// دنباله‌ی کامل انتخاب و اتصال. `skip` پروفایلی است که الان خراب شده و آخر امتحان می‌شود.
fn find_and_connect(app: &AppHandle, generation: u64, skip: Option<&str>) -> Result<Connected, String> {
    with_info(app, |i| i.phase = "testing".into());
    tools::core_ready(app)?;
    let all = load_candidates(app);
    if all.is_empty() {
        return Err("پروفایل قابل اتصالی نیست. در «پروفایل‌ها» لینک وارد کنید یا یک اشتراک اضافه کنید.".into());
    }
    step(app, format!("سنجش دسترسی {} پروفایل…", all.len()));
    let mut list = rank(all);
    current(app, generation)?;
    if list.is_empty() {
        return Err("هیچ سروری در دسترس نیست. اینترنت دستگاه را بررسی کنید.".into());
    }
    if let Some(s) = skip {
        list.sort_by_key(|c| c.fingerprint == s);
    }
    step(app, format!("{} سرور پاسخ دادند؛ تست واقعی از داخل تونل", list.len()));

    let mut ok: Vec<(usize, u16, f64, Tweak)> = Vec::new();
    for (idx, c) in list.iter().take(MAX_FULL_TESTS).enumerate() {
        for tweak in singbox::tweaks_for(&c.profile) {
            current(app, generation)?;
            match attempt(app, c, tweak) {
                Ok((port, ms)) => {
                    step(app, format!("✓ {} — {:.0} ms ({})", c.name, ms, tweak.label()));
                    ok.push((idx, port, ms, tweak));
                    break;
                }
                Err(e) => step(app, format!("✗ {} ({}) — {}", c.name, tweak.label(), e.lines().next().unwrap_or(""))),
            }
        }
        if ok.len() >= ENOUGH_SUCCESSES {
            break;
        }
    }

    let Some(&(idx, last_port, ms, tweak)) = ok.iter().min_by(|a, b| a.2.total_cmp(&b.2)) else {
        tools::core_kill(app);
        return Err("هیچ پروفایلی از داخل تونل پاسخ نداد (سرور، UUID/رمز یا فیلتر).".into());
    };
    current(app, generation)?;
    let best = &list[idx];
    // اگر بهترین، آخرین آزمایش نبوده، دوباره اجرایش کن
    let still_running = ok.last().map(|l| l.0) == Some(idx) && tools::core_alive(app, &best.fingerprint);
    let (port, ms) = if still_running { (last_port, ms) } else { attempt(app, best, tweak)? };
    current(app, generation)?;
    step(app, format!("اتصال برقرار شد: {}", best.name));
    store::log(app, "auto-connect", &best.fingerprint, true, tweak.label());
    publish(app, generation, best, port, Some(ms), tweak)
}

fn fail(app: &AppHandle, generation: u64, error: &str) {
    if current(app, generation).is_err() {
        // کاربر قطع کرده یا اتصال تازه‌ای در صف است. فراخواننده هنوز `busy` را دارد،
        // پس هر هسته‌ای که روشن مانده مال همین دنباله است و باید بسته شود.
        tools::core_kill(app);
        return;
    }
    tools::core_kill(app);
    let _ = sysproxy::restore(app);
    step(app, error.to_string());
    with_info(app, |i| {
        i.phase = "failed".into();
        i.connected = None;
    });
}

/// پایشگر: هر ۲۰ ثانیه سلامت تونل را می‌سنجد؛ پس از ۳ خطای پیاپی جابه‌جا می‌شود
fn start_watchdog(app: AppHandle, generation: u64) {
    std::thread::spawn(move || {
        let mut failures = 0;
        loop {
            for _ in 0..20 {
                std::thread::sleep(Duration::from_secs(1));
                if current(&app, generation).is_err() {
                    return;
                }
            }
            let conn = {
                let s = state(&app);
                let i = s.info.lock().unwrap_or_else(|e| e.into_inner());
                i.connected.clone()
            };
            let Some(conn) = conn else { return };
            let result = if tools::core_alive(&app, &conn.fingerprint) {
                tools::core_health(conn.port)
            } else {
                failures = 2; // هسته بسته شده؛ صبر فایده ندارد
                Err("هسته بسته شده".into())
            };
            if current(&app, generation).is_err() {
                return;
            }
            match result {
                Ok(ms) => {
                    failures = 0;
                    with_info(&app, |i| {
                        if let Some(c) = i.connected.as_mut() {
                            c.latency_ms = Some(ms);
                            c.healthy = true;
                        }
                    });
                }
                Err(_) => {
                    failures += 1;
                    with_info(&app, |i| {
                        if let Some(c) = i.connected.as_mut() {
                            c.healthy = false;
                        }
                    });
                    if failures >= 3 {
                        step(&app, format!("اتصال «{}» قطع شد؛ جابه‌جایی خودکار…", conn.name));
                        let s = state(&app);
                        let _busy = s.busy.lock().unwrap_or_else(|e| e.into_inner());
                        if let Err(e) = find_and_connect(&app, generation, Some(&conn.fingerprint)) {
                            fail(&app, generation, &e);
                            return;
                        }
                        failures = 0;
                    }
                }
            }
        }
    });
}

pub fn smart_connect_blocking(app: &AppHandle) -> Result<Connected, String> {
    let generation = bump(app);
    let s = state(app);
    let _busy = s.busy.lock().unwrap_or_else(|e| e.into_inner());
    current(app, generation)?;
    with_info(app, |i| {
        i.steps.clear();
        i.connected = None;
    });
    match find_and_connect(app, generation, None) {
        Ok(c) => {
            if settings(app).watchdog {
                start_watchdog(app.clone(), generation);
            }
            Ok(c)
        }
        Err(e) => {
            fail(app, generation, &e);
            Err(e)
        }
    }
}

/// اتصال دستی به یک پروفایل (از صفحه‌ی پروفایل‌ها)
pub fn connect_one_blocking(app: &AppHandle, fingerprint: &str) -> Result<Connected, String> {
    let generation = bump(app);
    let s = state(app);
    let _busy = s.busy.lock().unwrap_or_else(|e| e.into_inner());
    current(app, generation)?;
    with_info(app, |i| {
        i.steps.clear();
        i.phase = "testing".into();
        i.connected = None;
    });
    let run = || -> Result<Connected, String> {
        tools::core_ready(app)?;
        let c = load_candidates(app)
            .into_iter()
            .find(|c| c.fingerprint == fingerprint)
            .ok_or("متن کامل این پروفایل در انبار امن نیست، یا sing-box آن را پشتیبانی نمی‌کند")?;
        let mut last_error = String::new();
        for tweak in singbox::tweaks_for(&c.profile) {
            current(app, generation)?;
            match attempt(app, &c, tweak) {
                Ok((port, ms)) => return publish(app, generation, &c, port, Some(ms), tweak),
                Err(e) => {
                    step(app, format!("✗ {} ({}) — {}", c.name, tweak.label(), e.lines().next().unwrap_or("")));
                    last_error = e;
                }
            }
        }
        Err(last_error)
    };
    match run() {
        Ok(c) => {
            if settings(app).watchdog {
                start_watchdog(app.clone(), generation);
            }
            Ok(c)
        }
        Err(e) => {
            fail(app, generation, &e);
            Err(e)
        }
    }
}

// ------------------------------------------------------------ اشتراک‌ها

fn subs() -> Vec<String> {
    secrets::read(SUBS_KEY).map(|s| s.lines().map(str::to_string).filter(|l| !l.is_empty()).collect()).unwrap_or_default()
}

fn save_subs(v: &[String]) -> Result<(), String> {
    secrets::write(SUBS_KEY, &v.join("\n"))
}

fn host_of(url: &str) -> String {
    let rest = url.split_once("://").map(|x| x.1).unwrap_or(url);
    let authority = rest.split(['/', '?', '#']).next().unwrap_or("");
    authority.rsplit('@').next().unwrap_or("").to_string()
}

fn valid_sub_url(url: &str) -> Result<(), String> {
    if !url.starts_with("https://") || url.len() > 1000 || host_of(url).is_empty() {
        return Err("نشانی اشتراک باید با https:// شروع شود".into());
    }
    if url.chars().any(|c| c.is_whitespace() || c.is_control() || c == '"' || c == '\\') {
        return Err("نشانی اشتراک کاراکتر غیرمجاز دارد".into());
    }
    Ok(())
}

/// دریافت با curl خود ویندوز. نشانی (که معمولاً رمز دارد) از stdin داده می‌شود
/// تا در فهرست پردازه‌ها دیده نشود.
fn fetch(url: &str) -> Result<String, String> {
    valid_sub_url(url)?;
    let mut cmd = Command::new("curl");
    cmd.args(["--config", "-"]).stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::piped());
    tools::hide_window(&mut cmd);
    let mut child = cmd.spawn().map_err(|e| format!("curl اجرا نشد: {e}"))?;
    let conf = format!(
        "url = \"{url}\"\ngloboff\nproto = \"=https\"\nproto-redir = \"=https\"\nlocation\nmax-redirs = 5\nconnect-timeout = 10\nmax-time = 30\nmax-filesize = 2000000\nfail\nsilent\nshow-error\n"
    );
    child.stdin.take().ok_or("stdin در دسترس نیست")?.write_all(conf.as_bytes()).map_err(|e| e.to_string())?;
    let out = child.wait_with_output().map_err(|e| e.to_string())?;
    if !out.status.success() {
        let err = String::from_utf8_lossy(&out.stderr).replace(url, "<url>");
        return Err(err.trim().to_string());
    }
    if out.stdout.len() > 2_000_000 {
        return Err("اشتراک بیش از حد بزرگ است".into());
    }
    Ok(String::from_utf8_lossy(&out.stdout).into_owned())
}

pub fn update_subs_blocking(app: &AppHandle) -> Value {
    let (mut added, mut errors) = (0u64, Vec::new());
    for url in subs() {
        let host = host_of(&url);
        match fetch(&url).and_then(|text| commands::import_profiles(app.clone(), text, Some(format!("اشتراک {host}")))) {
            Ok(r) => added += r["added"].as_u64().unwrap_or(0),
            Err(e) => errors.push(format!("{host}: {e}")),
        }
    }
    store::log(app, "subscription", "update", errors.is_empty(), &format!("added={added}"));
    json!({ "added": added, "errors": errors })
}

/// کارهای خودکار هنگام باز شدن برنامه
pub fn on_start(app: AppHandle) {
    // اگر دفعه‌ی قبل برنامه ناگهانی بسته شده، پروکسی سیستم را برگردان
    let _ = sysproxy::restore(&app);
    with_info(&app, |i| i.phase = "idle".into());
    std::thread::spawn(move || {
        let s = settings(&app);
        if s.update_subs_on_start && !subs().is_empty() {
            update_subs_blocking(&app);
        }
        if s.connect_on_start {
            let _ = smart_connect_blocking(&app);
        }
    });
}

// ------------------------------------------------------------ فرمان‌ها

#[tauri::command]
pub fn auto_status(app: AppHandle) -> Value {
    let s = state(&app);
    let info = s.info.lock().unwrap_or_else(|e| e.into_inner()).clone();
    json!({
        "phase": if info.phase.is_empty() { "idle" } else { info.phase.as_str() },
        "steps": info.steps,
        "connected": info.connected,
        "systemProxy": sysproxy::is_ours(&app),
    })
}

#[tauri::command]
pub async fn smart_connect(app: AppHandle) -> Result<Connected, String> {
    tools::blocking(move || smart_connect_blocking(&app)).await?
}

#[tauri::command]
pub async fn auto_disconnect(app: AppHandle) -> Result<(), String> {
    tools::blocking(move || disconnect_inner(&app)).await
}

#[tauri::command]
pub fn auto_settings_get(app: AppHandle) -> Settings {
    settings(&app)
}

#[tauri::command]
pub async fn auto_settings_set(app: AppHandle, settings: Settings) -> Result<(), String> {
    if settings.port < 1024 {
        return Err("پورت باید ۱۰۲۴ یا بیشتر باشد".into());
    }
    store::save(&app, SETTINGS, &settings)?;
    tools::blocking(move || {
        // روشن/خاموش کردن پروکسی سیستم بلافاصله روی اتصال فعلی اعمال می‌شود
        let port = {
            let s = state(&app);
            let i = s.info.lock().unwrap_or_else(|e| e.into_inner());
            i.connected.as_ref().map(|c| c.port)
        };
        match (settings.system_proxy, port) {
            (true, Some(p)) => sysproxy::enable(&app, p),
            (false, _) => sysproxy::restore(&app),
            _ => Ok(()),
        }
    })
    .await?
}

#[tauri::command]
pub fn sub_list() -> Vec<String> {
    // فقط نام میزبان به رابط کاربری می‌رود، نه کل نشانی
    subs().iter().map(|u| host_of(u)).collect()
}

#[tauri::command]
pub fn sub_add(url: String) -> Result<(), String> {
    let url = url.trim().to_string();
    valid_sub_url(&url)?;
    let mut v = subs();
    if !v.contains(&url) {
        v.push(url);
    }
    if v.join("\n").len() > 2000 {
        return Err("ظرفیت انبار امن برای اشتراک‌ها پر است".into());
    }
    save_subs(&v)
}

#[tauri::command]
pub fn sub_remove(index: usize) -> Result<(), String> {
    let mut v = subs();
    if index < v.len() {
        v.remove(index);
    }
    save_subs(&v)
}

#[tauri::command]
pub async fn sub_update(app: AppHandle) -> Result<Value, String> {
    tools::blocking(move || update_subs_blocking(&app)).await
}
