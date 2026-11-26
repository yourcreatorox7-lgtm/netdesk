//! آمار نمونه‌های تأخیر: min/avg/median/p95/max، jitter و درصد گم‌شدن بسته.

#[derive(Debug, Clone, PartialEq)]
pub struct Summary {
    pub sent: usize,
    pub received: usize,
    pub loss_pct: f64,
    pub min_ms: Option<f64>,
    pub avg_ms: Option<f64>,
    pub median_ms: Option<f64>,
    pub p95_ms: Option<f64>,
    pub max_ms: Option<f64>,
    /// میانگین قدرمطلق اختلاف نمونه‌های متوالی (تعریف RFC 3550 ساده‌شده)
    pub jitter_ms: Option<f64>,
}

/// `samples`: برای هر تلاش، `Some(ms)` اگر موفق و `None` اگر ناموفق/timeout.
pub fn summarize(samples: &[Option<f64>]) -> Summary {
    let ok: Vec<f64> = samples.iter().flatten().copied().collect();
    let sent = samples.len();
    let received = ok.len();
    let loss_pct = if sent == 0 { 0.0 } else { (sent - received) as f64 * 100.0 / sent as f64 };
    if ok.is_empty() {
        return Summary { sent, received, loss_pct, min_ms: None, avg_ms: None, median_ms: None, p95_ms: None, max_ms: None, jitter_ms: None };
    }
    let mut sorted = ok.clone();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let pct = |p: f64| {
        let idx = ((p / 100.0) * (sorted.len() - 1) as f64).round() as usize;
        sorted[idx.min(sorted.len() - 1)]
    };
    let jitter = if ok.len() < 2 {
        None
    } else {
        Some(ok.windows(2).map(|w| (w[1] - w[0]).abs()).sum::<f64>() / (ok.len() - 1) as f64)
    };
    Summary {
        sent,
        received,
        loss_pct,
        min_ms: Some(sorted[0]),
        avg_ms: Some(ok.iter().sum::<f64>() / ok.len() as f64),
        median_ms: Some(pct(50.0)),
        p95_ms: Some(pct(95.0)),
        max_ms: Some(*sorted.last().unwrap()),
        jitter_ms: jitter,
    }
}

/// نمره‌ی کیفیت ۰ تا ۱۰۰ برای نمایش در داشبورد. فرمول ساده و شفاف:
/// از ۱۰۰ کم می‌شود: هر ۱۰ms تأخیر بالای ۲۰ms یک امتیاز، هر ۱ms jitter نیم امتیاز،
/// و هر ۱٪ loss پنج امتیاز.
pub fn quality_score(s: &Summary) -> u8 {
    let Some(avg) = s.avg_ms else { return 0 };
    let lat = ((avg - 20.0).max(0.0) / 10.0).min(40.0);
    let jit = (s.jitter_ms.unwrap_or(0.0) * 0.5).min(20.0);
    let loss = (s.loss_pct * 5.0).min(100.0);
    (100.0 - lat - jit - loss).clamp(0.0, 100.0).round() as u8
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn basic() {
        let s = summarize(&[Some(10.0), None, Some(20.0), Some(30.0)]);
        assert_eq!(s.sent, 4);
        assert_eq!(s.received, 3);
        assert_eq!(s.loss_pct, 25.0);
        assert_eq!(s.avg_ms, Some(20.0));
        assert_eq!(s.median_ms, Some(20.0));
        assert_eq!(s.jitter_ms, Some(10.0));
        assert!(quality_score(&s) < 100);
    }
    #[test]
    fn all_lost() {
        let s = summarize(&[None, None]);
        assert_eq!(s.loss_pct, 100.0);
        assert_eq!(quality_score(&s), 0);
    }
}
