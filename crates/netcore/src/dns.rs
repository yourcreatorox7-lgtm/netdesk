//! پرس‌وجوی DNS خام روی UDP (RFC 1035) — بدون وابستگی به resolver سیستم.
//! برای «تست سلامت resolverهای عمومی» و «تحلیل زمان پاسخ DNS».

use crate::stats::{summarize, Summary};
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr, UdpSocket};
use std::time::{Duration, Instant};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RecordType {
    A = 1,
    Aaaa = 28,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Rcode {
    NoError,
    FormErr,
    ServFail,
    NxDomain,
    NotImp,
    Refused,
    Other(u8),
}

impl From<u8> for Rcode {
    fn from(v: u8) -> Self {
        match v {
            0 => Rcode::NoError,
            1 => Rcode::FormErr,
            2 => Rcode::ServFail,
            3 => Rcode::NxDomain,
            4 => Rcode::NotImp,
            5 => Rcode::Refused,
            o => Rcode::Other(o),
        }
    }
}

#[derive(Debug, Clone)]
pub struct DnsAnswer {
    pub rcode: Rcode,
    pub addresses: Vec<IpAddr>,
    pub truncated: bool,
    pub elapsed_ms: f64,
}

/// فهرست پیش‌فرض resolverهای عمومی. این‌ها «سرویس خارجی بدون حساب» هستند،
/// نه بخشی از برنامه؛ رابط کاربری باید همین را نشان دهد.
pub const PUBLIC_RESOLVERS: &[(&str, &str)] = &[
    ("Cloudflare", "1.1.1.1"),
    ("Cloudflare 2", "1.0.0.1"),
    ("Google", "8.8.8.8"),
    ("Google 2", "8.8.4.4"),
    ("Quad9", "9.9.9.9"),
    ("OpenDNS", "208.67.222.222"),
    ("AdGuard", "94.140.14.14"),
    ("Shecan", "178.22.122.100"),
    ("Electro", "78.157.42.100"),
    ("Radar", "10.202.10.10"),
];

pub fn build_query(id: u16, name: &str, rtype: RecordType) -> Result<Vec<u8>, String> {
    let mut q = Vec::with_capacity(32 + name.len());
    q.extend_from_slice(&id.to_be_bytes());
    q.extend_from_slice(&[0x01, 0x00]); // RD=1
    q.extend_from_slice(&[0, 1, 0, 0, 0, 0, 0, 0]); // QD=1
    let name = name.trim_end_matches('.');
    if name.is_empty() || name.len() > 253 {
        return Err("نام دامنه نامعتبر است".into());
    }
    for label in name.split('.') {
        if label.is_empty() || label.len() > 63 || !label.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_') {
            return Err(format!("برچسب دامنه نامعتبر: «{label}» (برای دامنه‌ی فارسی ابتدا punycode کنید)"));
        }
        q.push(label.len() as u8);
        q.extend_from_slice(label.as_bytes());
    }
    q.push(0);
    q.extend_from_slice(&(rtype as u16).to_be_bytes());
    q.extend_from_slice(&[0, 1]); // IN
    Ok(q)
}

fn skip_name(buf: &[u8], mut i: usize) -> Result<usize, String> {
    for _ in 0..128 {
        let len = *buf.get(i).ok_or("نام بریده‌شده")? as usize;
        if len == 0 {
            return Ok(i + 1);
        }
        if len & 0xC0 == 0xC0 {
            return Ok(i + 2); // pointer فشرده‌سازی
        }
        i += 1 + len;
    }
    Err("نام بیش از حد طولانی".into())
}

fn u16_at(b: &[u8], i: usize) -> Result<u16, String> {
    b.get(i..i + 2).map(|s| u16::from_be_bytes([s[0], s[1]])).ok_or_else(|| "پاسخ کوتاه".into())
}

/// تجزیه‌ی پاسخ؛ id باید با پرس‌وجو یکی باشد (جلوگیری از پذیرش پاسخ جعلی/دیرهنگام).
pub fn parse_response(buf: &[u8], expect_id: u16) -> Result<(Rcode, Vec<IpAddr>, bool), String> {
    if buf.len() < 12 {
        return Err("پاسخ DNS کوتاه‌تر از header است".into());
    }
    if u16_at(buf, 0)? != expect_id {
        return Err("شناسه‌ی پاسخ با پرس‌وجو نمی‌خواند".into());
    }
    if buf[2] & 0x80 == 0 {
        return Err("بسته پاسخ نیست (QR=0)".into());
    }
    let truncated = buf[2] & 0x02 != 0;
    let rcode = Rcode::from(buf[3] & 0x0F);
    let qd = u16_at(buf, 4)?;
    let an = u16_at(buf, 6)?;
    let mut i = 12;
    for _ in 0..qd {
        i = skip_name(buf, i)? + 4;
    }
    let mut addrs = Vec::new();
    for _ in 0..an {
        i = skip_name(buf, i)?;
        let t = u16_at(buf, i)?;
        let rdlen = u16_at(buf, i + 8)? as usize;
        let data = buf.get(i + 10..i + 10 + rdlen).ok_or("rdata بریده‌شده")?;
        match (t, rdlen) {
            (1, 4) => addrs.push(IpAddr::V4(Ipv4Addr::new(data[0], data[1], data[2], data[3]))),
            (28, 16) => {
                let mut o = [0u8; 16];
                o.copy_from_slice(data);
                addrs.push(IpAddr::V6(Ipv6Addr::from(o)));
            }
            _ => {}
        }
        i += 10 + rdlen;
    }
    Ok((rcode, addrs, truncated))
}

fn next_id() -> u16 {
    // شناسه‌ی غیرقابل‌پیش‌بینی ساده از ساعت + آدرس پشته (برای تست کافی؛ امنیتی نیست)
    let t = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.subsec_nanos()).unwrap_or(7);
    let x = &t as *const u32 as usize as u32;
    ((t ^ x.rotate_left(13)) & 0xFFFF) as u16
}

/// یک پرس‌وجو به یک resolver.
pub fn query(resolver: SocketAddr, name: &str, rtype: RecordType, timeout: Duration) -> Result<DnsAnswer, String> {
    let id = next_id();
    let q = build_query(id, name, rtype)?;
    let bind: SocketAddr = if resolver.is_ipv4() { "0.0.0.0:0".parse().unwrap() } else { "[::]:0".parse().unwrap() };
    let sock = UdpSocket::bind(bind).map_err(|e| format!("bind: {e}"))?;
    sock.set_read_timeout(Some(timeout)).ok();
    sock.connect(resolver).map_err(|e| format!("connect: {e}"))?;
    let start = Instant::now();
    sock.send(&q).map_err(|e| format!("ارسال: {e}"))?;
    let mut buf = [0u8; 1500];
    loop {
        let n = sock.recv(&mut buf).map_err(|e| {
            if matches!(e.kind(), std::io::ErrorKind::WouldBlock | std::io::ErrorKind::TimedOut) {
                "timeout".to_string()
            } else {
                format!("دریافت: {e}")
            }
        })?;
        match parse_response(&buf[..n], id) {
            Ok((rcode, addresses, truncated)) => {
                return Ok(DnsAnswer { rcode, addresses, truncated, elapsed_ms: start.elapsed().as_secs_f64() * 1000.0 })
            }
            Err(_) if start.elapsed() < timeout => continue, // بسته‌ی نامربوط؛ صبر برای پاسخ درست
            Err(e) => return Err(e),
        }
    }
}

#[derive(Debug, Clone)]
pub struct ResolverHealth {
    pub resolver: SocketAddr,
    pub summary: Summary,
    pub last_rcode: Option<Rcode>,
    pub last_addresses: Vec<IpAddr>,
    pub errors: Vec<String>,
}

/// چند پرس‌وجوی پیاپی و خلاصه‌ی سلامت resolver.
pub fn health_check(resolver: SocketAddr, name: &str, attempts: usize, timeout: Duration) -> ResolverHealth {
    let mut samples = Vec::with_capacity(attempts);
    let mut errors = Vec::new();
    let mut last_rcode = None;
    let mut last_addresses = Vec::new();
    for _ in 0..attempts {
        match query(resolver, name, RecordType::A, timeout) {
            Ok(a) => {
                samples.push(if a.rcode == Rcode::NoError || a.rcode == Rcode::NxDomain { Some(a.elapsed_ms) } else { None });
                last_rcode = Some(a.rcode);
                last_addresses = a.addresses;
            }
            Err(e) => {
                samples.push(None);
                errors.push(e);
            }
        }
    }
    ResolverHealth { resolver, summary: summarize(&samples), last_rcode, last_addresses, errors }
}

/// نشانه‌های ساده‌ی دست‌کاری DNS: پاسخ با آدرس خصوصی/رزروشده برای دامنه‌ی عمومی.
/// این فقط «هشدار» است، نه حکم قطعی.
pub fn suspicious_answer(addrs: &[IpAddr]) -> Option<String> {
    for a in addrs {
        if let IpAddr::V4(v4) = a {
            if v4.is_private() || v4.is_loopback() || v4.is_unspecified() || v4.octets()[0] == 10 {
                return Some(format!("پاسخ {v4} آدرس خصوصی/رزرو است؛ ممکن است DNS دست‌کاری یا مسدود شده باشد"));
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fake_response(q: &[u8], ip: [u8; 4]) -> Vec<u8> {
        let mut r = q.to_vec();
        r[2] = 0x81;
        r[3] = 0x80;
        r[7] = 1; // AN=1
        r.extend_from_slice(&[0xC0, 0x0C, 0, 1, 0, 1, 0, 0, 0, 60, 0, 4]);
        r.extend_from_slice(&ip);
        r
    }

    #[test]
    fn build_and_parse() {
        let q = build_query(0x1234, "example.com", RecordType::A).unwrap();
        let r = fake_response(&q, [93, 184, 216, 34]);
        let (rc, a, tc) = parse_response(&r, 0x1234).unwrap();
        assert_eq!(rc, Rcode::NoError);
        assert!(!tc);
        assert_eq!(a, vec![IpAddr::V4(Ipv4Addr::new(93, 184, 216, 34))]);
        assert!(parse_response(&r, 0x9999).is_err());
    }

    #[test]
    fn rejects_bad_names() {
        assert!(build_query(1, "", RecordType::A).is_err());
        assert!(build_query(1, "a..b", RecordType::A).is_err());
        assert!(build_query(1, "سلام.ir", RecordType::A).is_err());
        assert!(build_query(1, &"a".repeat(64), RecordType::A).is_err());
    }

    #[test]
    fn suspicious() {
        assert!(suspicious_answer(&["10.10.34.35".parse().unwrap()]).is_some());
        assert!(suspicious_answer(&["1.2.3.4".parse().unwrap()]).is_none());
    }

    #[test]
    fn live_query_against_local_fake_server() {
        let server = UdpSocket::bind("127.0.0.1:0").unwrap();
        let addr = server.local_addr().unwrap();
        std::thread::spawn(move || {
            let mut buf = [0u8; 512];
            for _ in 0..3 {
                let (n, from) = server.recv_from(&mut buf).unwrap();
                server.send_to(&fake_response(&buf[..n], [1, 2, 3, 4]), from).unwrap();
            }
        });
        let h = health_check(addr, "example.com", 3, Duration::from_secs(2));
        assert_eq!(h.summary.received, 3, "{:?}", h.errors);
        assert_eq!(h.last_addresses, vec!["1.2.3.4".parse::<IpAddr>().unwrap()]);
    }
}
