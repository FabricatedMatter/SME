//! Starbound's JSON dialect: standard JSON plus `//` and `/* */` comments (and the odd
//! trailing comma in community files). We strip those and hand the rest to serde_json.

use anyhow::{Context, Result};
use serde_json::Value;

pub fn clean(src: &str) -> String {
    let b = src.as_bytes();
    let mut out = Vec::with_capacity(b.len());
    let mut i = 0;
    let mut in_str = false;
    while i < b.len() {
        let c = b[i];
        if in_str {
            out.push(c);
            if c == b'\\' && i + 1 < b.len() {
                out.push(b[i + 1]);
                i += 2;
                continue;
            }
            if c == b'"' {
                in_str = false;
            }
            i += 1;
            continue;
        }
        match c {
            b'"' => {
                in_str = true;
                out.push(c);
                i += 1;
            }
            b'/' if b.get(i + 1) == Some(&b'/') => {
                while i < b.len() && b[i] != b'\n' {
                    i += 1;
                }
            }
            b'/' if b.get(i + 1) == Some(&b'*') => {
                i += 2;
                while i + 1 < b.len() && !(b[i] == b'*' && b[i + 1] == b'/') {
                    i += 1;
                }
                i += 2;
            }
            b',' => {
                // drop trailing commas: ", }" / ", ]"
                let mut j = i + 1;
                loop {
                    while j < b.len() && b[j].is_ascii_whitespace() {
                        j += 1;
                    }
                    if b.get(j) == Some(&b'/') && b.get(j + 1) == Some(&b'/') {
                        while j < b.len() && b[j] != b'\n' {
                            j += 1;
                        }
                        continue;
                    }
                    if b.get(j) == Some(&b'/') && b.get(j + 1) == Some(&b'*') {
                        j += 2;
                        while j + 1 < b.len() && !(b[j] == b'*' && b[j + 1] == b'/') {
                            j += 1;
                        }
                        j += 2;
                        continue;
                    }
                    break;
                }
                if !matches!(b.get(j), Some(b'}') | Some(b']')) {
                    out.push(c);
                }
                i += 1;
            }
            _ => {
                out.push(c);
                i += 1;
            }
        }
    }
    String::from_utf8(out).unwrap_or_default()
}

pub fn parse(bytes: &[u8]) -> Result<Value> {
    let s = String::from_utf8_lossy(bytes);
    let s = s.strip_prefix('\u{feff}').unwrap_or(&s);
    match serde_json::from_str(s) {
        Ok(v) => Ok(v),
        Err(_) => serde_json::from_str(&clean(s)).context("invalid JSON"),
    }
}

/// Pretty JSON in the style of vanilla assets (2-space indent).
pub fn pretty(v: &Value) -> String {
    let mut s = serde_json::to_string_pretty(v).unwrap_or_default();
    s.push('\n');
    s
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn comments_and_commas() {
        let v = parse(br#"{ "a": "x//y", // c
            /* block */ "b": [1, 2, ], }"#)
        .unwrap();
        assert_eq!(v["a"], "x//y");
        assert_eq!(v["b"].as_array().unwrap().len(), 2);
    }
}
