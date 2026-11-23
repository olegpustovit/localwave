//! A small JSON reader for bundled, trusted documents (the demo catalog, the reference fixture): objects keep their key order,
//! numbers are f64, and a malformed document is an error, never a panic.
#[derive(Debug, PartialEq)]
pub enum Json {
    Null,
    Bool(bool),
    Num(f64),
    Str(String),
    Arr(Vec<Json>),
    Obj(Vec<(String, Json)>),
}

impl Json {
    pub fn get(&self, key: &str) -> Option<&Json> {
        match self {
            Json::Obj(items) => items.iter().find(|(k, _)| k == key).map(|(_, v)| v),
            _ => None,
        }
    }
    pub fn str(&self, key: &str) -> &str {
        match self.get(key) {
            Some(Json::Str(s)) => s,
            _ => "",
        }
    }
    pub fn num(&self, key: &str) -> f64 {
        match self.get(key) {
            Some(Json::Num(n)) => *n,
            _ => 0.0,
        }
    }
    pub fn flag(&self, key: &str) -> bool {
        matches!(self.get(key), Some(Json::Bool(true)))
    }
    pub fn items(&self, key: &str) -> &[Json] {
        match self.get(key) {
            Some(Json::Arr(items)) => items,
            _ => &[],
        }
    }
}

pub fn parse(text: &str) -> Result<Json, String> {
    let mut p = (text.as_bytes(), 0usize);
    let value = value(&mut p)?;
    ws(&mut p);
    if p.1 != p.0.len() {
        return Err("Trailing JSON data".into());
    }
    Ok(value)
}

type P<'a> = (&'a [u8], usize);
fn ws(p: &mut P) {
    while p.0.get(p.1).is_some_and(u8::is_ascii_whitespace) {
        p.1 += 1;
    }
}
fn eat(p: &mut P, byte: u8) -> Result<(), String> {
    ws(p);
    if p.0.get(p.1) == Some(&byte) {
        p.1 += 1;
        Ok(())
    } else {
        Err(format!("Expected '{}' at byte {}", byte as char, p.1))
    }
}
fn value(p: &mut P) -> Result<Json, String> {
    ws(p);
    match p.0.get(p.1).copied() {
        Some(b'{') => {
            p.1 += 1;
            let mut items = Vec::new();
            ws(p);
            if p.0.get(p.1) == Some(&b'}') {
                p.1 += 1;
                return Ok(Json::Obj(items));
            }
            loop {
                ws(p);
                let key = string(p)?;
                eat(p, b':')?;
                items.push((key, value(p)?));
                ws(p);
                match p.0.get(p.1) {
                    Some(b',') => p.1 += 1,
                    Some(b'}') => {
                        p.1 += 1;
                        return Ok(Json::Obj(items));
                    }
                    _ => return Err(format!("Bad object at byte {}", p.1)),
                }
            }
        }
        Some(b'[') => {
            p.1 += 1;
            let mut items = Vec::new();
            ws(p);
            if p.0.get(p.1) == Some(&b']') {
                p.1 += 1;
                return Ok(Json::Arr(items));
            }
            loop {
                items.push(value(p)?);
                ws(p);
                match p.0.get(p.1) {
                    Some(b',') => p.1 += 1,
                    Some(b']') => {
                        p.1 += 1;
                        return Ok(Json::Arr(items));
                    }
                    _ => return Err(format!("Bad array at byte {}", p.1)),
                }
            }
        }
        Some(b'"') => Ok(Json::Str(string(p)?)),
        Some(b't') if p.0[p.1..].starts_with(b"true") => {
            p.1 += 4;
            Ok(Json::Bool(true))
        }
        Some(b'f') if p.0[p.1..].starts_with(b"false") => {
            p.1 += 5;
            Ok(Json::Bool(false))
        }
        Some(b'n') if p.0[p.1..].starts_with(b"null") => {
            p.1 += 4;
            Ok(Json::Null)
        }
        Some(_) => {
            let start = p.1;
            while p.0.get(p.1).is_some_and(|b| b"+-.eE0123456789".contains(b)) {
                p.1 += 1;
            }
            std::str::from_utf8(&p.0[start..p.1])
                .ok()
                .and_then(|s| s.parse().ok())
                .map(Json::Num)
                .ok_or_else(|| format!("Bad value at byte {start}"))
        }
        None => Err("Unexpected end of JSON".into()),
    }
}
fn string(p: &mut P) -> Result<String, String> {
    if p.0.get(p.1) != Some(&b'"') {
        return Err(format!("Expected string at byte {}", p.1));
    }
    p.1 += 1;
    let mut out: Vec<u8> = Vec::new();
    let mut high: Option<u32> = None;
    loop {
        let byte = *p.0.get(p.1).ok_or("Unterminated string")?;
        p.1 += 1;
        match byte {
            b'"' => return String::from_utf8(out).map_err(|e| e.to_string()),
            b'\\' => {
                let escape = *p.0.get(p.1).ok_or("Unterminated escape")?;
                p.1 += 1;
                let c = match escape {
                    b'n' => '\n',
                    b't' => '\t',
                    b'r' => '\r',
                    b'b' => '\u{8}',
                    b'f' => '\u{c}',
                    b'u' => {
                        let hex = p.0.get(p.1..p.1 + 4).ok_or("Short \\u escape")?;
                        let unit = u32::from_str_radix(std::str::from_utf8(hex).map_err(|e| e.to_string())?, 16)
                            .map_err(|e| e.to_string())?;
                        p.1 += 4;
                        let code = match (high.take(), unit) {
                            (Some(h), 0xDC00..=0xDFFF) => 0x10000 + ((h - 0xD800) << 10) + (unit - 0xDC00),
                            (_, 0xD800..=0xDBFF) => {
                                high = Some(unit);
                                continue;
                            }
                            _ => unit,
                        };
                        char::from_u32(code).ok_or("Bad \\u escape")?
                    }
                    other => other as char,
                };
                out.extend_from_slice(c.encode_utf8(&mut [0; 4]).as_bytes());
            }
            _ => out.push(byte),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_escapes_and_nesting() {
        let json = parse(r#"{"a":[1,-2.5,true,null],"s":"x·😀\n","o":{}}"#).unwrap();
        assert_eq!(json.items("a").len(), 4);
        assert_eq!(json.str("s"), "x\u{b7}\u{1f600}\n");
        assert!(parse("{\"a\":1} x").is_err());
    }
}
