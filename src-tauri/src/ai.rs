//! دستیار هوش مصنوعی **محلی** با Ollama (فقط 127.0.0.1).
//!
//! قواعد امنیتی:
//! - مدل هیچ‌وقت رمز، UUID، نشانی سرور، نام پروفایل یا لینک اشتراک نمی‌بیند.
//! - مدل فقط می‌تواند یکی از کارهای فهرست `ACTIONS` را «پیشنهاد» بدهد؛ اجرای
//!   فرمان دلخواه، فایل یا ابزار پرخطر و تأیید هشدارها برایش وجود ندارد.

use crate::{auto, store, tools};
use netcore::{localhttp, netenv};
use serde::Deserialize;
use serde_json::{json, Value};
use std::net::SocketAddr;
use std::process::{Child, Command, Stdio};
use std::sync::Mutex;
use std::time::Duration;
use tauri::{AppHandle, Manager};

fn ollama() -> SocketAddr {
    ([127, 0, 0, 1], 11434).into()
}

/// تنها کارهایی که مدل می‌تواند پیشنهاد دهد
const ACTIONS: &[&str] = &[
    "none",
    "smart_connect",
    "disconnect",
    "update_subscriptions",
    "run_diagnostics",
    "open_profiles",
    "open_tutorials",
    "open_tools",
];

#[derive(Default)]
pub struct AiState {
    pull: Mutex<Option<Child>>,
}

impl AiState {
    pub fn kill(&self) {
        if let Ok(mut p) = self.pull.lock() {
            if let Some(mut c) = p.take() {
                let _ = c.kill();
            }
        }
    }
}

fn valid_model(m: &str) -> Result<(), String> {
    let ok = !m.is_empty()
        && m.len() <= 80
        && !m.starts_with('-')
        && m.chars().all(|c| c.is_ascii_alphanumeric() || "._:-/".contains(c));
    if ok { Ok(()) } else { Err("نام مدل نامعتبر است".into()) }
}

#[tauri::command]
pub async fn ai_status() -> Result<Value, String> {
    tools::blocking(|| match localhttp::request(ollama(), "GET", "/api/tags", None, Duration::from_secs(5)) {
        Ok((200, body)) => {
            let v: Value = serde_json::from_str(&body).unwrap_or_default();
            let models: Vec<String> = v["models"]
                .as_array()
                .map(|a| a.iter().filter_map(|m| m["name"].as_str().map(str::to_string)).collect())
                .unwrap_or_default();
            json!({ "running": true, "models": models })
        }
        _ => json!({ "running": false, "models": [] }),
    })
    .await
}

/// دانلود مدل (چند گیگابایت)؛ فقط با کلیک کاربر
#[tauri::command]
pub fn ai_pull(app: AppHandle, model: String) -> Result<(), String> {
    valid_model(&model)?;
    let state = app.state::<AiState>();
    let mut slot = state.pull.lock().map_err(|e| e.to_string())?;
    if slot.as_mut().is_some_and(|c| matches!(c.try_wait(), Ok(None))) {
        return Err("دانلود دیگری در جریان است".into());
    }
    let out = std::fs::File::create(tools::log_path(&app, "ai-pull")?).map_err(|e| e.to_string())?;
    let err = out.try_clone().map_err(|e| e.to_string())?;
    let mut cmd = Command::new("ollama");
    cmd.args(["pull", &model]).stdin(Stdio::null()).stdout(out).stderr(err);
    tools::hide_window(&mut cmd);
    *slot = Some(cmd.spawn().map_err(|e| format!("برنامه‌ی ollama پیدا نشد: {e}"))?);
    store::log(&app, "ai-pull", &model, true, "started");
    Ok(())
}

/// حذف کدهای رنگ/مکان‌نمای ترمینال
fn strip_ansi(s: &str) -> String {
    let mut out = String::new();
    let mut it = s.chars();
    while let Some(c) = it.next() {
        if c == '\u{1b}' {
            for n in it.by_ref() {
                if n.is_ascii_alphabetic() {
                    break;
                }
            }
        } else if c == '\r' {
            out.push('\n');
        } else {
            out.push(c);
        }
    }
    out
}

#[tauri::command]
pub fn ai_pull_status(app: AppHandle) -> Result<Value, String> {
    let state = app.state::<AiState>();
    let mut slot = state.pull.lock().map_err(|e| e.to_string())?;
    let (running, ok) = match slot.as_mut() {
        Some(c) => match c.try_wait() {
            Ok(None) => (true, None),
            Ok(Some(s)) => (false, Some(s.success())),
            Err(_) => (false, Some(false)),
        },
        None => (false, None),
    };
    let text = strip_ansi(&tools::tail(&tools::log_path(&app, "ai-pull")?, 3000));
    let line = text.lines().rev().find(|l| !l.trim().is_empty()).unwrap_or("").trim().to_string();
    Ok(json!({ "running": running, "ok": ok, "line": line }))
}

/// خلاصه‌ی وضعیت بدون هیچ داده‌ی حساس
fn safe_context(app: &AppHandle) -> String {
    let profiles: Vec<store::StoredProfile> = store::load(app, "profiles.json");
    let list = tools::tool_list(app.clone());
    let core = list.iter().find(|t| t["id"] == "sing-box");
    let core_state = match core {
        Some(t) if t["consented"] == true => "تنظیم و تأیید شده",
        Some(t) if !t["setup"].is_null() => "تنظیم شده ولی هشدارها تأیید نشده",
        _ => "تنظیم نشده",
    };
    let mut s = format!("هسته‌ی sing-box: {core_state}\nتعداد پروفایل‌ها: {}\n", profiles.len());
    for (i, p) in profiles.iter().take(20).enumerate() {
        let get = |k: &str| p.params.get(k).map(String::as_str).unwrap_or("-");
        s.push_str(&format!(
            "- پروفایل {}: {} ، security={} ، transport={}\n",
            i + 1,
            p.protocol,
            get("security"),
            get("type"),
        ));
    }
    let status = auto::auto_status(app.clone());
    s.push_str(&format!("وضعیت اتصال: {}\nپروکسی سیستم توسط برنامه: {}\n", status["phase"].as_str().unwrap_or("?"), status["systemProxy"]));
    if let Some(c) = status["connected"].as_object() {
        s.push_str(&format!("تأخیر تونل: {} ms ، حالت: {} ، سالم: {}\n", c["latencyMs"], c["tweak"], c["healthy"]));
    }
    // گام‌های اخیر؛ نام پروفایل‌ها بر اساس ساختار خط حذف می‌شود
    if let Some(steps) = status["steps"].as_array() {
        for line in steps.iter().rev().take(40).rev().filter_map(Value::as_str) {
            s.push_str(&format!("رویداد: {}\n", redact_step(line)));
        }
    }
    for w in crate::netenv::scan(app) {
        s.push_str(&format!("محیط: {}\n", netenv::scrub_addresses(&w)));
    }
    s
}

/// فقط بخش بی‌خطر هر رویداد می‌ماند. خط‌های ناشناخته‌ی حاوی نام، کلاً خلاصه می‌شوند.
fn redact_step(line: &str) -> String {
    // خطای خام هسته ممکن است نشانی سرور داشته باشد
    let line = match line.find("sing-box اجرا نشد") {
        Some(i) => format!("{}sing-box اجرا نشد", &line[..i]),
        None => line.to_string(),
    };
    for mark in ["✓", "✗", "⊘"] {
        if let Some(rest) = line.strip_prefix(mark) {
            // قالب: «✗ [شماره] نام (حالت) — خطا»؛ نام حذف می‌شود، شماره می‌ماند
            let who = rest
                .trim_start()
                .strip_prefix('[')
                .and_then(|r| r.split_once(']'))
                .map(|(n, _)| format!("پروفایل {}", n.trim()))
                .unwrap_or_else(|| "یک پروفایل".into());
            let tail = line.rsplit_once(" — ").map(|x| x.1).unwrap_or("");
            return format!("{mark} {who} — {}", netenv::scrub_addresses(tail));
        }
    }
    if line.starts_with("اتصال برقرار شد") {
        return "اتصال برقرار شد".into();
    }
    if line.starts_with("اتصال «") {
        return "اتصال قطع شد؛ جابه‌جایی خودکار".into();
    }
    netenv::scrub_addresses(&line)
}

const SYSTEM: &str = "تو دستیار برنامه‌ی NetDesk هستی (برنامه‌ی دسکتاپ عیب‌یابی شبکه و اتصال پروکسی). همیشه کوتاه، ساده و به فارسی جواب بده.
برای کار کردن اتصال: ۱) هسته‌ی sing-box باید طبق آموزش نصب و در «ابزارهای پرخطر» تأیید شود. ۲) کاربر باید پروفایل یا اشتراک خودش را وارد کند؛ برنامه سرور رایگان نمی‌سازد. ۳) «اتصال هوشمند» بقیه را خودکار انجام می‌دهد.
در فیلد action فقط وقتی کاربر کاری می‌خواهد یا برای حل مشکل لازم است یکی از این‌ها را بگذار، وگرنه none:
smart_connect = پیدا کردن بهترین پروفایل و اتصال
disconnect = قطع اتصال
update_subscriptions = تازه کردن اشتراک‌ها
run_diagnostics = تست DNS و دسترسی اینترنت
open_profiles = رفتن به صفحه‌ی پروفایل‌ها
open_tutorials = رفتن به آموزش‌ها
open_tools = رفتن به تنظیم هسته و ابزارها
درباره‌ی چیزی که در وضعیت نیامده حدس نزن. اگر کاری انجام نشده، نگو انجام شد.";

#[derive(Debug, Deserialize)]
pub struct Message {
    role: String,
    content: String,
}

#[tauri::command]
pub async fn ai_chat(app: AppHandle, model: String, messages: Vec<Message>) -> Result<Value, String> {
    valid_model(&model)?;
    tools::blocking(move || -> Result<Value, String> {
        let mut list = vec![json!({ "role": "system", "content": format!("{SYSTEM}\n\nوضعیت فعلی:\n{}", safe_context(&app)) })];
        let skip = messages.len().saturating_sub(12);
        for m in messages.into_iter().skip(skip) {
            let role = if m.role == "assistant" { "assistant" } else { "user" };
            list.push(json!({ "role": role, "content": m.content.chars().take(6000).collect::<String>() }));
        }
        let body = json!({
            "model": model,
            "stream": false,
            "messages": list,
            "format": {
                "type": "object",
                "properties": { "reply": { "type": "string" }, "action": { "type": "string", "enum": ACTIONS } },
                "required": ["reply", "action"]
            },
            "options": { "temperature": 0.2, "num_ctx": 4096 }
        });
        let (code, text) = localhttp::request(ollama(), "POST", "/api/chat", Some(&body.to_string()), Duration::from_secs(300))
            .map_err(|e| format!("Ollama در دسترس نیست: {e}"))?;
        let v: Value = serde_json::from_str(&text).map_err(|_| "پاسخ نامعتبر از Ollama".to_string())?;
        if code != 200 {
            return Err(format!("Ollama: {}", v["error"].as_str().unwrap_or("خطای ناشناخته")));
        }
        let content = v["message"]["content"].as_str().unwrap_or("");
        let parsed: Value = serde_json::from_str(content).unwrap_or_else(|_| json!({ "reply": content, "action": "none" }));
        let action = parsed["action"].as_str().filter(|a| ACTIONS.contains(a)).unwrap_or("none");
        Ok(json!({ "reply": parsed["reply"].as_str().unwrap_or(content), "action": action }))
    })
    .await?
}

/// متن آماده و بدون داده‌ی حساس برای پرسیدن از هر هوش مصنوعی دیگر
#[tauri::command]
pub fn ai_report(app: AppHandle) -> String {
    format!(
        "من از برنامه‌ی NetDesk (کلاینت پروکسی مبتنی بر sing-box) استفاده می‌کنم. با وضعیت زیر کمکم کن مشکل اتصال را پیدا کنم. (نام و نشانی سرورها و رمزها عمداً حذف شده‌اند.)\n\n{}",
        safe_context(&app)
    )
}
