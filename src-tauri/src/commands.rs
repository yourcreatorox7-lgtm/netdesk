//! فرمان‌های IPC. همه‌ی عملیات شبکه‌ای در `spawn_blocking` اجرا می‌شوند
//! تا رابط کاربری قفل نشود.

use crate::store::{self, Server, StoredProfile};
use netcore::{config, dns, probe, proxy, rdp, report, server, stats};
use serde_json::{json, Value};
use std::net::SocketAddr;
use std::time::Duration;
use tauri::AppHandle;

async fn blocking<T: Send + 'static>(f: impl FnOnce() -> T + Send + 'static) -> Result<T, String> {
    tauri::async_runtime::spawn_blocking(f).await.map_err(|e| e.to_string())
}

fn summary_json(s: &stats::Summary) -> Value {
    json!({
        "sent": s.sent, "received": s.received, "lossPct": s.loss_pct,
        "minMs": s.min_ms, "avgMs": s.avg_ms, "medianMs": s.median_ms,
        "p95Ms": s.p95_ms, "maxMs": s.max_ms, "jitterMs": s.jitter_ms,
        "score": stats::quality_score(s),
    })
}

fn health_json(name: &str, h: &dns::ResolverHealth) -> Value {
    json!({
        "name": name,
        "address": h.resolver.to_string(),
        "summary": summary_json(&h.summary),
        "rcode": h.last_rcode.map(|r| format!("{r:?}")),
        "answers": h.last_addresses.iter().map(|a| a.to_string()).collect::<Vec<_>>(),
        "warning": dns::suspicious_answer(&h.last_addresses),
        "errors": h.errors,
        "locality": "publicEndpoint",
    })
}

fn timeout(ms: Option<u64>) -> Duration {
    Duration::from_millis(ms.unwrap_or(2000).clamp(200, 15_000))
}

// ------------------------------------------------------------ DNS / شبکه

#[tauri::command]
pub async fn dns_health(resolver: String, domain: String, attempts: Option<usize>, timeout_ms: Option<u64>) -> Result<Value, String> {
    let addr: SocketAddr = if resolver.contains(':') && !resolver.contains('[') && resolver.matches(':').count() == 1 {
        resolver.parse().map_err(|_| "آدرس resolver نامعتبر")?
    } else {
        let ip: std::net::IpAddr = resolver.trim_matches(['[', ']']).parse().map_err(|_| "آدرس resolver نامعتبر")?;
        SocketAddr::new(ip, 53)
    };
    let n = attempts.unwrap_or(5).clamp(1, 50);
    blocking(move || health_json(&resolver, &dns::health_check(addr, &domain, n, timeout(timeout_ms)))).await
}

/// مقایسه‌ی همزمان همه‌ی resolverهای پیش‌فرض + سفارشی کاربر
#[tauri::command]
pub async fn dns_compare(domain: String, extra: Vec<String>) -> Result<Vec<Value>, String> {
    let mut list: Vec<(String, SocketAddr)> = dns::PUBLIC_RESOLVERS.iter().map(|(n, ip)| (n.to_string(), format!("{ip}:53").parse().unwrap())).collect();
    for e in extra.into_iter().take(20) {
        if let Ok(ip) = e.parse::<std::net::IpAddr>() {
            list.push((e.clone(), SocketAddr::new(ip, 53)));
        }
    }
    blocking(move || {
        let hs: Vec<_> = list.into_iter().map(|(n, a)| {
            let d = domain.clone();
            std::thread::spawn(move || health_json(&n, &dns::health_check(a, &d, 4, Duration::from_secs(2))))
        }).collect();
        let mut out: Vec<Value> = hs.into_iter().filter_map(|h| h.join().ok()).collect();
        out.sort_by(|a, b| {
            let k = |v: &Value| v["summary"]["avgMs"].as_f64().unwrap_or(f64::MAX);
            k(a).partial_cmp(&k(b)).unwrap()
        });
        out
    }).await
}

#[tauri::command]
pub async fn tcp_probe(app: AppHandle, target: String, count: Option<usize>, timeout_ms: Option<u64>) -> Result<Value, String> {
    let n = count.unwrap_or(5).clamp(1, 100);
    let t = target.clone();
    let r = blocking(move || -> Result<Value, String> {
        let addr = probe::resolve(&t)?;
        let r = probe::tcp_series(addr, n, timeout(timeout_ms), Duration::from_millis(200));
        Ok(json!({
            "target": t, "address": addr.to_string(), "portOpen": r.port_open,
            "summary": summary_json(&r.summary),
            "samples": r.outcomes.iter().map(|o| o.rtt()).collect::<Vec<_>>(),
            "locality": "local",
        }))
    }).await??;
    store::log(&app, "tcp", &target, r["portOpen"].as_bool().unwrap_or(false), &r["summary"].to_string());
    Ok(r)
}

/// تست UDP واقعی: پرس‌وجوی DNS به پورت ۵۳ مقصد (تنها روش معنی‌دار بدون سرویس اکو)
#[tauri::command]
pub async fn udp_dns_probe(target: String) -> Result<Value, String> {
    blocking(move || -> Result<Value, String> {
        let addr = probe::resolve(&target)?;
        let q = dns::build_query(0x4e44, "example.com", dns::RecordType::A)?;
        let o = probe::udp_check(addr, &q, Duration::from_secs(2));
        Ok(json!({ "target": target, "outcome": format!("{o:?}") }))
    }).await?
}

#[tauri::command]
pub async fn proxy_check(kind: String, address: String, username: Option<String>, secret_key: Option<String>) -> Result<Value, String> {
    blocking(move || -> Result<Value, String> {
        let addr = probe::resolve(&address)?;
        let pass = secret_key.as_deref().and_then(crate::secrets::read);
        let v = match kind.as_str() {
            "socks5" => {
                let c = username.as_deref().zip(pass.as_deref());
                proxy::check_socks5(addr, c, "example.com", 443, Duration::from_secs(4))
            }
            "http" => {
                let auth = username.zip(pass).map(|(u, p)| netcore::b64::encode(format!("{u}:{p}").as_bytes()));
                proxy::check_http_connect(addr, auth.as_deref(), "example.com", 443, Duration::from_secs(4))
            }
            _ => return Err("نوع پروکسی باید socks5 یا http باشد".into()),
        };
        Ok(json!({ "verdict": format!("{v:?}"), "works": matches!(v, proxy::ProxyVerdict::Works { .. }) }))
    }).await?
}

#[tauri::command]
pub async fn quick_report(domain: Option<String>, format: Option<String>) -> Result<String, String> {
    blocking(move || {
        let res: Vec<(&str, SocketAddr)> = dns::PUBLIC_RESOLVERS.iter().map(|(n, ip)| (*n, format!("{ip}:53").parse().unwrap())).collect();
        let tcp: Vec<(&str, SocketAddr)> = probe::CONNECTIVITY_TARGETS.iter().map(|(n, s)| (*n, s.parse().unwrap())).collect();
        let opts = report::Options { test_domain: domain.unwrap_or_else(|| "example.com".into()), ..Default::default() };
        let r = report::run_quick(&opts, &res, &tcp);
        if format.as_deref() == Some("json") { r.to_json() } else { r.to_markdown() }
    }).await
}

// ---------------------------------------------------------- پروفایل‌ها

#[tauri::command]
pub fn import_profiles(app: AppHandle, text: String, folder: Option<String>) -> Result<Value, String> {
    let rep = config::import_text(&text);
    let mut stored: Vec<StoredProfile> = store::load(&app, "profiles.json");
    let (mut added, mut skipped) = (0, 0);
    let mut lines = Vec::new();
    for l in &rep.lines {
        match &l.result {
            Err(e) => lines.push(json!({ "line": l.line_no, "ok": false, "error": e })),
            Ok(p) => {
                let fp = config::fingerprint(p);
                let issues: Vec<Value> = config::validate(p).iter().map(|i| json!({ "severity": format!("{:?}", i.severity), "message": i.message })).collect();
                let dup = stored.iter().any(|s| s.fingerprint == fp);
                if dup {
                    skipped += 1;
                } else {
                    // متن کامل (که رمز دارد) فقط در keyring؛ فایل JSON فقط فراداده
                    let has_secret = crate::secrets::write(&format!("profile:{fp}"), &p.raw).is_ok();
                    stored.push(StoredProfile {
                        fingerprint: fp.clone(), protocol: p.protocol.scheme().into(), name: p.name.clone(),
                        host: p.host.clone(), port: p.port,
                        params: p.params.iter().filter(|(k, _)| !matches!(k.as_str(), "pbk" | "sid" | "password" | "key")).map(|(k, v)| (k.clone(), v.clone())).collect(),
                        folder: folder.clone(), tags: vec![], favorite: false,
                        standard_proxy: p.protocol.is_standard_proxy(), has_secret, added_at: store::now(),
                    });
                    added += 1;
                }
                lines.push(json!({ "line": l.line_no, "ok": true, "name": p.name, "fingerprint": fp, "duplicate": dup, "issues": issues }));
            }
        }
    }
    store::save(&app, "profiles.json", &stored)?;
    Ok(json!({ "wasBase64": rep.was_base64, "added": added, "skippedDuplicates": skipped, "inFileDuplicates": rep.duplicates.len(), "lines": lines }))
}

#[tauri::command]
pub fn list_profiles(app: AppHandle) -> Vec<StoredProfile> {
    store::load(&app, "profiles.json")
}

#[tauri::command]
pub fn update_profile_meta(app: AppHandle, fingerprint: String, name: Option<String>, folder: Option<String>, tags: Option<Vec<String>>, favorite: Option<bool>) -> Result<(), String> {
    let mut v: Vec<StoredProfile> = store::load(&app, "profiles.json");
    let p = v.iter_mut().find(|p| p.fingerprint == fingerprint).ok_or("پروفایل پیدا نشد")?;
    if let Some(n) = name { p.name = n.chars().take(120).collect(); }
    if folder.is_some() { p.folder = folder.filter(|f| !f.is_empty()); }
    if let Some(t) = tags { p.tags = t.into_iter().take(20).collect(); }
    if let Some(f) = favorite { p.favorite = f; }
    store::save(&app, "profiles.json", &v)
}

#[tauri::command]
pub fn delete_profile(app: AppHandle, fingerprint: String) -> Result<(), String> {
    let mut v: Vec<StoredProfile> = store::load(&app, "profiles.json");
    v.retain(|p| p.fingerprint != fingerprint);
    let _ = crate::secrets::secret_delete(format!("profile:{fingerprint}"));
    store::save(&app, "profiles.json", &v)
}

/// redacted=true: بدون رمز (برای اشتراک). redacted=false: متن کامل از keyring.
#[tauri::command]
pub fn export_profiles(app: AppHandle, fingerprints: Vec<String>, redacted: bool) -> Result<String, String> {
    let v: Vec<StoredProfile> = store::load(&app, "profiles.json");
    let mut out = Vec::new();
    for s in v.iter().filter(|p| fingerprints.is_empty() || fingerprints.contains(&p.fingerprint)) {
        let Some(raw) = crate::secrets::read(&format!("profile:{}", s.fingerprint)) else { continue };
        let Ok(mut p) = config::parse_line(&raw) else { continue };
        p.name = s.name.clone();
        out.push(config::export_line(&p, if redacted { config::ExportMode::Redacted } else { config::ExportMode::Full }));
    }
    Ok(out.join("\n"))
}

// ------------------------------------------------------- سرورها / SSH / RDP

#[tauri::command]
pub fn list_servers(app: AppHandle) -> Vec<Server> {
    store::load(&app, "servers.json")
}

#[tauri::command]
pub fn save_server(app: AppHandle, mut server: Server) -> Result<Server, String> {
    if server.host.trim().is_empty() || server.host.contains(char::is_whitespace) {
        return Err("میزبان نامعتبر".into());
    }
    if server.ssh_user.starts_with('-') || server.host.starts_with('-') {
        return Err("نام کاربر/میزبان نباید با «-» شروع شود".into()); // جلوگیری از تزریق گزینه به ssh
    }
    if server.id.is_empty() {
        server.id = format!("srv-{:x}", store::now() ^ (server.host.len() as u64) << 32);
    }
    let mut v: Vec<Server> = store::load(&app, "servers.json");
    v.retain(|s| s.id != server.id);
    v.push(server.clone());
    store::save(&app, "servers.json", &v)?;
    Ok(server)
}

#[tauri::command]
pub fn delete_server(app: AppHandle, id: String) -> Result<(), String> {
    let mut v: Vec<Server> = store::load(&app, "servers.json");
    v.retain(|s| s.id != id);
    store::save(&app, "servers.json", &v)
}

/// مانیتورینگ با OpenSSH سیستم و احراز با کلید (BatchMode). رمز عبور SSH
/// پشتیبانی نمی‌شود تا هیچ رمزی در آرگومان یا حافظه‌ی پردازه نماند.
/// StrictHostKeyChecking=accept-new: اولین اتصال کلید را ثبت می‌کند و
/// تغییر بعدی کلید (نشانه‌ی MITM) اتصال را متوقف می‌کند.
#[tauri::command]
pub async fn server_metrics(app: AppHandle, id: String) -> Result<Value, String> {
    let s = list_servers(app.clone()).into_iter().find(|s| s.id == id).ok_or("سرور پیدا نشد")?;
    let host = s.host.clone();
    let res = blocking(move || -> Result<server::ServerMetrics, String> {
        let mut cmd = std::process::Command::new("ssh");
        cmd.args(["-o", "BatchMode=yes", "-o", "ConnectTimeout=8", "-o", "StrictHostKeyChecking=accept-new", "-p", &s.ssh_port.to_string()]);
        if let Some(k) = &s.ssh_key_path {
            cmd.args(["-i", k]);
        }
        cmd.arg("--").arg(format!("{}@{}", s.ssh_user, s.host)).arg(server::PROBE_COMMAND);
        let out = cmd.output().map_err(|e| format!("اجرای ssh ممکن نشد (OpenSSH نصب است؟): {e}"))?;
        if !out.status.success() {
            return Err(String::from_utf8_lossy(&out.stderr).trim().chars().take(300).collect());
        }
        server::parse_probe_output(&String::from_utf8_lossy(&out.stdout))
    }).await?;
    store::log(&app, "ssh", &host, res.is_ok(), res.as_ref().err().map(|s| s.as_str()).unwrap_or("ok"));
    let m = res?;
    Ok(json!({
        "cpuPct": m.cpu_pct, "cpus": m.cpus, "load": [m.load1, m.load5, m.load15],
        "memUsedPct": m.mem_used_pct(), "memTotalKb": m.mem_total_kb,
        "diskUsedPct": m.disk_used_pct(), "diskTotalB": m.disk_total_b,
        "uptimeS": m.uptime_s, "health": format!("{:?}", m.health()),
    }))
}

#[tauri::command]
pub fn rdp_launch(app: AppHandle, id: String) -> Result<String, String> {
    let s = list_servers(app.clone()).into_iter().find(|s| s.id == id).ok_or("سرور پیدا نشد")?;
    let t = rdp::RdpTarget { host: s.host.clone(), port: s.rdp_port.unwrap_or(3389), username: s.rdp_user.clone(), ..Default::default() };
    store::log(&app, "rdp", &s.host, true, "launch");
    #[cfg(target_os = "linux")]
    {
        std::process::Command::new("xfreerdp").args(rdp::xfreerdp_args(&t)).spawn().map_err(|e| format!("xfreerdp پیدا نشد: {e}"))?;
        return Ok("xfreerdp".into());
    }
    #[cfg(not(target_os = "linux"))]
    {
        let file = std::env::temp_dir().join(format!("netdesk-{}.rdp", s.id));
        std::fs::write(&file, rdp::to_rdp_file(&t)?).map_err(|e| e.to_string())?;
        #[cfg(target_os = "windows")]
        std::process::Command::new("mstsc").arg(&file).spawn().map_err(|e| e.to_string())?;
        #[cfg(target_os = "macos")]
        std::process::Command::new("open").arg(&file).spawn().map_err(|e| e.to_string())?;
        Ok(file.display().to_string())
    }
}
