//! مدیریت Remote Desktop — کاملاً مستقل از VPN/پروکسی (الزام بخش ۲).
//! برنامه فایل .rdp می‌سازد و کلاینت بومی سیستم را اجرا می‌کند:
//! Windows: mstsc.exe   macOS: Microsoft Remote Desktop / Windows App   Linux: xfreerdp
//! رمز عبور هرگز داخل فایل .rdp نوشته نمی‌شود؛ کلاینت بومی خودش می‌پرسد
//! یا از Credential Manager سیستم می‌خواند.

#[derive(Debug, Clone)]
pub struct RdpTarget {
    pub host: String,
    pub port: u16,
    pub username: Option<String>,
    pub domain: Option<String>,
    pub fullscreen: bool,
    pub width: u32,
    pub height: u32,
    pub redirect_clipboard: bool,
    pub redirect_drives: bool,
    pub gateway: Option<String>,
}

impl Default for RdpTarget {
    fn default() -> Self {
        RdpTarget { host: String::new(), port: 3389, username: None, domain: None, fullscreen: true, width: 1920, height: 1080, redirect_clipboard: true, redirect_drives: false, gateway: None }
    }
}

fn clean(s: &str) -> Result<&str, String> {
    if s.contains(['\r', '\n']) {
        Err("مقدار شامل خط جدید است (تزریق تنظیمات در فایل .rdp)".into())
    } else {
        Ok(s)
    }
}

pub fn to_rdp_file(t: &RdpTarget) -> Result<String, String> {
    if t.host.is_empty() {
        return Err("میزبان خالی است".into());
    }
    let mut o = String::new();
    let addr = if t.port == 3389 { clean(&t.host)?.to_string() } else { format!("{}:{}", clean(&t.host)?, t.port) };
    o += &format!("full address:s:{addr}\r\n");
    o += &format!("screen mode id:i:{}\r\n", if t.fullscreen { 2 } else { 1 });
    o += &format!("desktopwidth:i:{}\r\ndesktopheight:i:{}\r\n", t.width, t.height);
    if let Some(u) = &t.username {
        o += &format!("username:s:{}\r\n", clean(u)?);
    }
    if let Some(d) = &t.domain {
        o += &format!("domain:s:{}\r\n", clean(d)?);
    }
    o += &format!("redirectclipboard:i:{}\r\n", t.redirect_clipboard as u8);
    o += &format!("drivestoredirect:s:{}\r\n", if t.redirect_drives { "*" } else { "" });
    // احراز سطح شبکه (NLA) و هشدار در صورت گواهی نامعتبر — پیش‌فرض‌های امن
    o += "enablecredsspsupport:i:1\r\nauthentication level:i:2\r\nprompt for credentials:i:1\r\n";
    if let Some(g) = &t.gateway {
        o += &format!("gatewayhostname:s:{}\r\ngatewayusagemethod:i:1\r\n", clean(g)?);
    }
    Ok(o)
}

/// آرگومان‌های xfreerdp برای لینوکس (بدون رمز — /from-stdin برای پرسش امن)
pub fn xfreerdp_args(t: &RdpTarget) -> Vec<String> {
    let mut a = vec![format!("/v:{}:{}", t.host, t.port), "/cert:tofu".into()];
    if let Some(u) = &t.username {
        a.push(format!("/u:{u}"));
    }
    if let Some(d) = &t.domain {
        a.push(format!("/d:{d}"));
    }
    a.push(if t.fullscreen { "/f".into() } else { format!("/size:{}x{}", t.width, t.height) });
    if t.redirect_clipboard {
        a.push("+clipboard".into());
    }
    a.push("/from-stdin".into());
    a
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn builds() {
        let t = RdpTarget { host: "10.0.0.5".into(), port: 3390, username: Some("admin".into()), ..Default::default() };
        let f = to_rdp_file(&t).unwrap();
        assert!(f.contains("full address:s:10.0.0.5:3390"));
        assert!(!f.to_lowercase().contains("password"));
        assert!(xfreerdp_args(&t).contains(&"/u:admin".to_string()));
    }
    #[test]
    fn injection() {
        let t = RdpTarget { host: "h\r\nalternate shell:s:cmd".into(), ..Default::default() };
        assert!(to_rdp_file(&t).is_err());
    }
}
