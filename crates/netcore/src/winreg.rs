//! تجزیه‌ی خروجی `reg query` (برای خواندن تنظیم پروکسی ویندوز).

/// مقدار `name` را از خروجی `reg query <key>` برمی‌گرداند.
/// خطوط این شکل را دارند: `    ProxyServer    REG_SZ    127.0.0.1:10808`
pub fn value(output: &str, name: &str) -> Option<String> {
    output.lines().find_map(|line| {
        let rest = line.trim().strip_prefix(name)?;
        // نام باید کامل منطبق باشد (ProxyServer نباید با ProxyServerX اشتباه شود)
        let rest = rest.strip_prefix(char::is_whitespace)?.trim_start();
        let (kind, data) = rest.split_once(char::is_whitespace).unwrap_or((rest, ""));
        kind.starts_with("REG_").then(|| data.trim().to_string())
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_reg_query() {
        let out = "\r\nHKEY_CURRENT_USER\\Software\\X\r\n    ProxyEnable    REG_DWORD    0x1\r\n    ProxyServerX    REG_SZ    no\r\n    ProxyServer    REG_SZ    127.0.0.1:10809\r\n    ProxyOverride    REG_SZ    localhost;<local>\r\n";
        assert_eq!(value(out, "ProxyEnable").as_deref(), Some("0x1"));
        assert_eq!(value(out, "ProxyServer").as_deref(), Some("127.0.0.1:10809"));
        assert_eq!(value(out, "ProxyOverride").as_deref(), Some("localhost;<local>"));
        assert_eq!(value(out, "AutoConfigURL"), None);
    }
}
