//! فهرست ابزارهای خارجیِ اختیاری و پرخطر.
//!
//! هیچ کدی از این پروژه‌ها داخل NetDesk نیست (لایسنس GPL برخی‌شان با MIT
//! سازگار نیست و ممیزی هم نشده‌اند). NetDesk فقط برنامه‌ای را اجرا می‌کند
//! که **خود کاربر** دانلود و نصب کرده، آن هم پس از:
//!   ۱. انتخاب مسیر فایل اجرایی و سنجاق کردن هش SHA-256 آن،
//!   ۲. خواندن همه‌ی هشدارها و تأیید تک‌تک آن‌ها،
//!   ۳. تایپ عبارت تأیید.
//! اگر فایل اجرایی عوض شود یا متن هشدارها تغییر کند (`CONSENT_VERSION`)،
//! تأیید باطل می‌شود. منطق این قواعد در Rust است تا رابط کاربری نتواند دورش بزند.

/// با هر تغییر معنادار در هشدارها یک واحد بالا برود تا تأییدهای قبلی باطل شوند.
pub const CONSENT_VERSION: u32 = 2;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Risk {
    /// هسته‌ی پروکسی شناخته‌شده؛ ریسک اصلی، اعتماد به سرور مقصد است.
    Medium,
    High,
    /// می‌تواند امنیت کل سیستم را از بین ببرد (CA ریشه، دسترسی مدیر، دست‌کاری بسته).
    Critical,
}

impl Risk {
    pub fn as_str(&self) -> &'static str {
        match self {
            Risk::Medium => "medium",
            Risk::High => "high",
            Risk::Critical => "critical",
        }
    }
    /// عبارتی که کاربر باید دقیقاً تایپ کند
    pub fn confirm_phrase(&self) -> &'static str {
        match self {
            Risk::Medium => "می‌دانم چه می‌کنم",
            Risk::High => "خطر را می‌پذیرم",
            Risk::Critical => "مسئولیت کامل را می‌پذیرم",
        }
    }
}

#[derive(Debug, Clone)]
pub struct ToolSpec {
    pub id: &'static str,
    pub name: &'static str,
    pub url: &'static str,
    pub license: &'static str,
    pub summary: &'static str,
    pub risk: Risk,
    pub requires_admin: bool,
    pub installs_root_ca: bool,
    /// صفحه‌ی پروژه هنگام بررسی در دسترس نبود
    pub unreviewed: bool,
    /// هر هشدار جداگانه باید تیک بخورد
    pub warnings: &'static [&'static str],
    /// راهنمای مسیر و آرگومان
    pub setup_hint: &'static str,
}

const COMMON: &str = "این ابزار کار سازنده‌ی دیگری است. NetDesk کد آن را ممیزی نکرده و درباره‌ی رفتارش تضمینی نمی‌دهد. فایلی را اجرا می‌کنید که خودتان ساخته یا نصب کرده‌اید.";
const CA_WARN: &str = "یک گواهی CA ریشه روی سیستم نصب می‌کند و ترافیک HTTPS را رمزگشایی می‌کند (MITM). هر کس به کلید این CA برسد می‌تواند همه‌ی سایت‌ها، از جمله بانک و ایمیل شما را جعل کند.";
const CA_REMOVE: &str = "بعد از استفاده گواهی را حذف کنید: Win+R ← certmgr.msc ← Trusted Root Certification Authorities ← Certificates ← گواهی این ابزار ← Delete.";
const APPS_SCRIPT: &str = "از Google Apps Script/Drive خارج از کاربرد مجاز استفاده می‌کند؛ ممکن است حساب Google شما تعلیق شود. از حساب اصلی‌تان استفاده نکنید.";
const ADMIN_WARN: &str = "با دسترسی Administrator/root اجرا می‌شود و بسته‌های TCP را در سطح هسته‌ی شبکه دست‌کاری می‌کند. هر باگ یا کد مخرب در آن یعنی کنترل کامل سیستم.";
const UAC_NOTE: &str = "ویندوز پنجره‌ی UAC نشان می‌دهد. NetDesk خودش دسترسی مدیر نمی‌گیرد؛ تصمیم با شماست.";

static CATALOG: &[ToolSpec] = &[
    ToolSpec {
        id: "sing-box",
        name: "sing-box",
        url: "https://github.com/yourcreatorox7-lgtm/netdesk/blob/main/docs/tutorials/sing-box.fa.md",
        license: "GPL-3.0-or-later",
        summary: "هسته‌ی پروکسی برای اتصال واقعی با پروفایل‌های vless، vmess، trojan، ss، hysteria2 و tuic. NetDesk کانفیگ را می‌سازد و فقط روی 127.0.0.1 گوش می‌دهد.",
        risk: Risk::Medium,
        requires_admin: false,
        installs_root_ca: false,
        unreviewed: false,
        warnings: &[
            COMMON,
            "فایل را طبق آموزش از سورس بسازید. اگر فایل آماده دانلود می‌کنید، فقط از منبع رسمی سازنده بگیرید و checksum آن را با هش NetDesk مقایسه کنید.",
            "همه‌ی ترافیکی که از پورت محلی عبور کند، برای سرور پروفایل قابل مشاهده است. فقط به سرور خودتان یا ارائه‌دهنده‌ی مورد اعتماد وصل شوید.",
            "کانفیگ موقت (که رمز پروفایل را دارد) در پوشه‌ی داده‌ی برنامه نوشته و بلافاصله پس از راه‌اندازی هسته پاک می‌شود.",
        ],
        setup_hint: "دکمه‌ی «پیدا کردن خودکار» را بزنید، یا مسیر sing-box.exe را انتخاب کنید. آرگومان لازم نیست.",
    },
    ToolSpec {
        id: "xray",
        name: "Xray-core",
        url: "https://github.com/yourcreatorox7-lgtm/netdesk/blob/main/docs/tutorials/xray.fa.md",
        license: "MPL-2.0",
        summary: "هسته‌ی دوم. «اتصال هوشمند» آن را فقط برای پروفایل‌های xhttp به‌کار می‌برد (sing-box xhttp ندارد) و کانفیگش را خودش می‌سازد. برای کانفیگ دستی هم می‌توانید جداگانه اجرایش کنید.",
        risk: Risk::Medium,
        requires_admin: false,
        installs_root_ca: false,
        unreviewed: false,
        warnings: &[
            COMMON,
            "فایل را طبق آموزش از سورس بسازید. اگر فایل آماده دانلود می‌کنید، فقط از منبع رسمی سازنده بگیرید و checksum را مقایسه کنید.",
            "کانفیگ Xray را خودتان می‌نویسید؛ اگر inbound را روی 0.0.0.0 باز کنید، دیگران در شبکه هم می‌توانند از آن استفاده کنند.",
        ],
        setup_hint: "«پیدا کردن خودکار» را بزنید. برای پروفایل‌های xhttp آرگومان لازم نیست؛ فقط برای اجرای دستی: run -c C:\\path\\config.json",
    },
    ToolSpec {
        id: "master-http-relay",
        name: "MasterHttpRelayVPN",
        url: "https://github.com/yourcreatorox7-lgtm/netdesk/blob/main/docs/tutorials/master-http-relay.fa.md",
        license: "MIT",
        summary: "پروکسی محلی با domain-fronting روی Google Apps Script و رهگیری TLS.",
        risk: Risk::Critical,
        requires_admin: false,
        installs_root_ca: true,
        unreviewed: false,
        warnings: &[COMMON, CA_WARN, CA_REMOVE, APPS_SCRIPT, "ترافیک شما از سرورهای Google و اسکریپتی که خودتان مستقر کرده‌اید عبور می‌کند."],
        setup_hint: "مسیر .venv\\Scripts\\python.exe که اسکریپت نصب چاپ می‌کند، آرگومان main.py و همان پوشه‌ی کاری.",
    },
    ToolSpec {
        id: "mhr-cfw",
        name: "mhr-cfw",
        url: "https://github.com/yourcreatorox7-lgtm/netdesk/blob/main/docs/tutorials/mhr-cfw.fa.md",
        license: "MIT",
        summary: "مشتق MasterHttpRelay با Cloudflare Worker؛ همان رهگیری TLS.",
        risk: Risk::Critical,
        requires_admin: false,
        installs_root_ca: true,
        unreviewed: false,
        warnings: &[
            COMMON,
            CA_WARN,
            CA_REMOVE,
            APPS_SCRIPT,
            "حالت UPSTREAM_FAIL_MODE=open ممکن است رله‌ی باز بسازد که دیگران از آن سوءاستفاده کنند؛ آن را روی closed بگذارید.",
        ],
        setup_hint: "مسیر .venv\\Scripts\\python.exe که اسکریپت نصب چاپ می‌کند، آرگومان main.py و همان پوشه‌ی کاری.",
    },
    ToolSpec {
        id: "flowdriver",
        name: "FlowDriver",
        url: "https://github.com/yourcreatorox7-lgtm/netdesk/blob/main/docs/tutorials/flowdriver.fa.md",
        license: "MIT",
        summary: "تونل SOCKS5 که Google Drive را صف پیام می‌کند.",
        risk: Risk::High,
        requires_admin: false,
        installs_root_ca: false,
        unreviewed: false,
        warnings: &[
            COMMON,
            APPS_SCRIPT,
            "توکن OAuth حساب Google در یک فایل ساده روی دیسک ذخیره می‌شود؛ هر برنامه‌ی دیگری روی سیستم می‌تواند آن را بخواند.",
            "خود نویسنده استفاده‌ی تولیدی را توصیه نمی‌کند.",
        ],
        setup_hint: "مسیر flowdriver-client.exe و آرگومان: -c client_config.json -gc credentials.json",
    },
    ToolSpec {
        id: "sni-spoofing-py",
        name: "SNI-Spoofing (Python)",
        url: "https://github.com/yourcreatorox7-lgtm/netdesk/blob/main/docs/tutorials/sni-spoofing-py.fa.md",
        license: "GPL-3.0",
        summary: "تزریق ClientHello جعلی با sequence number خارج از پنجره برای فریب DPI.",
        risk: Risk::Critical,
        requires_admin: true,
        installs_root_ca: false,
        unreviewed: false,
        warnings: &[COMMON, ADMIN_WARN, UAC_NOTE, "پروژه‌ی جوان با commit کم و issue باز زیاد؛ ممکن است اتصال شبکه را مختل کند."],
        setup_hint: "مسیر .venv\\Scripts\\python.exe که اسکریپت نصب چاپ می‌کند، آرگومان main.py و همان پوشه‌ی کاری.",
    },
    ToolSpec {
        id: "sni-spoofing-rust",
        name: "sni-spoofing-rust",
        url: "https://github.com/yourcreatorox7-lgtm/netdesk/blob/main/docs/tutorials/sni-spoofing-rust.fa.md",
        license: "MIT",
        summary: "پیاده‌سازی Rust همان تکنیک با WinDivert.",
        risk: Risk::Critical,
        requires_admin: true,
        installs_root_ca: false,
        unreviewed: false,
        warnings: &[
            COMMON,
            ADMIN_WARN,
            UAC_NOTE,
            "درایور WinDivert در هسته‌ی ویندوز بارگذاری می‌شود.",
            "دکمه‌ی «دانلود Xray» داخل خود این برنامه را استفاده نکنید؛ Xray را خودتان از منبع رسمی بگیرید.",
        ],
        setup_hint: "مسیر فایل ساخته‌شده و آرگومان‌های آموزش.",
    },
    ToolSpec {
        id: "sni-spoofing-go",
        name: "SNI-Spoofing-Go",
        url: "https://github.com/yourcreatorox7-lgtm/netdesk/blob/main/docs/tutorials/sni-spoofing-go.fa.md",
        license: "GPL-3.0",
        summary: "پیاده‌سازی Go با رابط Wails و پردازه‌ی کمکی ارتقایافته.",
        risk: Risk::Critical,
        requires_admin: true,
        installs_root_ca: false,
        unreviewed: false,
        warnings: &[
            COMMON,
            ADMIN_WARN,
            UAC_NOTE,
            "یک پردازه‌ی کمکی با دسترسی مدیر روی پورت TCP محلی باز می‌کند؛ هر برنامه‌ی دیگری روی سیستم ممکن است به آن فرمان بدهد.",
        ],
        setup_hint: "مسیر sni-spoofing-go.exe و آرگومان‌های آموزش.",
    },
];

pub fn catalog() -> &'static [ToolSpec] {
    CATALOG
}

pub fn find(id: &str) -> Option<&'static ToolSpec> {
    CATALOG.iter().find(|t| t.id == id)
}

/// بررسی تأیید کاربر: همه‌ی هشدارها تیک خورده و عبارت درست تایپ شده باشد.
pub fn check_consent(spec: &ToolSpec, acknowledged: &[usize], phrase: &str) -> Result<(), String> {
    for i in 0..spec.warnings.len() {
        if !acknowledged.contains(&i) {
            return Err(format!("هشدار شماره‌ی {} تأیید نشده است", i + 1));
        }
    }
    let norm = |s: &str| s.replace('\u{200c}', " ").split_whitespace().collect::<Vec<_>>().join(" ");
    if norm(phrase) != norm(spec.risk.confirm_phrase()) {
        return Err(format!("برای ادامه دقیقاً بنویسید: «{}»", spec.risk.confirm_phrase()));
    }
    Ok(())
}

/// تقسیم آرگومان‌ها با پشتیبانی از "..." و '...' (بدون shell؛ هیچ تفسیری انجام نمی‌شود)
pub fn split_args(s: &str) -> Result<Vec<String>, String> {
    let mut out = Vec::new();
    let mut cur = String::new();
    let mut quote: Option<char> = None;
    let mut has = false;
    for c in s.chars() {
        match (quote, c) {
            (Some(q), c) if c == q => quote = None,
            (Some(_), c) => cur.push(c),
            (None, '"' | '\'') => {
                quote = Some(c);
                has = true;
            }
            (None, c) if c.is_whitespace() => {
                if has || !cur.is_empty() {
                    out.push(std::mem::take(&mut cur));
                    has = false;
                }
            }
            (None, c) => cur.push(c),
        }
        if c == '\0' || c == '\n' || c == '\r' {
            return Err("آرگومان نباید کاراکتر کنترلی داشته باشد".into());
        }
    }
    if quote.is_some() {
        return Err("نقل‌قول بسته نشده".into());
    }
    if has || !cur.is_empty() {
        out.push(cur);
    }
    if out.len() > 64 {
        return Err("آرگومان‌ها بیش از حد زیادند".into());
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn catalog_is_consistent() {
        let mut ids: Vec<_> = catalog().iter().map(|t| t.id).collect();
        ids.sort();
        ids.dedup();
        assert_eq!(ids.len(), catalog().len(), "شناسه‌ی تکراری");
        for t in catalog() {
            assert!(!t.warnings.is_empty());
            if t.installs_root_ca || t.requires_admin || t.unreviewed {
                assert_eq!(t.risk, Risk::Critical, "{} باید بحرانی باشد", t.id);
            }
        }
        assert!(find("sing-box").is_some() && find("nope").is_none());
    }

    #[test]
    fn consent_requires_every_warning_and_phrase() {
        let t = find("master-http-relay").unwrap();
        let all: Vec<usize> = (0..t.warnings.len()).collect();
        assert!(check_consent(t, &all[1..], t.risk.confirm_phrase()).is_err());
        assert!(check_consent(t, &all, "بله").is_err());
        assert!(check_consent(t, &all, t.risk.confirm_phrase()).is_ok());
        // نیم‌فاصله و فاصله‌ی اضافه نباید مانع شود
        assert!(check_consent(t, &all, "  مسئولیت   کامل را می پذیرم ").is_ok());
    }

    #[test]
    fn args() {
        assert_eq!(split_args(r#"run -c "C:\my dir\c.json""#).unwrap(), vec!["run", "-c", r"C:\my dir\c.json"]);
        assert_eq!(split_args("a '' b").unwrap(), vec!["a", "", "b"]);
        assert!(split_args("\"open").is_err());
        assert!(split_args("a\nb").is_err());
        assert!(split_args("").unwrap().is_empty());
    }
}
