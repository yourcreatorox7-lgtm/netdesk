//! اعتبارسنجی پروکسی استاندارد (HTTP CONNECT و SOCKS5، RFC 1928/1929).
//! فقط «دست‌دهی» را تست می‌کند: آیا پروکسی پاسخ می‌دهد، احراز هویت می‌خواهد،
//! و می‌تواند به مقصد آزمایشی وصل شود. هیچ ترافیکی رهگیری یا رمزگشایی نمی‌شود.

use std::io::{Read, Write};
use std::net::{SocketAddr, TcpStream};
use std::time::{Duration, Instant};

#[derive(Debug, Clone, PartialEq)]
pub enum ProxyVerdict {
    /// پروکسی به مقصد وصل شد
    Works { handshake_ms: f64 },
    AuthRequired,
    AuthFailed,
    /// پروکسی پاسخ داد اما اتصال به مقصد را رد کرد (کد خطا)
    TargetRejected(String),
    NotAProxy(String),
    Unreachable(String),
}

fn io<T>(r: std::io::Result<T>) -> Result<T, ProxyVerdict> {
    r.map_err(|e| ProxyVerdict::Unreachable(e.to_string()))
}

pub fn check_socks5(proxy: SocketAddr, creds: Option<(&str, &str)>, target_host: &str, target_port: u16, timeout: Duration) -> ProxyVerdict {
    let run = || -> Result<ProxyVerdict, ProxyVerdict> {
        let start = Instant::now();
        let mut s = io(TcpStream::connect_timeout(&proxy, timeout))?;
        s.set_read_timeout(Some(timeout)).ok();
        s.set_write_timeout(Some(timeout)).ok();
        let methods: &[u8] = if creds.is_some() { &[0x00, 0x02] } else { &[0x00] };
        let mut hello = vec![0x05, methods.len() as u8];
        hello.extend_from_slice(methods);
        io(s.write_all(&hello))?;
        let mut r = [0u8; 2];
        io(s.read_exact(&mut r))?;
        if r[0] != 0x05 {
            return Ok(ProxyVerdict::NotAProxy(format!("نسخه‌ی پاسخ {:#x} است، نه SOCKS5", r[0])));
        }
        match r[1] {
            0x00 => {}
            0x02 => {
                let Some((u, p)) = creds else { return Ok(ProxyVerdict::AuthRequired) };
                if u.len() > 255 || p.len() > 255 {
                    return Ok(ProxyVerdict::AuthFailed);
                }
                let mut a = vec![0x01, u.len() as u8];
                a.extend_from_slice(u.as_bytes());
                a.push(p.len() as u8);
                a.extend_from_slice(p.as_bytes());
                io(s.write_all(&a))?;
                io(s.read_exact(&mut r))?;
                if r[1] != 0 {
                    return Ok(ProxyVerdict::AuthFailed);
                }
            }
            0xFF => return Ok(ProxyVerdict::AuthRequired),
            m => return Ok(ProxyVerdict::NotAProxy(format!("روش احراز ناشناخته {m:#x}"))),
        }
        if target_host.len() > 255 {
            return Ok(ProxyVerdict::TargetRejected("نام مقصد طولانی است".into()));
        }
        let mut req = vec![0x05, 0x01, 0x00, 0x03, target_host.len() as u8];
        req.extend_from_slice(target_host.as_bytes());
        req.extend_from_slice(&target_port.to_be_bytes());
        io(s.write_all(&req))?;
        let mut head = [0u8; 4];
        io(s.read_exact(&mut head))?;
        if head[1] != 0 {
            let why = match head[1] {
                1 => "خطای عمومی",
                2 => "قانون پروکسی اجازه نمی‌دهد",
                3 => "شبکه در دسترس نیست",
                4 => "میزبان در دسترس نیست",
                5 => "اتصال رد شد",
                6 => "TTL منقضی شد",
                7 => "فرمان پشتیبانی نمی‌شود",
                8 => "نوع آدرس پشتیبانی نمی‌شود",
                _ => "کد ناشناخته",
            };
            return Ok(ProxyVerdict::TargetRejected(format!("{why} (کد {})", head[1])));
        }
        Ok(ProxyVerdict::Works { handshake_ms: start.elapsed().as_secs_f64() * 1000.0 })
    };
    run().unwrap_or_else(|v| v)
}

/// تست سلامت انتها-به-انتها: از داخل تونل SOCKS5 یک درخواست HTTP واقعی می‌فرستد
/// و منتظر پاسخ سرور مقصد می‌ماند. پاسخ موفق CONNECT به‌تنهایی کافی نیست، چون
/// برخی هسته‌ها پیش از اطمینان از درستی UUID/رمز هم آن را می‌دهند.
/// خروجی: زمان رفت‌وبرگشت کامل بر حسب میلی‌ثانیه.
pub fn check_socks5_http(proxy: SocketAddr, host: &str, path: &str, timeout: Duration) -> Result<f64, String> {
    if host.len() > 255 || [host, path].iter().any(|x| x.contains(['\r', '\n', ' '])) {
        return Err("نشانی تست نامعتبر است".into());
    }
    let e = |x: std::io::Error| x.to_string();
    let start = Instant::now();
    let mut s = TcpStream::connect_timeout(&proxy, timeout).map_err(e)?;
    s.set_read_timeout(Some(timeout)).ok();
    s.set_write_timeout(Some(timeout)).ok();
    s.write_all(&[5, 1, 0]).map_err(e)?;
    let mut r = [0u8; 2];
    s.read_exact(&mut r).map_err(e)?;
    if r != [5, 0] {
        return Err("پروکسی محلی SOCKS5 پاسخ درست نداد".into());
    }
    let mut req = vec![5, 1, 0, 3, host.len() as u8];
    req.extend_from_slice(host.as_bytes());
    req.extend_from_slice(&80u16.to_be_bytes());
    s.write_all(&req).map_err(e)?;
    let mut head = [0u8; 4];
    s.read_exact(&mut head).map_err(e)?;
    if head[1] != 0 {
        return Err(format!("تونل برقرار نشد (کد SOCKS {})", head[1]));
    }
    let skip = match head[3] {
        1 => 4 + 2,
        4 => 16 + 2,
        3 => {
            let mut l = [0u8; 1];
            s.read_exact(&mut l).map_err(e)?;
            l[0] as usize + 2
        }
        t => return Err(format!("نوع آدرس ناشناخته در پاسخ SOCKS ({t})")),
    };
    s.read_exact(&mut vec![0u8; skip]).map_err(e)?;
    s.write_all(format!("GET {path} HTTP/1.1\r\nHost: {host}\r\nConnection: close\r\n\r\n").as_bytes()).map_err(e)?;
    let mut status = [0u8; 12];
    s.read_exact(&mut status)
        .map_err(|_| "از سرور پروفایل پاسخی برنگشت (UUID/رمز، SNI یا خود سرور را بررسی کنید)".to_string())?;
    let line = String::from_utf8_lossy(&status).into_owned();
    if line.starts_with("HTTP/") && line.get(9..).is_some_and(|c| c.starts_with(['2', '3'])) {
        Ok(start.elapsed().as_secs_f64() * 1000.0)
    } else {
        Err(format!("پاسخ نامنتظره: {line}"))
    }
}

pub fn check_http_connect(proxy: SocketAddr, basic_auth_b64: Option<&str>, target_host: &str, target_port: u16, timeout: Duration) -> ProxyVerdict {
    let run = || -> Result<ProxyVerdict, ProxyVerdict> {
        if target_host.contains(['\r', '\n', ' ']) {
            return Ok(ProxyVerdict::TargetRejected("نام مقصد کاراکتر غیرمجاز دارد".into()));
        }
        let start = Instant::now();
        let mut s = io(TcpStream::connect_timeout(&proxy, timeout))?;
        s.set_read_timeout(Some(timeout)).ok();
        let mut req = format!("CONNECT {target_host}:{target_port} HTTP/1.1\r\nHost: {target_host}:{target_port}\r\n");
        if let Some(a) = basic_auth_b64 {
            req.push_str(&format!("Proxy-Authorization: Basic {a}\r\n"));
        }
        req.push_str("\r\n");
        io(s.write_all(req.as_bytes()))?;
        let mut buf = [0u8; 512];
        let n = io(s.read(&mut buf))?;
        let line = String::from_utf8_lossy(&buf[..n]).lines().next().unwrap_or("").to_string();
        let code = line.split_whitespace().nth(1).unwrap_or("");
        Ok(match (line.starts_with("HTTP/"), code) {
            (false, _) => ProxyVerdict::NotAProxy(format!("پاسخ HTTP نیست: «{}»", line.chars().take(40).collect::<String>())),
            (true, c) if c.starts_with('2') => ProxyVerdict::Works { handshake_ms: start.elapsed().as_secs_f64() * 1000.0 },
            (true, "407") if basic_auth_b64.is_some() => ProxyVerdict::AuthFailed,
            (true, "407") => ProxyVerdict::AuthRequired,
            (true, c) => ProxyVerdict::TargetRejected(format!("HTTP {c}")),
        })
    };
    run().unwrap_or_else(|v| v)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::TcpListener;

    #[test]
    fn socks5_noauth_ok() {
        let l = TcpListener::bind("127.0.0.1:0").unwrap();
        let a = l.local_addr().unwrap();
        std::thread::spawn(move || {
            let (mut s, _) = l.accept().unwrap();
            let mut b = [0u8; 3];
            s.read_exact(&mut b).unwrap();
            s.write_all(&[5, 0]).unwrap();
            let mut req = [0u8; 5];
            s.read_exact(&mut req).unwrap();
            let mut rest = vec![0u8; req[4] as usize + 2];
            s.read_exact(&mut rest).unwrap();
            s.write_all(&[5, 0, 0, 1, 0, 0, 0, 0, 0, 0]).unwrap();
        });
        assert!(matches!(check_socks5(a, None, "example.com", 443, Duration::from_secs(2)), ProxyVerdict::Works { .. }));
    }

    fn fake_socks(reply_http: bool) -> SocketAddr {
        let l = TcpListener::bind("127.0.0.1:0").unwrap();
        let a = l.local_addr().unwrap();
        std::thread::spawn(move || {
            let (mut s, _) = l.accept().unwrap();
            let mut b = [0u8; 3];
            s.read_exact(&mut b).unwrap();
            s.write_all(&[5, 0]).unwrap();
            let mut req = [0u8; 5];
            s.read_exact(&mut req).unwrap();
            let mut rest = vec![0u8; req[4] as usize + 2];
            s.read_exact(&mut rest).unwrap();
            s.write_all(&[5, 0, 0, 1, 0, 0, 0, 0, 0, 0]).unwrap();
            let mut get = [0u8; 256];
            let _ = s.read(&mut get);
            if reply_http {
                s.write_all(b"HTTP/1.1 204 No Content\r\n\r\n").unwrap();
            }
        });
        a
    }

    #[test]
    fn end_to_end_needs_real_http_reply() {
        let t = Duration::from_secs(2);
        assert!(check_socks5_http(fake_socks(true), "example.com", "/generate_204", t).is_ok());
        // CONNECT موفق ولی بدون پاسخ مقصد = ناموفق
        assert!(check_socks5_http(fake_socks(false), "example.com", "/generate_204", t).is_err());
        assert!(check_socks5_http("127.0.0.1:1".parse().unwrap(), "a b", "/", t).is_err());
    }

    #[test]
    fn http_407() {
        let l = TcpListener::bind("127.0.0.1:0").unwrap();
        let a = l.local_addr().unwrap();
        std::thread::spawn(move || {
            let (mut s, _) = l.accept().unwrap();
            let mut b = [0u8; 256];
            let _ = s.read(&mut b);
            s.write_all(b"HTTP/1.1 407 Proxy Authentication Required\r\n\r\n").unwrap();
        });
        assert_eq!(check_http_connect(a, None, "example.com", 443, Duration::from_secs(2)), ProxyVerdict::AuthRequired);
    }

    #[test]
    fn header_injection_blocked() {
        let v = check_http_connect("127.0.0.1:1".parse().unwrap(), None, "a.com\r\nX: y", 443, Duration::from_millis(100));
        assert!(matches!(v, ProxyVerdict::TargetRejected(_)));
    }
}
