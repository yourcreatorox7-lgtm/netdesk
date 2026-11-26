//! ابزار خط فرمان برای تست هسته بدون رابط گرافیکی.
//!
//!   netcore-cli dns <resolver-ip> <domain> [count]
//!   netcore-cli tcp <host:port> [count]
//!   netcore-cli quick [domain]            گزارش کامل Markdown
//!   netcore-cli quick-json [domain]
//!   netcore-cli import <file>             اعتبارسنجی و تشخیص تکراری
//!   netcore-cli socks5 <host:port>        تست پروکسی SOCKS5

use netcore::{config, dns, probe, proxy, report};
use std::net::SocketAddr;
use std::time::Duration;

fn main() {
    let a: Vec<String> = std::env::args().skip(1).collect();
    let arg = |i: usize| a.get(i).map(String::as_str);
    let t = Duration::from_secs(2);
    match arg(0) {
        Some("dns") => {
            let r: SocketAddr = format!("{}:53", arg(1).unwrap_or("1.1.1.1")).parse().expect("IP نامعتبر");
            let h = dns::health_check(r, arg(2).unwrap_or("example.com"), arg(3).and_then(|x| x.parse().ok()).unwrap_or(4), t);
            println!("{h:#?}");
        }
        Some("tcp") => {
            let addr = probe::resolve(arg(1).unwrap_or("1.1.1.1:443")).unwrap_or_else(|e| fail(&e));
            let r = probe::tcp_series(addr, arg(2).and_then(|x| x.parse().ok()).unwrap_or(5), t, Duration::from_millis(200));
            println!("{:#?}", r.summary);
        }
        Some(cmd @ ("quick" | "quick-json")) => {
            let res: Vec<(&str, SocketAddr)> = dns::PUBLIC_RESOLVERS.iter().map(|(n, ip)| (*n, format!("{ip}:53").parse().unwrap())).collect();
            let tcp: Vec<(&str, SocketAddr)> = probe::CONNECTIVITY_TARGETS.iter().map(|(n, s)| (*n, s.parse().unwrap())).collect();
            let opts = report::Options { test_domain: arg(1).unwrap_or("example.com").into(), ..Default::default() };
            let r = report::run_quick(&opts, &res, &tcp);
            println!("{}", if cmd == "quick" { r.to_markdown() } else { r.to_json() });
        }
        Some("import") => {
            let text = std::fs::read_to_string(arg(1).unwrap_or_else(|| fail("مسیر فایل؟"))).unwrap_or_else(|e| fail(&e.to_string()));
            let r = config::import_text(&text);
            println!("Base64: {}  |  موفق: {}/{}  |  تکراری: {}", r.was_base64, r.ok_count(), r.lines.len(), r.duplicates.len());
            for l in &r.lines {
                match &l.result {
                    Ok(p) => {
                        println!("✓ خط {:>4}  {:<10} {}  [{}]", l.line_no, p.protocol.scheme(), p.name, config::fingerprint(p));
                        for i in config::validate(p) {
                            println!("           {:?}: {}", i.severity, i.message);
                        }
                    }
                    Err(e) => println!("✗ خط {:>4}  {e}", l.line_no),
                }
            }
        }
        Some("socks5") => {
            let addr = probe::resolve(arg(1).unwrap_or("127.0.0.1:1080")).unwrap_or_else(|e| fail(&e));
            println!("{:?}", proxy::check_socks5(addr, None, "example.com", 443, t));
        }
        _ => println!("فرمان‌ها: dns | tcp | quick | quick-json | import | socks5   (جزئیات در کد منبع)"),
    }
}

fn fail(m: &str) -> ! {
    eprintln!("خطا: {m}");
    std::process::exit(1)
}
