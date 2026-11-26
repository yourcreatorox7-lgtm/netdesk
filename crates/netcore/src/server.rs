//! مانیتورینگ VPS: برنامه از طریق SSH (در لایه‌ی Tauri با crate «russh»)
//! یک فرمان فقط‌خواندنی اجرا می‌کند و خروجی متنی را این‌جا تجزیه می‌کنیم.
//! جداسازی تجزیه از انتقال یعنی این منطق بدون سرور واقعی تست‌پذیر است.

/// فرمان استاندارد جمع‌آوری؛ هیچ تغییری روی سرور نمی‌دهد و به root نیاز ندارد.
pub const PROBE_COMMAND: &str = "cat /proc/loadavg; echo ---; head -1 /proc/stat; sleep 1; head -1 /proc/stat; echo ---; cat /proc/meminfo; echo ---; df -P -B1 / ; echo ---; cat /proc/uptime; echo ---; nproc";

#[derive(Debug, Clone, PartialEq, Default)]
pub struct ServerMetrics {
    pub load1: f64,
    pub load5: f64,
    pub load15: f64,
    pub cpu_pct: Option<f64>,
    pub cpus: Option<u32>,
    pub mem_total_kb: u64,
    pub mem_available_kb: u64,
    pub swap_total_kb: u64,
    pub swap_free_kb: u64,
    pub disk_total_b: u64,
    pub disk_used_b: u64,
    pub uptime_s: f64,
}

impl ServerMetrics {
    pub fn mem_used_pct(&self) -> f64 {
        if self.mem_total_kb == 0 { 0.0 } else { (self.mem_total_kb - self.mem_available_kb) as f64 * 100.0 / self.mem_total_kb as f64 }
    }
    pub fn disk_used_pct(&self) -> f64 {
        if self.disk_total_b == 0 { 0.0 } else { self.disk_used_b as f64 * 100.0 / self.disk_total_b as f64 }
    }
    /// وضعیت کلی برای کارت سرور
    pub fn health(&self) -> Health {
        let cpu = self.cpu_pct.unwrap_or(0.0);
        if cpu > 95.0 || self.mem_used_pct() > 95.0 || self.disk_used_pct() > 95.0 {
            Health::Critical
        } else if cpu > 80.0 || self.mem_used_pct() > 85.0 || self.disk_used_pct() > 85.0 {
            Health::Warning
        } else {
            Health::Ok
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Health {
    Ok,
    Warning,
    Critical,
}

fn cpu_line(l: &str) -> Option<(u64, u64)> {
    let v: Vec<u64> = l.split_whitespace().skip(1).filter_map(|x| x.parse().ok()).collect();
    if v.len() < 4 {
        return None;
    }
    let idle = v[3] + v.get(4).copied().unwrap_or(0); // idle + iowait
    Some((v.iter().sum(), idle))
}

pub fn parse_probe_output(out: &str) -> Result<ServerMetrics, String> {
    let sec: Vec<&str> = out.split("---").map(|s| s.trim()).collect();
    if sec.len() < 6 {
        return Err(format!("خروجی ناقص: {} بخش به‌جای ۶", sec.len()));
    }
    let mut m = ServerMetrics::default();
    let la: Vec<f64> = sec[0].split_whitespace().take(3).filter_map(|x| x.parse().ok()).collect();
    if la.len() == 3 {
        (m.load1, m.load5, m.load15) = (la[0], la[1], la[2]);
    }
    let cpu: Vec<(u64, u64)> = sec[1].lines().filter_map(cpu_line).collect();
    if cpu.len() == 2 {
        let (dt, di) = (cpu[1].0.saturating_sub(cpu[0].0), cpu[1].1.saturating_sub(cpu[0].1));
        if dt > 0 {
            m.cpu_pct = Some((dt - di.min(dt)) as f64 * 100.0 / dt as f64);
        }
    }
    for l in sec[2].lines() {
        let mut it = l.split_whitespace();
        let (Some(k), Some(v)) = (it.next(), it.next().and_then(|v| v.parse::<u64>().ok())) else { continue };
        match k {
            "MemTotal:" => m.mem_total_kb = v,
            "MemAvailable:" => m.mem_available_kb = v,
            "SwapTotal:" => m.swap_total_kb = v,
            "SwapFree:" => m.swap_free_kb = v,
            _ => {}
        }
    }
    if let Some(l) = sec[3].lines().nth(1) {
        let f: Vec<&str> = l.split_whitespace().collect();
        if f.len() >= 4 {
            m.disk_total_b = f[1].parse().unwrap_or(0);
            m.disk_used_b = f[2].parse().unwrap_or(0);
        }
    }
    m.uptime_s = sec[4].split_whitespace().next().and_then(|x| x.parse().ok()).unwrap_or(0.0);
    m.cpus = sec[5].trim().parse().ok();
    Ok(m)
}

#[cfg(test)]
mod tests {
    use super::*;
    const SAMPLE: &str = "0.52 0.40 0.31 1/123 4567
---
cpu  1000 0 500 8000 100 0 0 0 0 0
cpu  1100 0 550 8300 100 0 0 0 0 0
---
MemTotal:        2000000 kB
MemFree:          300000 kB
MemAvailable:     500000 kB
SwapTotal:       1000000 kB
SwapFree:        1000000 kB
---
Filesystem     1-blocks       Used  Available Capacity Mounted on
/dev/vda1    42000000000 21000000000 21000000000      50% /
---
86400.12 170000.00
---
2";

    #[test]
    fn parses() {
        let m = parse_probe_output(SAMPLE).unwrap();
        assert_eq!(m.load1, 0.52);
        assert_eq!(m.cpus, Some(2));
        // Δtotal=450، Δidle=300 → ۳۳.۳٪
        assert!((m.cpu_pct.unwrap() - 33.333).abs() < 0.01);
        assert_eq!(m.mem_used_pct(), 75.0);
        assert_eq!(m.disk_used_pct(), 50.0);
        assert_eq!(m.health(), Health::Ok);
    }
    #[test]
    fn incomplete() {
        assert!(parse_probe_output("1 2 3").is_err());
    }
}
