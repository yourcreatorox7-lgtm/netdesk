//! کلاینت HTTP بسیار کوچک که **فقط** به آدرس loopback وصل می‌شود.
//! برای گفتگو با مدل هوش مصنوعی محلی (Ollama) استفاده می‌شود؛ چون هر آدرس
//! غیر از 127.0.0.1/::1 رد می‌شود، داده نمی‌تواند از دستگاه خارج شود.
//! HTTP/1.0 استفاده می‌شود تا سرور پاسخ chunked ندهد.

use std::io::{Read, Write};
use std::net::{SocketAddr, TcpStream};
use std::time::Duration;

const MAX_RESPONSE: u64 = 4 << 20;

/// خروجی: (کد وضعیت، بدنه)
pub fn request(addr: SocketAddr, method: &str, path: &str, body: Option<&str>, timeout: Duration) -> Result<(u16, String), String> {
    if !addr.ip().is_loopback() {
        return Err("فقط آدرس محلی (loopback) مجاز است".into());
    }
    if !path.starts_with('/') || [method, path].iter().any(|x| x.contains(['\r', '\n', ' '])) {
        return Err("درخواست نامعتبر است".into());
    }
    let mut s = TcpStream::connect_timeout(&addr, Duration::from_secs(3)).map_err(|e| format!("اتصال به {addr} ممکن نشد: {e}"))?;
    s.set_read_timeout(Some(timeout)).ok();
    s.set_write_timeout(Some(timeout)).ok();
    let body = body.unwrap_or("");
    let head = format!(
        "{method} {path} HTTP/1.0\r\nHost: {addr}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
        body.len()
    );
    s.write_all(head.as_bytes()).map_err(|e| e.to_string())?;
    s.write_all(body.as_bytes()).map_err(|e| e.to_string())?;
    let mut raw = Vec::new();
    s.take(MAX_RESPONSE).read_to_end(&mut raw).map_err(|e| format!("خواندن پاسخ ناموفق بود: {e}"))?;
    let split = raw.windows(4).position(|w| w == b"\r\n\r\n").ok_or("پاسخ HTTP ناقص است")?;
    let status = String::from_utf8_lossy(&raw[..split])
        .split_whitespace()
        .nth(1)
        .and_then(|c| c.parse().ok())
        .ok_or("خط وضعیت HTTP نامعتبر است")?;
    Ok((status, String::from_utf8_lossy(&raw[split + 4..]).into_owned()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::TcpListener;

    #[test]
    fn round_trip_on_loopback() {
        let l = TcpListener::bind("127.0.0.1:0").unwrap();
        let a = l.local_addr().unwrap();
        std::thread::spawn(move || {
            let (mut s, _) = l.accept().unwrap();
            let mut seen = Vec::new();
            let mut b = [0u8; 512];
            while !seen.ends_with(b"{\"a\":1}") {
                let n = s.read(&mut b).unwrap();
                if n == 0 {
                    break;
                }
                seen.extend_from_slice(&b[..n]);
            }
            s.write_all(b"HTTP/1.0 200 OK\r\nContent-Type: application/json\r\n\r\n{\"ok\":true}").unwrap();
        });
        let (code, body) = request(a, "POST", "/api/x", Some("{\"a\":1}"), Duration::from_secs(2)).unwrap();
        assert_eq!((code, body.as_str()), (200, "{\"ok\":true}"));
    }

    #[test]
    fn refuses_remote_and_injection() {
        let t = Duration::from_millis(100);
        assert!(request("93.184.215.14:80".parse().unwrap(), "GET", "/", None, t).is_err());
        assert!(request("127.0.0.1:1".parse().unwrap(), "GET", "/a\r\nX: y", None, t).is_err());
    }
}
