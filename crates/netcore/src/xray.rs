//! ساخت کانفیگ Xray-core برای پروفایل‌هایی که sing-box اجرا نمی‌کند (فعلاً xhttp).
//!
//! مثل `singbox`، این ماژول فقط متن JSON می‌سازد. ورودی SOCKS5 فقط روی
//! 127.0.0.1 باز می‌شود. نام فیلدها از سورس Xray-core داخل `vendor/Xray-core`
//! (infra/conf/transport_internet.go, transport_security.go, transport_method.go) گرفته شده.

use crate::config::{Profile, Protocol};
use crate::json::{self, Json};
use std::collections::BTreeMap;

fn obj(pairs: Vec<(&str, Json)>) -> Json {
    Json::Obj(pairs.into_iter().map(|(k, v)| (k.to_string(), v)).collect())
}
fn s(v: &str) -> Json {
    Json::Str(v.to_string())
}
fn param<'a>(p: &'a Profile, key: &str) -> Option<&'a str> {
    p.params.get(key).map(|v| v.as_str()).filter(|v| !v.is_empty())
}
fn truthy(v: Option<&str>) -> bool {
    matches!(v, Some("1" | "true" | "True"))
}

fn transport_kind(p: &Profile) -> &str {
    param(p, "type").or(param(p, "net")).unwrap_or("tcp")
}

/// این پروفایل فقط با Xray اجرا می‌شود (VLESS روی xhttp)
pub fn needs_xray(p: &Profile) -> bool {
    p.protocol == Protocol::Vless && matches!(transport_kind(p), "xhttp" | "splithttp")
}

fn security(p: &Profile) -> &str {
    param(p, "security").or(param(p, "tls")).unwrap_or("none")
}

fn stream_settings(p: &Profile) -> Result<Json, String> {
    let mut m: BTreeMap<String, Json> = BTreeMap::new();
    m.insert("network".into(), s("xhttp"));
    let sni = param(p, "sni").or(param(p, "peer")).unwrap_or(&p.host).to_string();
    match security(p) {
        "tls" => {
            if truthy(param(p, "allowInsecure")) || truthy(param(p, "insecure")) {
                return Err("Xray جدید allowInsecure را حذف کرده؛ در لینک باید pcs (هش گواهی) یا vcn (نام گواهی) باشد".into());
            }
            let mut t: BTreeMap<String, Json> = BTreeMap::new();
            t.insert("serverName".into(), Json::Str(sni));
            if let Some(fp) = param(p, "fp") {
                t.insert("fingerprint".into(), s(fp));
            }
            if let Some(a) = param(p, "alpn") {
                t.insert("alpn".into(), Json::Arr(a.split(',').map(|x| s(x.trim())).collect()));
            }
            // pcs = pinnedPeerCertSha256، vcn = verifyPeerCertByName (نام کوتاه در لینک‌های اشتراک)
            if let Some(h) = param(p, "pcs").or(param(p, "pinSHA256")) {
                t.insert("pinnedPeerCertSha256".into(), s(h));
            }
            if let Some(v) = param(p, "vcn") {
                t.insert("verifyPeerCertByName".into(), s(v));
            }
            m.insert("security".into(), s("tls"));
            m.insert("tlsSettings".into(), Json::Obj(t));
        }
        "reality" => {
            let mut r: BTreeMap<String, Json> = BTreeMap::new();
            r.insert("serverName".into(), Json::Str(sni));
            r.insert("fingerprint".into(), s(param(p, "fp").unwrap_or("chrome")));
            r.insert("publicKey".into(), s(param(p, "pbk").ok_or("کلید عمومی Reality (pbk) در پروفایل نیست")?));
            if let Some(v) = param(p, "sid") {
                r.insert("shortId".into(), s(v));
            }
            if let Some(v) = param(p, "spx") {
                r.insert("spiderX".into(), s(v));
            }
            m.insert("security".into(), s("reality"));
            m.insert("realitySettings".into(), Json::Obj(r));
        }
        _ => {
            m.insert("security".into(), s("none"));
        }
    }
    let mut x: BTreeMap<String, Json> = BTreeMap::new();
    x.insert("path".into(), s(param(p, "path").unwrap_or("/")));
    if let Some(h) = param(p, "host") {
        x.insert("host".into(), s(h));
    }
    x.insert("mode".into(), s(param(p, "mode").unwrap_or("auto")));
    if let Some(extra) = param(p, "extra") {
        x.insert("extra".into(), json::parse(extra).map_err(|e| format!("پارامتر extra معتبر نیست: {e}"))?);
    }
    m.insert("xhttpSettings".into(), Json::Obj(x));
    Ok(Json::Obj(m))
}

/// کانفیگ کامل: ورودی SOCKS5 روی 127.0.0.1:listen_port → خروجی VLESS/xhttp
pub fn build_config(p: &Profile, listen_port: u16) -> Result<String, String> {
    if !needs_xray(p) {
        return Err("این پروفایل نیازی به Xray ندارد".into());
    }
    if listen_port < 1024 {
        return Err("پورت محلی باید ۱۰۲۴ یا بیشتر باشد".into());
    }
    let uuid = p.secret.clone().filter(|u| !u.is_empty()).ok_or("UUID در پروفایل نیست")?;
    let mut settings: BTreeMap<String, Json> = BTreeMap::new();
    settings.insert("address".into(), Json::Str(p.host.clone()));
    settings.insert("port".into(), Json::Num(p.port as f64));
    settings.insert("id".into(), Json::Str(uuid));
    settings.insert("encryption".into(), s(param(p, "encryption").unwrap_or("none")));
    if let Some(f) = param(p, "flow") {
        settings.insert("flow".into(), s(f));
    }
    let outbound = obj(vec![
        ("tag", s("proxy")),
        ("protocol", s("vless")),
        ("settings", Json::Obj(settings)),
        ("streamSettings", stream_settings(p)?),
    ]);
    let inbound = obj(vec![
        ("tag", s("in")),
        ("listen", s("127.0.0.1")),
        ("port", Json::Num(listen_port as f64)),
        ("protocol", s("socks")),
        ("settings", obj(vec![("auth", s("noauth")), ("udp", Json::Bool(false))])),
    ]);
    Ok(obj(vec![
        ("log", obj(vec![("loglevel", s("warning"))])),
        ("inbounds", Json::Arr(vec![inbound])),
        ("outbounds", Json::Arr(vec![outbound])),
    ])
    .to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::parse_line;

    const UUID: &str = "11111111-2222-3333-4444-555555555555";

    fn cfg(line: &str) -> Json {
        json::parse(&build_config(&parse_line(line).unwrap(), 10808).unwrap()).unwrap()
    }

    #[test]
    fn xhttp_tls_with_pinning() {
        let pin = "ab".repeat(32);
        let j = cfg(&format!(
            "vless://{UUID}@h.example.net:443?security=tls&type=xhttp&path=%2Fx&host=cdn.example.com&mode=packet-up&sni=front.example.com&fp=chrome&pcs={pin}&vcn=cert.example.com&encryption=none#n"
        ));
        let Json::Arr(outs) = j.get("outbounds").unwrap() else { panic!() };
        let o = &outs[0];
        assert_eq!(o.get("protocol").unwrap().as_text().unwrap(), "vless");
        assert_eq!(o.get("settings").unwrap().get("id").unwrap().as_text().unwrap(), UUID);
        let ss = o.get("streamSettings").unwrap();
        assert_eq!(ss.get("network").unwrap().as_text().unwrap(), "xhttp");
        assert_eq!(ss.get("security").unwrap().as_text().unwrap(), "tls");
        let t = ss.get("tlsSettings").unwrap();
        assert_eq!(t.get("serverName").unwrap().as_text().unwrap(), "front.example.com");
        assert_eq!(t.get("pinnedPeerCertSha256").unwrap().as_text().unwrap(), pin);
        assert_eq!(t.get("verifyPeerCertByName").unwrap().as_text().unwrap(), "cert.example.com");
        let x = ss.get("xhttpSettings").unwrap();
        assert_eq!(x.get("path").unwrap().as_text().unwrap(), "/x");
        assert_eq!(x.get("host").unwrap().as_text().unwrap(), "cdn.example.com");
        assert_eq!(x.get("mode").unwrap().as_text().unwrap(), "packet-up");
        let Json::Arr(ins) = j.get("inbounds").unwrap() else { panic!() };
        assert_eq!(ins[0].get("listen").unwrap().as_text().unwrap(), "127.0.0.1");
        assert_eq!(ins[0].get("protocol").unwrap().as_text().unwrap(), "socks");
    }

    #[test]
    fn xhttp_reality_and_extra() {
        let j = cfg(&format!(
            "vless://{UUID}@h.example.net:443?security=reality&type=xhttp&pbk=PUBKEY&sid=ab12&sni=www.example.com&extra=%7B%22xPaddingBytes%22%3A%22100-1000%22%7D#r"
        ));
        let Json::Arr(outs) = j.get("outbounds").unwrap() else { panic!() };
        let ss = outs[0].get("streamSettings").unwrap();
        let r = ss.get("realitySettings").unwrap();
        assert_eq!(r.get("publicKey").unwrap().as_text().unwrap(), "PUBKEY");
        assert_eq!(r.get("fingerprint").unwrap().as_text().unwrap(), "chrome");
        assert!(ss.get("xhttpSettings").unwrap().get("extra").unwrap().get("xPaddingBytes").is_some());
    }

    #[test]
    fn only_vless_xhttp_needs_xray_and_bad_input_is_rejected() {
        let tcp = parse_line(&format!("vless://{UUID}@h.example.net:443?security=tls#t")).unwrap();
        assert!(!needs_xray(&tcp));
        assert!(build_config(&tcp, 10808).is_err());
        let insecure = parse_line(&format!("vless://{UUID}@h.example.net:443?security=tls&type=xhttp&allowInsecure=1#i")).unwrap();
        assert!(needs_xray(&insecure));
        assert!(build_config(&insecure, 10808).unwrap_err().contains("pcs"));
        let badextra = parse_line(&format!("vless://{UUID}@h.example.net:443?security=tls&type=xhttp&extra=%7Bnot-json#e")).unwrap();
        assert!(build_config(&badextra, 10808).is_err());
        let norealitykey = parse_line(&format!("vless://{UUID}@h.example.net:443?security=reality&type=xhttp#k")).unwrap();
        assert!(build_config(&norealitykey, 10808).is_err());
        let low = parse_line(&format!("vless://{UUID}@h.example.net:443?security=tls&type=xhttp#l")).unwrap();
        assert!(build_config(&low, 80).is_err());
    }
}
