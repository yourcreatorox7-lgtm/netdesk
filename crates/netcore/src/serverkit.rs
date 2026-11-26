//! ساخت «کیت سرور»: کانفیگ سرور VLESS + Reality برای سرور **خود کاربر** و
//! لینک کلاینت متناظر. فقط متن می‌سازد؛ کلیدها را هسته‌ی sing-box تولید می‌کند.
//! Reality دامنه و گواهی لازم ندارد.

use crate::config::percent_encode;
use crate::json::Json;

pub struct Keys {
    pub uuid: String,
    pub private_key: String,
    pub public_key: String,
    pub short_id: String,
}

fn obj(pairs: Vec<(&str, Json)>) -> Json {
    Json::Obj(pairs.into_iter().map(|(k, v)| (k.to_string(), v)).collect())
}
fn s(v: &str) -> Json {
    Json::Str(v.to_string())
}

/// IP یا نام دامنه، بدون پورت و مسیر
pub fn valid_host(host: &str) -> Result<(), String> {
    if host.parse::<std::net::IpAddr>().is_ok() {
        return Ok(());
    }
    let ok = !host.is_empty()
        && host.len() <= 253
        && host.contains('.')
        && host.split('.').all(|l| !l.is_empty() && !l.starts_with('-') && l.chars().all(|c| c.is_ascii_alphanumeric() || c == '-'))
        // چیزی که فقط عدد و نقطه است باید IP معتبر باشد
        && host.chars().any(|c| c.is_ascii_alphabetic());
    if ok { Ok(()) } else { Err("آدرس سرور باید فقط IP یا نام دامنه باشد (بدون پورت و http)".into()) }
}

pub fn valid_port(port: u16) -> Result<(), String> {
    match port {
        0 => Err("پورت نامعتبر است".into()),
        3389 => Err("پورت ۳۳۸۹ برای خود Remote Desktop است".into()),
        _ => Ok(()),
    }
}

/// خروجی `sing-box generate reality-keypair`
pub fn parse_keypair(output: &str) -> Option<(String, String)> {
    let get = |name: &str| output.lines().find_map(|l| l.trim().strip_prefix(name)).map(|v| v.trim().to_string()).filter(|v| !v.is_empty());
    Some((get("PrivateKey:")?, get("PublicKey:")?))
}

pub fn server_config(keys: &Keys, port: u16, sni: &str) -> String {
    let inbound = obj(vec![
        ("type", s("vless")),
        ("tag", s("in")),
        ("listen", s("::")),
        ("listen_port", Json::Num(port as f64)),
        ("users", Json::Arr(vec![obj(vec![("uuid", s(&keys.uuid)), ("flow", s("xtls-rprx-vision"))])])),
        (
            "tls",
            obj(vec![
                ("enabled", Json::Bool(true)),
                ("server_name", s(sni)),
                (
                    "reality",
                    obj(vec![
                        ("enabled", Json::Bool(true)),
                        ("handshake", obj(vec![("server", s(sni)), ("server_port", Json::Num(443.0))])),
                        ("private_key", s(&keys.private_key)),
                        ("short_id", Json::Arr(vec![s(&keys.short_id)])),
                    ]),
                ),
            ]),
        ),
    ]);
    // کاربران تونل نباید به خود سرور یا شبکه‌ی خصوصی پشت آن دسترسی داشته باشند
    let rules = Json::Arr(vec![
        obj(vec![("action", s("resolve"))]),
        obj(vec![("ip_is_private", Json::Bool(true)), ("action", s("reject"))]),
    ]);
    obj(vec![
        ("log", obj(vec![("level", s("warn")), ("timestamp", Json::Bool(true))])),
        ("dns", obj(vec![("servers", Json::Arr(vec![obj(vec![("type", s("local")), ("tag", s("local"))])]))])),
        ("inbounds", Json::Arr(vec![inbound])),
        ("outbounds", Json::Arr(vec![obj(vec![("type", s("direct")), ("tag", s("direct"))])])),
        ("route", obj(vec![("rules", rules), ("final", s("direct")), ("default_domain_resolver", s("local"))])),
    ])
    .to_string()
}

pub fn client_link(keys: &Keys, host: &str, port: u16, sni: &str, name: &str) -> String {
    let host = if host.contains(':') { format!("[{host}]") } else { host.to_string() };
    format!(
        "vless://{}@{host}:{port}?security=reality&type=tcp&flow=xtls-rprx-vision&fp=chrome&sni={}&pbk={}&sid={}#{}",
        keys.uuid,
        percent_encode(sni),
        percent_encode(&keys.public_key),
        keys.short_id,
        percent_encode(name),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{config, json, singbox};

    fn keys() -> Keys {
        Keys { uuid: "11111111-2222-3333-4444-555555555555".into(), private_key: "PRIV_k-1".into(), public_key: "PUB_k-2".into(), short_id: "0123456789abcdef".into() }
    }

    #[test]
    fn hosts_and_ports() {
        for good in ["203.0.113.9", "2001:db8::1", "my.server.example"] {
            assert!(valid_host(good).is_ok(), "{good}");
        }
        for bad in ["", "localhost", "999.1.1.1", "1.2.3.4:3389", "http://a.com", "a b.com", "a.com/x", "-a.com", "x\".com"] {
            assert!(valid_host(bad).is_err(), "{bad}");
        }
        assert!(valid_port(443).is_ok());
        assert!(valid_port(0).is_err() && valid_port(3389).is_err());
    }

    #[test]
    fn keypair_output() {
        let (a, b) = parse_keypair("PrivateKey: abc\r\nPublicKey: def\r\n").unwrap();
        assert_eq!((a.as_str(), b.as_str()), ("abc", "def"));
        assert!(parse_keypair("error").is_none());
    }

    #[test]
    fn link_matches_server_and_never_leaks_private_key() {
        let k = keys();
        let link = client_link(&k, "203.0.113.9", 443, "www.example.com", "سرور من");
        assert!(!link.contains(&k.private_key));
        let p = config::parse_line(&link).unwrap();
        assert_eq!(p.name, "سرور من");
        let client = singbox::build_config(&p, 10808).unwrap();
        assert!(client.contains("PUB_k-2") && client.contains("0123456789abcdef") && client.contains("xtls-rprx-vision"));
        let server = server_config(&k, 443, "www.example.com");
        assert!(server.contains("PRIV_k-1") && !server.contains("PUB_k-2"));
        assert!(server.contains("\"ip_is_private\":true"));
        json::parse(&server).unwrap();
        assert!(config::parse_line(&client_link(&k, "2001:db8::1", 8443, "a.com", "v6")).unwrap().host == "2001:db8::1");
    }
}
