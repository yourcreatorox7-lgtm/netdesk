//! تجزیه‌گر و نویسنده‌ی کوچک JSON (برای vmess:// و خروجی گزارش‌ها).
//! محدود و سخت‌گیر: عمق تودرتو و طول ورودی سقف دارد تا ورودی مخرب
//! (مثلاً اشتراک دست‌کاری‌شده) باعث مصرف بی‌حد حافظه/پشته نشود.

use std::collections::BTreeMap;

#[derive(Debug, Clone, PartialEq)]
pub enum Json {
    Null,
    Bool(bool),
    Num(f64),
    Str(String),
    Arr(Vec<Json>),
    Obj(BTreeMap<String, Json>),
}

pub const MAX_DEPTH: usize = 32;
pub const MAX_INPUT: usize = 1 << 20; // 1 MiB

impl Json {
    pub fn get(&self, k: &str) -> Option<&Json> {
        match self {
            Json::Obj(m) => m.get(k),
            _ => None,
        }
    }
    /// رشته یا عدد را به‌صورت متن برمی‌گرداند (vmess گاهی port را رشته می‌فرستد).
    pub fn as_text(&self) -> Option<String> {
        match self {
            Json::Str(s) => Some(s.clone()),
            Json::Num(n) if n.fract() == 0.0 => Some(format!("{}", *n as i64)),
            Json::Num(n) => Some(n.to_string()),
            _ => None,
        }
    }
    pub fn to_string(&self) -> String {
        let mut s = String::new();
        write(self, &mut s);
        s
    }
}

pub fn escape(s: &str, out: &mut String) {
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
}

fn write(j: &Json, out: &mut String) {
    match j {
        Json::Null => out.push_str("null"),
        Json::Bool(b) => out.push_str(if *b { "true" } else { "false" }),
        Json::Num(n) if n.is_finite() => out.push_str(&n.to_string()),
        Json::Num(_) => out.push_str("null"),
        Json::Str(s) => escape(s, out),
        Json::Arr(a) => {
            out.push('[');
            for (i, v) in a.iter().enumerate() {
                if i > 0 {
                    out.push(',');
                }
                write(v, out);
            }
            out.push(']');
        }
        Json::Obj(m) => {
            out.push('{');
            for (i, (k, v)) in m.iter().enumerate() {
                if i > 0 {
                    out.push(',');
                }
                escape(k, out);
                out.push(':');
                write(v, out);
            }
            out.push('}');
        }
    }
}

pub fn parse(input: &str) -> Result<Json, String> {
    if input.len() > MAX_INPUT {
        return Err("ورودی JSON بیش از حد بزرگ است".into());
    }
    let mut p = P { s: input.as_bytes(), i: 0 };
    let v = p.value(0)?;
    p.ws();
    if p.i != p.s.len() {
        return Err(format!("کاراکتر اضافه در موقعیت {}", p.i));
    }
    Ok(v)
}

struct P<'a> {
    s: &'a [u8],
    i: usize,
}

impl P<'_> {
    fn ws(&mut self) {
        while self.i < self.s.len() && matches!(self.s[self.i], b' ' | b'\n' | b'\r' | b'\t') {
            self.i += 1;
        }
    }
    fn lit(&mut self, w: &str, v: Json) -> Result<Json, String> {
        if self.s[self.i..].starts_with(w.as_bytes()) {
            self.i += w.len();
            Ok(v)
        } else {
            Err(format!("مقدار نامعتبر در موقعیت {}", self.i))
        }
    }
    fn value(&mut self, depth: usize) -> Result<Json, String> {
        if depth > MAX_DEPTH {
            return Err("تودرتویی JSON بیش از حد مجاز".into());
        }
        self.ws();
        match self.s.get(self.i) {
            None => Err("پایان غیرمنتظره‌ی JSON".into()),
            Some(b'n') => self.lit("null", Json::Null),
            Some(b't') => self.lit("true", Json::Bool(true)),
            Some(b'f') => self.lit("false", Json::Bool(false)),
            Some(b'"') => self.string().map(Json::Str),
            Some(b'[') => {
                self.i += 1;
                let mut a = Vec::new();
                self.ws();
                if self.s.get(self.i) == Some(&b']') {
                    self.i += 1;
                    return Ok(Json::Arr(a));
                }
                loop {
                    a.push(self.value(depth + 1)?);
                    self.ws();
                    match self.s.get(self.i) {
                        Some(b',') => self.i += 1,
                        Some(b']') => {
                            self.i += 1;
                            return Ok(Json::Arr(a));
                        }
                        _ => return Err(format!("انتظار ',' یا ']' در {}", self.i)),
                    }
                }
            }
            Some(b'{') => {
                self.i += 1;
                let mut m = BTreeMap::new();
                self.ws();
                if self.s.get(self.i) == Some(&b'}') {
                    self.i += 1;
                    return Ok(Json::Obj(m));
                }
                loop {
                    self.ws();
                    if self.s.get(self.i) != Some(&b'"') {
                        return Err(format!("انتظار کلید در {}", self.i));
                    }
                    let k = self.string()?;
                    self.ws();
                    if self.s.get(self.i) != Some(&b':') {
                        return Err(format!("انتظار ':' در {}", self.i));
                    }
                    self.i += 1;
                    let v = self.value(depth + 1)?;
                    m.insert(k, v);
                    self.ws();
                    match self.s.get(self.i) {
                        Some(b',') => self.i += 1,
                        Some(b'}') => {
                            self.i += 1;
                            return Ok(Json::Obj(m));
                        }
                        _ => return Err(format!("انتظار ',' یا '}}' در {}", self.i)),
                    }
                }
            }
            Some(_) => self.number(),
        }
    }
    fn number(&mut self) -> Result<Json, String> {
        let st = self.i;
        while self.i < self.s.len() && matches!(self.s[self.i], b'-' | b'+' | b'.' | b'e' | b'E' | b'0'..=b'9') {
            self.i += 1;
        }
        std::str::from_utf8(&self.s[st..self.i])
            .ok()
            .and_then(|t| t.parse::<f64>().ok())
            .map(Json::Num)
            .ok_or_else(|| format!("عدد نامعتبر در {}", st))
    }
    fn string(&mut self) -> Result<String, String> {
        self.i += 1; // "
        let mut out = String::new();
        loop {
            let rest = std::str::from_utf8(&self.s[self.i..]).map_err(|_| "UTF-8 نامعتبر")?;
            let mut chars = rest.char_indices();
            let Some((_, c)) = chars.next() else { return Err("رشته‌ی بسته‌نشده".into()) };
            self.i += c.len_utf8();
            match c {
                '"' => return Ok(out),
                '\\' => {
                    let e = *self.s.get(self.i).ok_or("escape ناقص")?;
                    self.i += 1;
                    match e {
                        b'"' => out.push('"'),
                        b'\\' => out.push('\\'),
                        b'/' => out.push('/'),
                        b'b' => out.push('\u{8}'),
                        b'f' => out.push('\u{c}'),
                        b'n' => out.push('\n'),
                        b'r' => out.push('\r'),
                        b't' => out.push('\t'),
                        b'u' => {
                            let h = self.s.get(self.i..self.i + 4).ok_or("\\u ناقص")?;
                            let cp = u32::from_str_radix(std::str::from_utf8(h).map_err(|_| "hex")?, 16)
                                .map_err(|_| "hex نامعتبر")?;
                            self.i += 4;
                            out.push(char::from_u32(cp).unwrap_or('\u{FFFD}'));
                        }
                        _ => return Err("escape ناشناخته".into()),
                    }
                }
                c => out.push(c),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn parse_and_write() {
        let j = parse(r#"{"a":[1,2.5,"x\n"],"b":{"c":null,"d":true},"e":"سلام"}"#).unwrap();
        assert_eq!(j.get("e").unwrap().as_text().unwrap(), "سلام");
        let again = parse(&j.to_string()).unwrap();
        assert_eq!(j, again);
    }
    #[test]
    fn rejects_deep_and_trailing() {
        let deep = "[".repeat(100) + &"]".repeat(100);
        assert!(parse(&deep).is_err());
        assert!(parse("{} x").is_err());
    }
}
