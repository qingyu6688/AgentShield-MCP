//! 敏感字段脱敏。审计落盘前调用，保证 Token / 密码 / 密钥不明文存储。

use once_cell::sync::Lazy;
use regex::Regex;

/// 命中即整体替换的敏感 key 名（大小写不敏感）。
const SENSITIVE_KEYS: &[&str] = &[
    "token",
    "password",
    "passwd",
    "secret",
    "api_key",
    "apikey",
    "authorization",
    "access_key",
    "private_key",
];

/// 看起来像密钥 / token 的值模式。
static SECRET_VALUE: Lazy<Regex> = Lazy::new(|| {
    // 私钥块、sk- 前缀的 key、较长的连续凭据串
    Regex::new(r"-----BEGIN [A-Z ]*PRIVATE KEY-----|sk-[A-Za-z0-9]{16,}|[A-Za-z0-9_\-]{32,}")
        .expect("脱敏正则应当合法")
});

/// 对单个字符串脱敏：命中密钥模式的片段打码，保留首尾各 2 字符。
pub fn redact(input: &str) -> String {
    SECRET_VALUE
        .replace_all(input, |caps: &regex::Captures| mask(&caps[0]))
        .into_owned()
}

/// 对 json 值递归脱敏：按 key 名命中的整体打码，其余字符串按值模式脱敏。
pub fn redact_json(value: &mut serde_json::Value) {
    match value {
        serde_json::Value::Object(map) => {
            for (k, v) in map.iter_mut() {
                if is_sensitive_key(k) {
                    *v = serde_json::Value::String("***".to_string());
                } else {
                    redact_json(v);
                }
            }
        }
        serde_json::Value::Array(arr) => {
            for v in arr.iter_mut() {
                redact_json(v);
            }
        }
        serde_json::Value::String(s) => {
            *s = redact(s);
        }
        _ => {}
    }
}

fn is_sensitive_key(key: &str) -> bool {
    let lower = key.to_ascii_lowercase();
    SENSITIVE_KEYS.iter().any(|k| lower.contains(k))
}

/// 保留首尾各 2 字符，中间打码；过短则整体打码。
fn mask(s: &str) -> String {
    let chars: Vec<char> = s.chars().collect();
    if chars.len() <= 6 {
        return "***".to_string();
    }
    let head: String = chars[..2].iter().collect();
    let tail: String = chars[chars.len() - 2..].iter().collect();
    format!("{head}***{tail}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn redacts_sensitive_keys() {
        let mut v = serde_json::json!({
            "path": "./src/main.rs",
            "token": "abcd1234efgh5678",
            "nested": { "password": "hunter2" }
        });
        redact_json(&mut v);
        assert_eq!(v["token"], "***");
        assert_eq!(v["nested"]["password"], "***");
        // 普通字段不动
        assert_eq!(v["path"], "./src/main.rs");
    }

    #[test]
    fn masks_secret_like_values() {
        let out = redact("my key is sk-ABCDEFGHIJKLMNOPQRST done");
        assert!(!out.contains("sk-ABCDEFGHIJKLMNOPQRST"));
        assert!(out.contains("***"));
    }

    #[test]
    fn keeps_normal_text() {
        assert_eq!(redact("hello world"), "hello world");
    }
}
