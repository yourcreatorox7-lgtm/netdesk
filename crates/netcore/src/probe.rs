//! سنجش اتصال: زمان دست‌دهی TCP، بررسی UDP، و پایش پیوسته‌ی کیفیت.
//! ICMP ping به دسترسی root/raw socket نیاز دارد؛ عمداً از «TCP ping»
//! استفاده می‌کنیم که بدون دسترسی ویژه روی هر سه سیستم‌عامل کار می‌کند.

use crate::stats::{summarize, Summary};
use std::net::{SocketAddr, TcpStream, ToSocketAddrs, UdpSocket};
use std::time::{Duration, Instant};

#[derive(Debug, Clone)]
pub enum TcpOutcome {
    Connected { ms: f64 },
    Refused { ms: f64 },
    Timeout,
    Error(String),
}

impl TcpOutcome {
    /// «Refused» یعنی میزبان در دسترس است اما پورت بسته؛ برای سنجش تأخیر قابل استفاده است.
    pub fn rtt(&self) -> Option<f64> {
        match self {
            TcpOutcome::Connected { ms } | TcpOutcome::Refused { ms } => Some(*ms),
            _ => None,
        }
    }
}

pub fn resolve(target: &str) -> Result<SocketAddr, String> {
    target
        .to_socket_addrs()
        .map_err(|e| format!("resolve «{target}»: {e}"))?
        .next()
        .ok_or_else(|| format!("آدرسی برای «{target}» پیدا نشد"))
}

/// نکته‌ی ویندوز: اتصال به پورت بسته حدود ۲ ثانیه طول می‌کشد تا «Refused» شود
/// (تکرار SYN پس از RST)، پس با timeout زیر ۲ ثانیه ممکن است Timeout گزارش شود.
pub fn tcp_once(addr: SocketAddr, timeout: Duration) -> TcpOutcome {
    let start = Instant::now();
    match TcpStream::connect_timeout(&addr, timeout) {
        Ok(_) => TcpOutcome::Connected { ms: start.elapsed().as_secs_f64() * 1000.0 },
        Err(e) => match e.kind() {
            std::io::ErrorKind::ConnectionRefused => TcpOutcome::Refused { ms: start.elapsed().as_secs_f64() * 1000.0 },
            std::io::ErrorKind::TimedOut | std::io::ErrorKind::WouldBlock => TcpOutcome::Timeout,
            _ => TcpOutcome::Error(e.to_string()),
        },
    }
}

#[derive(Debug, Clone)]
pub struct TcpReport {
    pub target: SocketAddr,
    pub outcomes: Vec<TcpOutcome>,
    pub summary: Summary,
    pub port_open: bool,
}

pub fn tcp_series(addr: SocketAddr, count: usize, timeout: Duration, gap: Duration) -> TcpReport {
    let mut outcomes = Vec::with_capacity(count);
    for i in 0..count {
        outcomes.push(tcp_once(addr, timeout));
        if i + 1 < count {
            std::thread::sleep(gap);
        }
    }
    let samples: Vec<Option<f64>> = outcomes.iter().map(|o| o.rtt()).collect();
    let port_open = outcomes.iter().any(|o| matches!(o, TcpOutcome::Connected { .. }));
    TcpReport { target: addr, summary: summarize(&samples), outcomes, port_open }
}

#[derive(Debug, Clone, PartialEq)]
pub enum UdpOutcome {
    /// پاسخ دریافت شد
    Reply { ms: f64, bytes: usize },
    /// سیستم‌عامل «port unreachable» گزارش کرد: پورت بسته
    Closed,
    /// پاسخی نیامد؛ در UDP یعنی «باز یا فیلترشده» — قطعی نیست
    NoReply,
    Error(String),
}

/// UDP اتصال‌گرا نیست؛ فقط با یک payload که سرویس مقصد به آن جواب می‌دهد
/// (مثلاً پرس‌وجوی DNS برای پورت ۵۳) نتیجه‌ی معنی‌دار می‌گیریم.
pub fn udp_check(addr: SocketAddr, payload: &[u8], timeout: Duration) -> UdpOutcome {
    let bind = if addr.is_ipv4() { "0.0.0.0:0" } else { "[::]:0" };
    let sock = match UdpSocket::bind(bind) {
        Ok(s) => s,
        Err(e) => return UdpOutcome::Error(e.to_string()),
    };
    sock.set_read_timeout(Some(timeout)).ok();
    if let Err(e) = sock.connect(addr) {
        return UdpOutcome::Error(e.to_string());
    }
    let start = Instant::now();
    if let Err(e) = sock.send(payload) {
        return UdpOutcome::Error(e.to_string());
    }
    let mut buf = [0u8; 2048];
    match sock.recv(&mut buf) {
        Ok(n) => UdpOutcome::Reply { ms: start.elapsed().as_secs_f64() * 1000.0, bytes: n },
        Err(e) if e.kind() == std::io::ErrorKind::ConnectionRefused => UdpOutcome::Closed,
        Err(e) if matches!(e.kind(), std::io::ErrorKind::WouldBlock | std::io::ErrorKind::TimedOut) => UdpOutcome::NoReply,
        Err(e) => UdpOutcome::Error(e.to_string()),
    }
}

/// اهداف پیش‌فرض تست «اینترنت در دسترس است؟». همه سرور عمومی شخص ثالث هستند.
pub const CONNECTIVITY_TARGETS: &[(&str, &str)] = &[
    ("Cloudflare DNS (TCP 443)", "1.1.1.1:443"),
    ("Google DNS (TCP 443)", "8.8.8.8:443"),
    ("Quad9 (TCP 443)", "9.9.9.9:443"),
];

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::TcpListener;

    #[test]
    fn tcp_open_and_closed() {
        let l = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = l.local_addr().unwrap();
        std::thread::spawn(move || for s in l.incoming().take(3) { drop(s) });
        let r = tcp_series(addr, 3, Duration::from_secs(1), Duration::from_millis(5));
        assert!(r.port_open);
        assert_eq!(r.summary.received, 3);

        // پورت بسته: listener را می‌سازیم و فوراً آزاد می‌کنیم
        let closed = TcpListener::bind("127.0.0.1:0").unwrap().local_addr().unwrap();
        // ویندوز پس از RST چند بار SYN را تکرار می‌کند و «refused» را حدود ۲ ثانیه
        // بعد گزارش می‌دهد؛ با timeout کوتاه‌تر نتیجه Timeout می‌شد.
        let o = tcp_once(closed, Duration::from_secs(5));
        assert!(matches!(o, TcpOutcome::Refused { .. }), "{o:?}");
    }

    #[test]
    fn udp_echo() {
        let s = UdpSocket::bind("127.0.0.1:0").unwrap();
        let addr = s.local_addr().unwrap();
        std::thread::spawn(move || {
            let mut b = [0u8; 64];
            let (n, f) = s.recv_from(&mut b).unwrap();
            s.send_to(&b[..n], f).unwrap();
        });
        assert!(matches!(udp_check(addr, b"ping", Duration::from_secs(1)), UdpOutcome::Reply { bytes: 4, .. }));
    }
}
