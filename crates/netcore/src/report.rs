//! گزارش تشخیصی: یک اجرای کامل «عیب‌یابی سریع» و خروجی Markdown/JSON.

use crate::dns::{self, ResolverHealth};
use crate::json::Json;
use crate::probe::{self, TcpReport};
use crate::stats::{quality_score, Summary};
use std::collections::BTreeMap;
use std::net::SocketAddr;
use std::time::Duration;

#[derive(Debug)]
pub struct Report {
    pub resolvers: Vec<(String, ResolverHealth)>,
    pub tcp: Vec<(String, TcpReport)>,
    pub findings: Vec<String>,
}

pub struct Options {
    pub test_domain: String,
    pub attempts: usize,
    pub timeout: Duration,
}

impl Default for Options {
    fn default() -> Self {
        Options { test_domain: "example.com".into(), attempts: 4, timeout: Duration::from_secs(2) }
    }
}

pub fn run_quick(opts: &Options, resolvers: &[(&str, SocketAddr)], tcp_targets: &[(&str, SocketAddr)]) -> Report {
    // اجرای موازی با thread؛ هر هدف مستقل است
    let rs: Vec<_> = resolvers.iter().map(|(n, a)| {
        let (n, a, d, k, t) = (n.to_string(), *a, opts.test_domain.clone(), opts.attempts, opts.timeout);
        std::thread::spawn(move || (n, dns::health_check(a, &d, k, t)))
    }).collect();
    let ts: Vec<_> = tcp_targets.iter().map(|(n, a)| {
        let (n, a, k, t) = (n.to_string(), *a, opts.attempts, opts.timeout);
        std::thread::spawn(move || (n, probe::tcp_series(a, k, t, Duration::from_millis(150))))
    }).collect();
    let resolvers: Vec<_> = rs.into_iter().filter_map(|h| h.join().ok()).collect();
    let tcp: Vec<_> = ts.into_iter().filter_map(|h| h.join().ok()).collect();

    let mut findings = Vec::new();
    let dns_ok = resolvers.iter().filter(|(_, h)| h.summary.received > 0).count();
    let tcp_ok = tcp.iter().filter(|(_, r)| r.port_open).count();
    if dns_ok == 0 && tcp_ok == 0 {
        findings.push("هیچ مقصدی پاسخ نداد: اتصال اینترنت یا فایروال محلی را بررسی کنید.".into());
    } else if dns_ok == 0 {
        findings.push("TCP کار می‌کند اما هیچ resolver روی UDP/53 پاسخ نمی‌دهد: احتمالاً DNS مسدود شده؛ DoH/DoT را امتحان کنید.".into());
    } else if tcp_ok == 0 {
        findings.push("DNS پاسخ می‌دهد اما اتصال TCP به مقاصد آزمایشی برقرار نشد.".into());
    }
    for (n, h) in &resolvers {
        if let Some(w) = dns::suspicious_answer(&h.last_addresses) {
            findings.push(format!("{n}: {w}"));
        }
        if h.summary.loss_pct > 0.0 && h.summary.received > 0 {
            findings.push(format!("{n}: {:.0}٪ از پرس‌وجوها بی‌پاسخ ماند", h.summary.loss_pct));
        }
    }
    if let Some((n, h)) = resolvers.iter().filter(|(_, h)| h.summary.avg_ms.is_some())
        .min_by(|a, b| a.1.summary.avg_ms.partial_cmp(&b.1.summary.avg_ms).unwrap())
    {
        findings.push(format!("سریع‌ترین resolver: {n} ({:.1} ms)", h.summary.avg_ms.unwrap()));
    }
    Report { resolvers, tcp, findings }
}

fn fmt(v: Option<f64>) -> String {
    v.map(|x| format!("{x:.1}")).unwrap_or_else(|| "—".into())
}

fn sum_json(s: &Summary) -> Json {
    let mut m = BTreeMap::new();
    let n = |v: Option<f64>| v.map(Json::Num).unwrap_or(Json::Null);
    m.insert("sent".into(), Json::Num(s.sent as f64));
    m.insert("received".into(), Json::Num(s.received as f64));
    m.insert("lossPct".into(), Json::Num(s.loss_pct));
    m.insert("minMs".into(), n(s.min_ms));
    m.insert("avgMs".into(), n(s.avg_ms));
    m.insert("p95Ms".into(), n(s.p95_ms));
    m.insert("maxMs".into(), n(s.max_ms));
    m.insert("jitterMs".into(), n(s.jitter_ms));
    m.insert("score".into(), Json::Num(quality_score(s) as f64));
    Json::Obj(m)
}

impl Report {
    pub fn to_markdown(&self) -> String {
        let mut o = String::from("# گزارش عیب‌یابی شبکه\n\n## resolverهای DNS\n\n| نام | آدرس | میانگین ms | p95 | jitter | loss٪ | امتیاز |\n|---|---|---|---|---|---|---|\n");
        for (n, h) in &self.resolvers {
            let s = &h.summary;
            o += &format!("| {n} | {} | {} | {} | {} | {:.0} | {} |\n", h.resolver, fmt(s.avg_ms), fmt(s.p95_ms), fmt(s.jitter_ms), s.loss_pct, quality_score(s));
        }
        o += "\n## اتصال TCP\n\n| نام | مقصد | میانگین ms | loss٪ | پورت باز |\n|---|---|---|---|---|\n";
        for (n, r) in &self.tcp {
            o += &format!("| {n} | {} | {} | {:.0} | {} |\n", r.target, fmt(r.summary.avg_ms), r.summary.loss_pct, if r.port_open { "بله" } else { "خیر" });
        }
        o += "\n## یافته‌ها\n\n";
        for f in &self.findings {
            o += &format!("- {f}\n");
        }
        o
    }

    pub fn to_json(&self) -> String {
        let mut root = BTreeMap::new();
        root.insert("resolvers".into(), Json::Arr(self.resolvers.iter().map(|(n, h)| {
            let mut m = BTreeMap::new();
            m.insert("name".into(), Json::Str(n.clone()));
            m.insert("address".into(), Json::Str(h.resolver.to_string()));
            m.insert("summary".into(), sum_json(&h.summary));
            m.insert("answers".into(), Json::Arr(h.last_addresses.iter().map(|a| Json::Str(a.to_string())).collect()));
            m.insert("errors".into(), Json::Arr(h.errors.iter().cloned().map(Json::Str).collect()));
            Json::Obj(m)
        }).collect()));
        root.insert("tcp".into(), Json::Arr(self.tcp.iter().map(|(n, r)| {
            let mut m = BTreeMap::new();
            m.insert("name".into(), Json::Str(n.clone()));
            m.insert("target".into(), Json::Str(r.target.to_string()));
            m.insert("portOpen".into(), Json::Bool(r.port_open));
            m.insert("summary".into(), sum_json(&r.summary));
            Json::Obj(m)
        }).collect()));
        root.insert("findings".into(), Json::Arr(self.findings.iter().cloned().map(Json::Str).collect()));
        Json::Obj(root).to_string()
    }
}
