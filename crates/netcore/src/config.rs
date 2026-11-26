//! مدیر کانفیگ جهانی: وارد کردن، اعتبارسنجی، تشخیص تکراری، خروجی.
//!
//! اصل طراحی (بخش ۵ سند): «هر پروتکل فرمت خودش را دارد». برای همین هر
//! قالب تجزیه‌گر جداگانه دارد و خروجی همه به یک `Profile` مشترک با
//! فیلدهای *فراداده* می‌رسد؛ پارامترهای مخصوص پروتکل دست‌نخورده در
//! `params` می‌مانند تا در خروجی چیزی گم نشود.
//!
//! این ماژول هیچ اتصالی برقرار نمی‌کند؛ فقط متن را می‌خواند.

use crate::{b64, json};
use std::collections::{BTreeMap, BTreeSet, HashMap};

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Protocol {
    Vless,
    Vmess,
    Trojan,
    Shadowsocks,
    Hysteria2,
    Tuic,
    Socks5,
    Http,
    Https,
}

impl Protocol {
    pub fn scheme(&self) -> &'static str {
        match self {
            Protocol::Vless => "vless",
            Protocol::Vmess => "vmess",
            Protocol::Trojan => "trojan",
            Protocol::Shadowsocks => "ss",
            Protocol::Hysteria2 => "hysteria2",
            Protocol::Tuic => "tuic",
            Protocol::Socks5 => "socks5",
            Protocol::Http => "http",
            Protocol::Https => "https",
        }
    }
    fn from_scheme(s: &str) -> Option<Self> {
        Some(match s.to_ascii_lowercase().as_str() {
            "vless" => Protocol::Vless,
            "vmess" => Protocol::Vmess,
            "trojan" => Protocol::Trojan,
            "ss" => Protocol::Shadowsocks,
            "hysteria2" | "hy2" => Protocol::Hysteria2,
            "tuic" => Protocol::Tuic,
            "socks" | "socks5" => Protocol::Socks5,
            "http" => Protocol::Http,
            "https" => Protocol::Https,
            _ => return None,
        })
    }
    /// پروکسی استانداردی که سیستم‌عامل/مرورگر مستقیماً پشتیبانی می‌کند
    /// (بدون هسته‌ی خارجی مثل Xray یا sing-box).
    pub fn is_standard_proxy(&self) -> bool {
        matches!(self, Protocol::Socks5 | Protocol::Http | Protocol::Https)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Profile {
    pub protocol: Protocol,
    pub name: String,
    pub host: String,
    pub port: u16,
    /// UUID/رمز/کلید — حساس. در دیسک فقط در «انبار امن» ذخیره می‌شود.
    pub secret: Option<String>,
    pub username: Option<String>,
    /// پارامترهای مخصوص پروتکل (type, security, sni, path, ...)
    pub params: BTreeMap<String, String>,
    // --- فراداده‌ی قابل ویرایش در برنامه ---
    pub folder: Option<String>,
    pub tags: BTreeSet<String>,
    pub favorite: bool,
    /// متن اصلی ورودی (برای خروجی بدون اتلاف). حاوی secret است.
    pub raw: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Severity {
    Error,
    Warning,
}

#[derive(Debug, Clone)]
pub struct Issue {
    pub severity: Severity,
    pub message: String,
}

#[derive(Debug, Clone)]
pub struct ImportLine {
    pub line_no: usize,
    pub result: Result<Profile, String>,
}

#[derive(Debug, Default)]
pub struct ImportReport {
    pub lines: Vec<ImportLine>,
    /// (اندیس اصلی، اندیس تکراری) در `lines`
    pub duplicates: Vec<(usize, usize)>,
    pub was_base64: bool,
}

impl ImportReport {
    pub fn ok_count(&self) -> usize {
        self.lines.iter().filter(|l| l.result.is_ok()).count()
    }
    pub fn profiles(&self) -> impl Iterator<Item = &Profile> {
        self.lines.iter().filter_map(|l| l.result.as_ref().ok())
    }
}

pub const MAX_IMPORT_BYTES: usize = 4 << 20;
pub const MAX_LINES: usize = 20_000;

// ---------------------------------------------------------------- URL ساده

pub fn percent_decode(s: &str) -> String {
    let b = s.as_bytes();
    let mut out = Vec::with_capacity(b.len());
    let mut i = 0;
    while i < b.len() {
        if b[i] == b'%' && i + 2 < b.len() {
            let hex = |c: u8| (c as char).to_digit(16);
            if let (Some(h), Some(l)) = (hex(b[i + 1]), hex(b[i + 2])) {
                out.push((h * 16 + l) as u8);
                i += 3;
                continue;
            }
        }
        // «+» عمداً به فاصله تبدیل نمی‌شود: در Base64 و رمزها معنی دارد.
        out.push(b[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

pub fn percent_encode(s: &str) -> String {
    let mut o = String::new();
    for b in s.bytes() {
        if b.is_ascii_alphanumeric() || b"-._~".contains(&b) {
            o.push(b as char);
        } else {
            o.push_str(&format!("%{b:02X}"));
        }
    }
    o
}

struct Url {
    scheme: String,
    userinfo: Option<String>,
    host: String,
    port: Option<u16>,
    query: BTreeMap<String, String>,
    fragment: Option<String>,
}

fn parse_url(s: &str) -> Result<Url, String> {
    let (scheme, rest) = s.split_once("://").ok_or("«://» پیدا نشد")?;
    let (rest, fragment) = match rest.split_once('#') {
        Some((a, f)) => (a, Some(percent_decode(f))),
        None => (rest, None),
    };
    let (rest, q) = match rest.split_once('?') {
        Some((a, q)) => (a, Some(q)),
        None => (rest, None),
    };
    let rest = rest.trim_end_matches('/');
    let (userinfo, hostport) = match rest.rsplit_once('@') {
        Some((u, h)) => (Some(percent_decode(u)), h),
        None => (None, rest),
    };
    let (host, port) = if let Some(h) = hostport.strip_prefix('[') {
        let (h6, after) = h.split_once(']').ok_or("IPv6 بسته نشده")?;
        (h6.to_string(), after.strip_prefix(':'))
    } else {
        match hostport.rsplit_once(':') {
            Some((h, p)) => (h.to_string(), Some(p)),
            None => (hostport.to_string(), None),
        }
    };
    let port = match port {
        Some(p) if !p.is_empty() => Some(p.parse::<u16>().map_err(|_| format!("پورت نامعتبر: {p}"))?),
        _ => None,
    };
    let mut query = BTreeMap::new();
    if let Some(q) = q {
        for kv in q.split('&').filter(|x| !x.is_empty()) {
            let (k, v) = kv.split_once('=').unwrap_or((kv, ""));
            query.insert(percent_decode(k), percent_decode(v));
        }
    }
    Ok(Url { scheme: scheme.to_string(), userinfo, host, port, query, fragment })
}

// ------------------------------------------------------------- تجزیه‌گرها

fn base_profile(protocol: Protocol, raw: &str) -> Profile {
    Profile {
        protocol,
        name: String::new(),
        host: String::new(),
        port: 0,
        secret: None,
        username: None,
        params: BTreeMap::new(),
        folder: None,
        tags: BTreeSet::new(),
        favorite: false,
        raw: raw.to_string(),
    }
}

fn parse_vmess(raw: &str, body: &str) -> Result<Profile, String> {
    let bytes = b64::decode(body).ok_or("بدنه‌ی vmess Base64 معتبر نیست")?;
    let text = String::from_utf8(bytes).map_err(|_| "بدنه‌ی vmess UTF-8 نیست")?;
    let j = json::parse(&text)?;
    let mut p = base_profile(Protocol::Vmess, raw);
    p.host = j.get("add").and_then(|v| v.as_text()).ok_or("فیلد add ندارد")?;
    p.port = j.get("port").and_then(|v| v.as_text()).ok_or("فیلد port ندارد")?.parse().map_err(|_| "port نامعتبر")?;
    p.secret = j.get("id").and_then(|v| v.as_text());
    p.name = j.get("ps").and_then(|v| v.as_text()).unwrap_or_default();
    for k in ["net", "type", "host", "path", "tls", "sni", "alpn", "fp", "aid", "scy"] {
        if let Some(v) = j.get(k).and_then(|v| v.as_text()) {
            if !v.is_empty() {
                p.params.insert(k.to_string(), v);
            }
        }
    }
    Ok(p)
}

fn parse_ss(raw: &str) -> Result<Profile, String> {
    // SIP002: ss://base64(method:pass)@host:port#name   یا قالب قدیمی ss://base64(method:pass@host:port)#name
    let body = &raw[5..];
    let (main, frag) = body.split_once('#').map(|(a, b)| (a, Some(b))).unwrap_or((body, None));
    let main = main.split_once("?").map(|x| x.0).unwrap_or(main).trim_end_matches('/');
    let mut p = base_profile(Protocol::Shadowsocks, raw);
    let (cred, hostport) = if let Some((u, h)) = main.rsplit_once('@') {
        let u = percent_decode(u);
        let cred = if u.contains(':') { u } else { String::from_utf8(b64::decode(&u).ok_or("userinfo شادوساکس Base64 نیست")?).map_err(|_| "UTF-8")? };
        (cred, h.to_string())
    } else {
        let all = String::from_utf8(b64::decode(main).ok_or("ss قدیمی Base64 نیست")?).map_err(|_| "UTF-8")?;
        let (c, h) = all.rsplit_once('@').ok_or("ss بدون @")?;
        (c.to_string(), h.to_string())
    };
    let (method, pass) = cred.split_once(':').ok_or("method:password پیدا نشد")?;
    let (h, port) = hostport.rsplit_once(':').ok_or("پورت ندارد")?;
    p.host = h.trim_start_matches('[').trim_end_matches(']').to_string();
    p.port = port.parse().map_err(|_| "پورت نامعتبر")?;
    p.params.insert("method".into(), method.to_string());
    p.secret = Some(pass.to_string());
    p.name = frag.map(percent_decode).unwrap_or_default();
    Ok(p)
}

fn parse_generic(raw: &str, protocol: Protocol) -> Result<Profile, String> {
    let u = parse_url(raw)?;
    let mut p = base_profile(protocol.clone(), raw);
    p.host = u.host;
    p.port = u.port.or(match protocol {
        Protocol::Http => Some(80),
        Protocol::Https | Protocol::Trojan | Protocol::Vless | Protocol::Hysteria2 => Some(443),
        Protocol::Socks5 => Some(1080),
        _ => None,
    }).ok_or("پورت مشخص نشده")?;
    p.params = u.query;
    p.name = u.fragment.unwrap_or_default();
    if let Some(ui) = u.userinfo {
        match protocol {
            Protocol::Socks5 | Protocol::Http | Protocol::Https | Protocol::Tuic => {
                let (a, b) = ui.split_once(':').map(|(a, b)| (a.to_string(), Some(b.to_string()))).unwrap_or((ui, None));
                p.username = Some(a);
                p.secret = b;
            }
            _ => p.secret = Some(ui),
        }
    }
    let _ = u.scheme;
    Ok(p)
}

pub fn parse_line(line: &str) -> Result<Profile, String> {
    let line = line.trim();
    if line.len() > 64 * 1024 {
        return Err("خط بیش از حد طولانی است".into());
    }
    let scheme = line.split_once("://").map(|x| x.0).ok_or("قالب ناشناخته (scheme:// ندارد)")?;
    let proto = Protocol::from_scheme(scheme).ok_or_else(|| format!("پروتکل پشتیبانی‌نشده: {scheme}"))?;
    let mut p = match proto {
        Protocol::Vmess => parse_vmess(line, &line[8..])?,
        Protocol::Shadowsocks => parse_ss(line)?,
        other => parse_generic(line, other)?,
    };
    if p.name.trim().is_empty() {
        p.name = format!("{} {}:{}", p.protocol.scheme(), p.host, p.port);
    }
    Ok(p)
}

/// ورودی می‌تواند فهرست خطی یا یک بلوک Base64 (قالب رایج «اشتراک») باشد.
pub fn import_text(text: &str) -> ImportReport {
    let mut rep = ImportReport::default();
    if text.len() > MAX_IMPORT_BYTES {
        rep.lines.push(ImportLine { line_no: 0, result: Err("فایل ورودی بیش از ۴ مگابایت است".into()) });
        return rep;
    }
    let trimmed = text.trim();
    let decoded;
    let body = if !trimmed.contains("://") {
        match b64::decode(trimmed).and_then(|b| String::from_utf8(b).ok()) {
            Some(d) if d.contains("://") => {
                rep.was_base64 = true;
                decoded = d;
                decoded.as_str()
            }
            _ => trimmed,
        }
    } else {
        trimmed
    };
    for (i, line) in body.lines().enumerate().take(MAX_LINES) {
        let l = line.trim();
        if l.is_empty() || l.starts_with('#') || l.starts_with("//") {
            continue;
        }
        rep.lines.push(ImportLine { line_no: i + 1, result: parse_line(l) });
    }
    let mut seen: HashMap<String, usize> = HashMap::new();
    for (idx, l) in rep.lines.iter().enumerate() {
        if let Ok(p) = &l.result {
            let fp = fingerprint(p);
            if let Some(&first) = seen.get(&fp) {
                rep.duplicates.push((first, idx));
            } else {
                seen.insert(fp, idx);
            }
        }
    }
    rep
}

// --------------------------------------------------- تکراری و اعتبارسنجی

/// اثرانگشت تعیین‌کننده برای تشخیص تکراری: نام و فراداده را نادیده می‌گیرد
/// چون دو پروفایل با نام متفاوت ولی همان سرور/کلید، عملاً یکی هستند.
pub fn fingerprint(p: &Profile) -> String {
    let mut s = format!("{}|{}|{}|{}", p.protocol.scheme(), p.host.to_ascii_lowercase(), p.port, p.secret.as_deref().unwrap_or(""));
    for k in ["type", "net", "security", "tls", "sni", "path", "serviceName", "host", "method", "flow", "pbk", "sid"] {
        if let Some(v) = p.params.get(k) {
            s.push('|');
            s.push_str(k);
            s.push('=');
            s.push_str(v);
        }
    }
    // FNV-1a 64 بیتی؛ کوتاه برای نمایش، کافی برای مقایسه‌ی محلی
    let mut h: u64 = 0xcbf29ce484222325;
    for b in s.bytes() {
        h ^= b as u64;
        h = h.wrapping_mul(0x100000001b3);
    }
    format!("{h:016x}")
}

fn looks_like_uuid(s: &str) -> bool {
    let parts: Vec<&str> = s.split('-').collect();
    parts.len() == 5
        && [8, 4, 4, 4, 12].iter().zip(&parts).all(|(n, p)| p.len() == *n && p.bytes().all(|b| b.is_ascii_hexdigit()))
}

pub fn validate(p: &Profile) -> Vec<Issue> {
    let mut v = Vec::new();
    let mut err = |m: String| v.push(Issue { severity: Severity::Error, message: m });
    if p.host.is_empty() {
        err("میزبان خالی است".into());
    }
    if p.port == 0 {
        err("پورت صفر است".into());
    }
    match p.protocol {
        Protocol::Vless | Protocol::Vmess => match &p.secret {
            None => err("UUID وجود ندارد".into()),
            Some(id) if !looks_like_uuid(id) => err(format!("UUID نامعتبر به نظر می‌رسد ({} کاراکتر)", id.len())),
            _ => {}
        },
        Protocol::Trojan | Protocol::Shadowsocks | Protocol::Hysteria2 if p.secret.as_deref().unwrap_or("").is_empty() => {
            err("رمز عبور خالی است".into())
        }
        _ => {}
    }
    let mut warn = |m: String| v.push(Issue { severity: Severity::Warning, message: m });
    let sec = p.params.get("security").or(p.params.get("tls")).map(|s| s.as_str()).unwrap_or("");
    if matches!(p.protocol, Protocol::Vless) && (sec.is_empty() || sec == "none") {
        warn("VLESS بدون TLS/Reality: ترافیک رمزنگاری لایه‌ی انتقال ندارد".into());
    }
    if p.params.get("allowInsecure").map(|x| x == "1" || x == "true").unwrap_or(false)
        || p.params.get("insecure").map(|x| x == "1").unwrap_or(false)
    {
        warn("allowInsecure فعال است: گواهی سرور بررسی نمی‌شود و حمله‌ی MITM ممکن است".into());
    }
    if matches!(p.protocol, Protocol::Http | Protocol::Socks5) && p.secret.is_some() {
        warn("رمز پروکسی HTTP/SOCKS5 بدون TLS به‌صورت متن ساده ارسال می‌شود".into());
    }
    if p.host.parse::<std::net::IpAddr>().map(|ip| match ip {
        std::net::IpAddr::V4(a) => a.is_private() || a.is_loopback(),
        std::net::IpAddr::V6(a) => a.is_loopback(),
    }).unwrap_or(false)
    {
        warn("میزبان آدرس محلی/خصوصی است".into());
    }
    v
}

// ------------------------------------------------------------------ خروجی

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExportMode {
    /// متن اصلی با رمزها — فقط برای پشتیبان رمزنگاری‌شده
    Full,
    /// رمزها حذف می‌شوند — برای اشتراک‌گذاری یا گزارش باگ
    Redacted,
}

pub fn export_line(p: &Profile, mode: ExportMode) -> String {
    match mode {
        ExportMode::Full => p.raw.clone(),
        ExportMode::Redacted => {
            let host = if p.host.contains(':') { format!("[{}]", p.host) } else { p.host.clone() };
            let q: Vec<String> = p.params.iter().filter(|(k, _)| !matches!(k.as_str(), "pbk" | "sid" | "password" | "key"))
                .map(|(k, v)| format!("{}={}", percent_encode(k), percent_encode(v))).collect();
            let q = if q.is_empty() { String::new() } else { format!("?{}", q.join("&")) };
            format!("{}://REDACTED@{}:{}{}#{}", p.protocol.scheme(), host, p.port, q, percent_encode(&p.name))
        }
    }
}

/// فراداده (پوشه، برچسب، علاقه‌مندی) به‌صورت JSON — بدون رمز.
pub fn metadata_json(p: &Profile) -> json::Json {
    use json::Json;
    let mut m = BTreeMap::new();
    m.insert("fingerprint".into(), Json::Str(fingerprint(p)));
    m.insert("protocol".into(), Json::Str(p.protocol.scheme().into()));
    m.insert("name".into(), Json::Str(p.name.clone()));
    m.insert("host".into(), Json::Str(p.host.clone()));
    m.insert("port".into(), Json::Num(p.port as f64));
    m.insert("folder".into(), p.folder.clone().map(Json::Str).unwrap_or(Json::Null));
    m.insert("tags".into(), Json::Arr(p.tags.iter().cloned().map(Json::Str).collect()));
    m.insert("favorite".into(), Json::Bool(p.favorite));
    m.insert("standardProxy".into(), Json::Bool(p.protocol.is_standard_proxy()));
    Json::Obj(m)
}

#[cfg(test)]
mod tests {
    use super::*;

    const UUID: &str = "b831381d-6324-4d53-ad4f-8cda48b30811";

    #[test]
    fn vless_reality() {
        let raw = format!("vless://{UUID}@example.com:443?type=tcp&security=reality&sni=www.microsoft.com&pbk=abc&fp=chrome#%D8%B3%D8%B1%D9%88%D8%B1%201");
        let p = parse_line(&raw).unwrap();
        assert_eq!(p.protocol, Protocol::Vless);
        assert_eq!(p.host, "example.com");
        assert_eq!(p.port, 443);
        assert_eq!(p.name, "سرور 1");
        assert_eq!(p.params["security"], "reality");
        assert!(validate(&p).is_empty());
        let red = export_line(&p, ExportMode::Redacted);
        assert!(!red.contains(UUID) && !red.contains("pbk"));
    }

    #[test]
    fn vmess_b64() {
        let j = format!(r#"{{"v":"2","ps":"test","add":"1.2.3.4","port":"8080","id":"{UUID}","net":"ws","path":"/x","tls":""}}"#);
        let raw = format!("vmess://{}", b64::encode(j.as_bytes()));
        let p = parse_line(&raw).unwrap();
        assert_eq!(p.port, 8080);
        assert_eq!(p.params["net"], "ws");
        assert_eq!(p.secret.as_deref(), Some(UUID));
    }

    #[test]
    fn ss_both_forms() {
        let sip = format!("ss://{}@10.0.0.1:8388#home", b64::encode(b"chacha20-ietf-poly1305:pw"));
        let p = parse_line(&sip).unwrap();
        assert_eq!(p.params["method"], "chacha20-ietf-poly1305");
        assert_eq!(p.secret.as_deref(), Some("pw"));
        assert!(validate(&p).iter().any(|i| i.severity == Severity::Warning));
        let old = format!("ss://{}#x", b64::encode(b"aes-256-gcm:pw@host.net:443"));
        assert_eq!(parse_line(&old).unwrap().host, "host.net");
    }

    #[test]
    fn socks_and_ipv6() {
        let p = parse_line("socks5://user:pass@[2001:db8::1]:1080").unwrap();
        assert_eq!(p.host, "2001:db8::1");
        assert_eq!(p.username.as_deref(), Some("user"));
        assert!(p.protocol.is_standard_proxy());
        assert!(export_line(&p, ExportMode::Redacted).contains("[2001:db8::1]:1080"));
    }

    #[test]
    fn import_subscription_dedup_and_errors() {
        let a = format!("vless://{UUID}@h.com:443?security=tls#A");
        let b = format!("vless://{UUID}@H.com:443?security=tls#B-renamed");
        let text = format!("{a}\n\n# comment\n{b}\nfoo://bar\nnot a url\ntrojan://@x.com:443");
        let sub = b64::encode(text.as_bytes());
        let r = import_text(&sub);
        assert!(r.was_base64);
        assert_eq!(r.lines.len(), 5);
        assert_eq!(r.ok_count(), 3);
        assert_eq!(r.duplicates, vec![(0, 1)]);
        let trojan = r.lines[4].result.as_ref().unwrap();
        assert!(validate(trojan).iter().any(|i| i.severity == Severity::Error));
    }

    #[test]
    fn bad_uuid_and_insecure() {
        let p = parse_line("vless://nope@h.com:443?security=tls&allowInsecure=1").unwrap();
        let issues = validate(&p);
        assert!(issues.iter().any(|i| i.severity == Severity::Error));
        assert!(issues.iter().any(|i| i.message.contains("allowInsecure")));
    }

    #[test]
    fn metadata_has_no_secret() {
        let p = parse_line(&format!("vless://{UUID}@h.com:443#n")).unwrap();
        assert!(!metadata_json(&p).to_string().contains(UUID));
    }
}
