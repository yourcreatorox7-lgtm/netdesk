//! ساخت کانفیگ sing-box از یک `Profile`، برای اتصال واقعی با هسته‌ی خارجی.
//!
//! این ماژول فقط متن JSON می‌سازد و هیچ اتصالی برقرار نمی‌کند. خروجی یک
//! ورودی «mixed» (SOCKS5 + HTTP) فقط روی 127.0.0.1 باز می‌کند تا دستگاه‌های
//! دیگر شبکه نتوانند از آن استفاده کنند. باینری sing-box را کاربر خودش
//! نصب می‌کند و برنامه هش آن را سنجاق می‌کند (ماژول `tools`).

use crate::config::{Profile, Protocol};
use crate::json::Json;
use std::collections::BTreeMap;

fn obj(pairs: Vec<(&str, Json)>) -> Json {
    Json::Obj(pairs.into_iter().map(|(k, v)| (k.to_string(), v)).collect())
}
fn s(v: &str) -> Json {
    Json::Str(v.to_string())
}
fn n(v: u64) -> Json {
    Json::Num(v as f64)
}
fn truthy(v: Option<&String>) -> bool {
    matches!(v.map(|x| x.as_str()), Some("1" | "true" | "True"))
}

fn tls_block(p: &Profile, force: bool) -> Option<Json> {
    let q = &p.params;
    let sec = q.get("security").or(q.get("tls")).map(|x| x.as_str()).unwrap_or("");
    let reality = sec == "reality";
    if !(force || reality || sec == "tls" || sec == "xtls" || p.protocol == Protocol::Https) {
        return None;
    }
    let mut m: BTreeMap<String, Json> = BTreeMap::new();
    m.insert("enabled".into(), Json::Bool(true));
    let sni = q.get("sni").or(q.get("peer")).filter(|x| !x.is_empty()).cloned().unwrap_or_else(|| p.host.clone());
    m.insert("server_name".into(), Json::Str(sni));
    if truthy(q.get("allowInsecure")) || truthy(q.get("insecure")) {
        m.insert("insecure".into(), Json::Bool(true));
    }
    if let Some(a) = q.get("alpn").filter(|x| !x.is_empty()) {
        m.insert("alpn".into(), Json::Arr(a.split(',').map(|x| s(x.trim())).collect()));
    }
    let fp = q.get("fp").filter(|x| !x.is_empty()).cloned().or_else(|| reality.then(|| "chrome".to_string()));
    if let Some(fp) = fp {
        m.insert("utls".into(), obj(vec![("enabled", Json::Bool(true)), ("fingerprint", Json::Str(fp))]));
    }
    if reality {
        let mut r = vec![("enabled", Json::Bool(true)), ("public_key", s(q.get("pbk").map(|x| x.as_str()).unwrap_or("")))];
        if let Some(sid) = q.get("sid") {
            r.push(("short_id", s(sid)));
        }
        m.insert("reality".into(), obj(r));
    }
    Some(Json::Obj(m))
}

fn transport_block(p: &Profile) -> Result<Option<Json>, String> {
    let q = &p.params;
    let kind = q.get("type").or(q.get("net")).map(|x| x.as_str()).unwrap_or("tcp");
    let path = q.get("path").cloned().unwrap_or_else(|| "/".into());
    let host = q.get("host").filter(|x| !x.is_empty()).cloned();
    Ok(match kind {
        "" | "tcp" | "raw" => {
            if q.get("headerType").map(|x| x == "http").unwrap_or(false) {
                return Err("TCP با headerType=http در sing-box پشتیبانی نمی‌شود".into());
            }
            None
        }
        "ws" => {
            let mut v = vec![("type", s("ws")), ("path", Json::Str(path))];
            if let Some(h) = host {
                v.push(("headers", obj(vec![("Host", Json::Str(h))])));
            }
            Some(obj(v))
        }
        "grpc" => Some(obj(vec![("type", s("grpc")), ("service_name", s(q.get("serviceName").map(|x| x.as_str()).unwrap_or("")))])),
        "http" | "h2" => {
            let mut v = vec![("type", s("http")), ("path", Json::Str(path))];
            if let Some(h) = host {
                v.push(("host", Json::Arr(h.split(',').map(|x| s(x.trim())).collect())));
            }
            Some(obj(v))
        }
        "httpupgrade" => {
            let mut v = vec![("type", s("httpupgrade")), ("path", Json::Str(path))];
            if let Some(h) = host {
                v.push(("host", Json::Str(h)));
            }
            Some(obj(v))
        }
        other => return Err(format!("ترابری «{other}» در sing-box پشتیبانی نمی‌شود")),
    })
}

/// خروجی (outbound) sing-box برای یک پروفایل
pub fn outbound(p: &Profile) -> Result<Json, String> {
    let secret = p.secret.clone().unwrap_or_default();
    let q = &p.params;
    let mut m: BTreeMap<String, Json> = BTreeMap::new();
    m.insert("tag".into(), s("proxy"));
    m.insert("server".into(), Json::Str(p.host.clone()));
    m.insert("server_port".into(), n(p.port as u64));
    let need_secret = |what: &str| -> Result<(), String> {
        if secret.is_empty() { Err(format!("{what} در پروفایل نیست")) } else { Ok(()) }
    };
    let (kind, tls_forced, transport) = match p.protocol {
        Protocol::Vless => {
            need_secret("UUID")?;
            m.insert("uuid".into(), Json::Str(secret.clone()));
            if let Some(f) = q.get("flow").filter(|x| !x.is_empty()) {
                m.insert("flow".into(), Json::Str(f.clone()));
            }
            ("vless", false, true)
        }
        Protocol::Vmess => {
            need_secret("UUID")?;
            m.insert("uuid".into(), Json::Str(secret.clone()));
            m.insert("security".into(), s(q.get("scy").filter(|x| !x.is_empty()).map(|x| x.as_str()).unwrap_or("auto")));
            m.insert("alter_id".into(), n(q.get("aid").and_then(|x| x.parse().ok()).unwrap_or(0)));
            ("vmess", false, true)
        }
        Protocol::Trojan => {
            need_secret("رمز")?;
            m.insert("password".into(), Json::Str(secret.clone()));
            // trojan بدون TLS معنا ندارد؛ اگر security=none صریح نباشد TLS روشن است
            ("trojan", q.get("security").map(|x| x != "none").unwrap_or(true), true)
        }
        Protocol::Shadowsocks => {
            need_secret("رمز")?;
            m.insert("method".into(), s(q.get("method").map(|x| x.as_str()).unwrap_or("")));
            m.insert("password".into(), Json::Str(secret.clone()));
            ("shadowsocks", false, false)
        }
        Protocol::Hysteria2 => {
            need_secret("رمز")?;
            m.insert("password".into(), Json::Str(secret.clone()));
            if let Some(o) = q.get("obfs").filter(|x| !x.is_empty() && x.as_str() != "none") {
                let pw = q.get("obfs-password").cloned().unwrap_or_default();
                m.insert("obfs".into(), obj(vec![("type", Json::Str(o.clone())), ("password", Json::Str(pw))]));
            }
            ("hysteria2", true, false)
        }
        Protocol::Tuic => {
            m.insert("uuid".into(), Json::Str(p.username.clone().ok_or("UUID در پروفایل TUIC نیست")?));
            m.insert("password".into(), Json::Str(secret.clone()));
            if let Some(c) = q.get("congestion_control").filter(|x| !x.is_empty()) {
                m.insert("congestion_control".into(), Json::Str(c.clone()));
            }
            ("tuic", true, false)
        }
        Protocol::Socks5 => {
            m.insert("version".into(), s("5"));
            if let Some(u) = &p.username {
                m.insert("username".into(), Json::Str(u.clone()));
                m.insert("password".into(), Json::Str(secret.clone()));
            }
            ("socks", false, false)
        }
        Protocol::Http | Protocol::Https => {
            if let Some(u) = &p.username {
                m.insert("username".into(), Json::Str(u.clone()));
                m.insert("password".into(), Json::Str(secret.clone()));
            }
            ("http", p.protocol == Protocol::Https, false)
        }
    };
    m.insert("type".into(), s(kind));
    if let Some(t) = tls_block(p, tls_forced) {
        m.insert("tls".into(), t);
    }
    if transport {
        if let Some(t) = transport_block(p)? {
            m.insert("transport".into(), t);
        }
    }
    Ok(Json::Obj(m))
}

/// ترفند ضد فیلتر روی دست‌دهی TLS؛ اگر اتصال معمولی کار نکند امتحان می‌شود.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tweak {
    None,
    /// تقسیم ClientHello به چند رکورد TLS
    RecordFragment,
    /// تقسیم ClientHello به چند بسته‌ی TCP
    Fragment,
}

impl Tweak {
    pub fn label(&self) -> &'static str {
        match self {
            Tweak::None => "معمولی",
            Tweak::RecordFragment => "TLS record fragment",
            Tweak::Fragment => "TCP fragment",
        }
    }
}

/// ترتیب حالت‌هایی که برای این پروفایل معنا دارند. fragment فقط برای TLS روی TCP است.
pub fn tweaks_for(p: &Profile) -> Vec<Tweak> {
    let quic = matches!(p.protocol, Protocol::Hysteria2 | Protocol::Tuic);
    let has_tls = outbound(p).map(|o| o.get("tls").is_some()).unwrap_or(false);
    if has_tls && !quic {
        vec![Tweak::None, Tweak::RecordFragment, Tweak::Fragment]
    } else {
        vec![Tweak::None]
    }
}

/// کانفیگ کامل: ورودی mixed روی 127.0.0.1:listen_port → خروجی پروفایل.
pub fn build_config(p: &Profile, listen_port: u16) -> Result<String, String> {
    build_config_with(p, listen_port, Tweak::None)
}

pub fn build_config_with(p: &Profile, listen_port: u16, tweak: Tweak) -> Result<String, String> {
    if listen_port < 1024 {
        return Err("پورت محلی باید ۱۰۲۴ یا بیشتر باشد".into());
    }
    let inbound = obj(vec![("type", s("mixed")), ("tag", s("in")), ("listen", s("127.0.0.1")), ("listen_port", n(listen_port as u64))]);
    let mut proxy_out = outbound(p)?;
    let key = match tweak {
        Tweak::None => None,
        Tweak::RecordFragment => Some("record_fragment"),
        Tweak::Fragment => Some("fragment"),
    };
    if let (Some(key), Json::Obj(o)) = (key, &mut proxy_out) {
        if let Some(Json::Obj(tls)) = o.get_mut("tls") {
            tls.insert(key.to_string(), Json::Bool(true));
        }
    }
    let root = obj(vec![
        ("log", obj(vec![("level", s("warn")), ("timestamp", Json::Bool(true))])),
        ("inbounds", Json::Arr(vec![inbound])),
        ("outbounds", Json::Arr(vec![proxy_out, obj(vec![("type", s("direct")), ("tag", s("direct"))])])),
        ("route", obj(vec![("final", s("proxy"))])),
    ]);
    Ok(root.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::parse_line;
    use crate::json;

    fn cfg(line: &str) -> Json {
        json::parse(&build_config(&parse_line(line).unwrap(), 10808).unwrap()).unwrap()
    }

    #[test]
    fn vless_reality() {
        let j = cfg("vless://11111111-2222-3333-4444-555555555555@de.example.net:443?security=reality&sni=www.microsoft.com&pbk=PUBKEY&sid=ab12&fp=firefox&type=grpc&serviceName=gun&flow=xtls-rprx-vision#DE");
        let o = &j.get("outbounds").unwrap();
        let Json::Arr(a) = o else { panic!() };
        let ob = &a[0];
        assert_eq!(ob.get("type").unwrap().as_text().unwrap(), "vless");
        assert_eq!(ob.get("uuid").unwrap().as_text().unwrap(), "11111111-2222-3333-4444-555555555555");
        assert_eq!(ob.get("flow").unwrap().as_text().unwrap(), "xtls-rprx-vision");
        let tls = ob.get("tls").unwrap();
        assert_eq!(tls.get("server_name").unwrap().as_text().unwrap(), "www.microsoft.com");
        assert_eq!(tls.get("reality").unwrap().get("public_key").unwrap().as_text().unwrap(), "PUBKEY");
        assert_eq!(tls.get("utls").unwrap().get("fingerprint").unwrap().as_text().unwrap(), "firefox");
        assert_eq!(ob.get("transport").unwrap().get("service_name").unwrap().as_text().unwrap(), "gun");
        let Json::Arr(ins) = j.get("inbounds").unwrap() else { panic!() };
        assert_eq!(ins[0].get("listen").unwrap().as_text().unwrap(), "127.0.0.1");
        assert_eq!(ins[0].get("listen_port").unwrap().as_text().unwrap(), "10808");
    }

    #[test]
    fn trojan_ws_and_ss() {
        let j = cfg("trojan://pass@t.example.com:443?type=ws&path=%2Fws&host=cdn.example.com#T");
        let Json::Arr(a) = j.get("outbounds").unwrap() else { panic!() };
        assert_eq!(a[0].get("password").unwrap().as_text().unwrap(), "pass");
        assert!(a[0].get("tls").is_some(), "trojan باید TLS پیش‌فرض داشته باشد");
        let tr = a[0].get("transport").unwrap();
        assert_eq!(tr.get("path").unwrap().as_text().unwrap(), "/ws");
        assert_eq!(tr.get("headers").unwrap().get("Host").unwrap().as_text().unwrap(), "cdn.example.com");

        let j = cfg("ss://Y2hhY2hhMjAtaWV0Zi1wb2x5MTMwNTpzZWNyZXQ@nl.example.org:8388#NL");
        let Json::Arr(a) = j.get("outbounds").unwrap() else { panic!() };
        assert_eq!(a[0].get("method").unwrap().as_text().unwrap(), "chacha20-ietf-poly1305");
        assert!(a[0].get("tls").is_none());
    }

    #[test]
    fn rejects_unsupported_and_low_port() {
        let p = parse_line("vless://11111111-2222-3333-4444-555555555555@h:443?type=kcp").unwrap();
        assert!(build_config(&p, 10808).is_err());
        let p = parse_line("socks5://127.0.0.1:1080").unwrap();
        assert!(build_config(&p, 80).is_err());
        assert!(build_config(&p, 2080).is_ok());
    }

    #[test]
    fn fragment_tweaks() {
        let tls = parse_line("trojan://pw@t.example.com:443#t").unwrap();
        assert_eq!(tweaks_for(&tls), vec![Tweak::None, Tweak::RecordFragment, Tweak::Fragment]);
        let text = build_config_with(&tls, 10808, Tweak::Fragment).unwrap();
        assert!(text.contains("\"fragment\":true"));
        assert!(!build_config(&tls, 10808).unwrap().contains("fragment"));
        let quic = parse_line("hysteria2://pw@h.example.com:443#h").unwrap();
        assert_eq!(tweaks_for(&quic), vec![Tweak::None]);
        let plain = parse_line("socks5://127.0.0.1:1080").unwrap();
        assert_eq!(tweaks_for(&plain), vec![Tweak::None]);
    }

    #[test]
    fn escapes_hostile_values() {
        // مقدار مخرب نباید ساختار JSON را بشکند
        let p = parse_line("trojan://a%22%2C%22x%22%3A%22@h.example:443#n").unwrap();
        let text = build_config(&p, 10808).unwrap();
        let j = json::parse(&text).unwrap();
        let Json::Arr(a) = j.get("outbounds").unwrap() else { panic!() };
        assert_eq!(a[0].get("password").unwrap().as_text().unwrap(), "a\",\"x\":\"");
        assert!(a[0].get("x").is_none());
    }
}
