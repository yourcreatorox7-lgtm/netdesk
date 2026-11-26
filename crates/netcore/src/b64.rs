//! رمزگشایی/رمزگذاری Base64 (استاندارد و URL-safe، با یا بدون padding).

const STD: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

pub fn encode(input: &[u8]) -> String {
    let mut out = String::with_capacity((input.len() + 2) / 3 * 4);
    for chunk in input.chunks(3) {
        let b = [chunk[0], *chunk.get(1).unwrap_or(&0), *chunk.get(2).unwrap_or(&0)];
        let n = ((b[0] as u32) << 16) | ((b[1] as u32) << 8) | b[2] as u32;
        out.push(STD[(n >> 18) as usize & 63] as char);
        out.push(STD[(n >> 12) as usize & 63] as char);
        out.push(if chunk.len() > 1 { STD[(n >> 6) as usize & 63] as char } else { '=' });
        out.push(if chunk.len() > 2 { STD[n as usize & 63] as char } else { '=' });
    }
    out
}

fn val(c: u8) -> Option<u32> {
    Some(match c {
        b'A'..=b'Z' => c - b'A',
        b'a'..=b'z' => c - b'a' + 26,
        b'0'..=b'9' => c - b'0' + 52,
        b'+' | b'-' => 62,
        b'/' | b'_' => 63,
        _ => return None,
    } as u32)
}

/// هر دو الفبای استاندارد و URL-safe را می‌پذیرد؛ فاصله و padding نادیده گرفته می‌شود.
pub fn decode(input: &str) -> Option<Vec<u8>> {
    let clean: Vec<u8> = input.bytes().filter(|c| !c.is_ascii_whitespace() && *c != b'=').collect();
    if clean.len() % 4 == 1 {
        return None;
    }
    let mut out = Vec::with_capacity(clean.len() * 3 / 4);
    for chunk in clean.chunks(4) {
        let mut n = 0u32;
        for (i, &c) in chunk.iter().enumerate() {
            n |= val(c)? << (18 - 6 * i);
        }
        out.push((n >> 16) as u8);
        if chunk.len() > 2 {
            out.push((n >> 8) as u8);
        }
        if chunk.len() > 3 {
            out.push(n as u8);
        }
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn roundtrip() {
        for s in ["", "f", "fo", "foo", "foob", "fooba", "foobar", "سلام"] {
            assert_eq!(decode(&encode(s.as_bytes())).unwrap(), s.as_bytes());
        }
        assert_eq!(decode("Zm9vYg").unwrap(), b"foob");
        assert!(decode("@@@@").is_none());
    }
}
