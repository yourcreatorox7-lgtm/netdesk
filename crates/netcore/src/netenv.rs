//! تشخیص برنامه‌های دیگری که ترافیک را پیش از NetDesk می‌گیرند: VPN با کارت
//! مجازی (TUN) و پروکسی سیستم. فقط متن را تحلیل می‌کند؛ اجرای دستور با برنامه است.

/// نام‌هایی که روی کارت مجازی VPN/TUN دیده می‌شود
const TUNNEL_WORDS: &[&str] = &[
    "happ", "xray", "wintun", "tun", "hiddify", "v2ray", "v2rayn", "clash", "mihomo", "wireguard", "nekoray", "nekobox",
    "singbox", "openvpn", "tap", "warp", "outline", "psiphon", "tailscale", "zerotier",
];

/// هر خط `نام|توضیح` (خروجی Get-NetAdapter برای کارت‌های روشن). فقط نام کارت‌های
/// مشکوک برمی‌گردد. کارت‌های VirtualBox/VMware/Hyper-V مشکوک حساب نمی‌شوند.
pub fn tunnel_adapters(listing: &str) -> Vec<String> {
    let mut found = Vec::new();
    for line in listing.lines() {
        let Some((name, desc)) = line.split_once('|') else { continue };
        let words: Vec<String> = format!("{name} {desc}")
            .to_lowercase()
            .split(|c: char| !c.is_ascii_alphanumeric())
            .filter(|w| !w.is_empty())
            .map(str::to_string)
            .collect();
        let virtual_machine = words.iter().any(|w| matches!(w.as_str(), "virtualbox" | "vmware" | "hyper" | "vethernet"));
        if !virtual_machine && words.iter().any(|w| TUNNEL_WORDS.contains(&w.as_str())) {
            let n = name.trim().to_string();
            if !n.is_empty() && !found.contains(&n) {
                found.push(n);
            }
        }
    }
    found
}

/// نشانی پروکسی سیستم اگر روشن باشد (از خروجی `reg query` کلید Internet Settings)
pub fn system_proxy(reg_output: &str) -> Option<String> {
    let enabled = crate::winreg::value(reg_output, "ProxyEnable").map(|v| v == "0x1" || v == "1").unwrap_or(false);
    let server = crate::winreg::value(reg_output, "ProxyServer").filter(|s| !s.is_empty())?;
    enabled.then_some(server)
}

/// نشانی‌ها (IP، دامنه، URL، IPv6) را از متن خطا حذف می‌کند تا گزارش بتواند
/// بدون لو دادن سرورها به هوش مصنوعی یا دیگران داده شود.
pub fn scrub_addresses(text: &str) -> String {
    text.split_whitespace()
        .map(|tok| {
            let cleaned: String = tok.chars().filter(|c| !matches!(c, '[' | ']')).collect();
            let t = cleaned.trim_matches(|c: char| matches!(c, '(' | ')' | ',' | ';' | '"' | '\''));
            let t = t.trim_end_matches(['.', ':']);
            let no_port = t.split(':').next().unwrap_or("");
            let dotted = no_port.contains('.')
                && no_port.chars().any(|c| c.is_ascii_alphanumeric())
                && no_port.split('.').all(|p| !p.is_empty() && p.chars().all(|c| c.is_ascii_alphanumeric() || c == '-'));
            let is_ip4 = no_port.split('.').count() == 4 && no_port.split('.').all(|p| !p.is_empty() && p.chars().all(|c| c.is_ascii_digit()));
            let ipv6 = t.matches(':').count() >= 2 && t.chars().all(|c| c.is_ascii_hexdigit() || c == ':' || c == '.');
            if t.contains("://") || dotted || is_ip4 || ipv6 { "<نشانی>".to_string() } else { tok.to_string() }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finds_tunnel_adapters_but_not_vm_or_lan() {
        let listing = "Ethernet|Realtek PCIe 2.5GbE Family Controller\nEthernet 2|VirtualBox Host-Only Ethernet Adapter\nhapp-xray|Happ Tunnel\nWi-Fi|Intel Wireless\nvEthernet (WSL)|Hyper-V Virtual Ethernet Adapter\nMyVPN|Wintun Userspace Tunnel";
        assert_eq!(tunnel_adapters(listing), vec!["happ-xray".to_string(), "MyVPN".to_string()]);
        assert!(tunnel_adapters("Ethernet|Realtek PCIe GbE\nWi-Fi|Intel(R) Wi-Fi 6").is_empty());
    }

    #[test]
    fn scrubs_hosts_and_ips_but_keeps_plain_words() {
        let t = "dial tcp 203.0.113.9:443: connectex: failed to reach my.server.example:8443 (os error 10060) via [2001:db8::1]:443 https://a.b/c";
        let s = scrub_addresses(t);
        for secret in ["203.0.113.9", "my.server.example", "2001:db8", "a.b/c"] {
            assert!(!s.contains(secret), "{secret} leaked in {s}");
        }
        assert!(s.contains("os error 10060") && s.contains("failed to reach"));
        assert_eq!(scrub_addresses("تونل برقرار نشد (کد SOCKS 1)"), "تونل برقرار نشد (کد SOCKS 1)");
    }

    #[test]
    fn reads_enabled_system_proxy_only() {
        let on = "    ProxyEnable    REG_DWORD    0x1\n    ProxyServer    REG_SZ    127.0.0.1:10809\n";
        let off = "    ProxyEnable    REG_DWORD    0x0\n    ProxyServer    REG_SZ    127.0.0.1:10809\n";
        assert_eq!(system_proxy(on).as_deref(), Some("127.0.0.1:10809"));
        assert_eq!(system_proxy(off), None);
        assert_eq!(system_proxy(""), None);
    }
}
