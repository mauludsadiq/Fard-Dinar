use serde::Serialize;
use serde_json::{Number, Value};

pub fn canonical_json_bytes<T: Serialize>(value: &T) -> Result<Vec<u8>, serde_json::Error> {
    let value = serde_json::to_value(value)?;
    Ok(canonicalize_value(&value).into_bytes())
}

pub fn canonical_json_string<T: Serialize>(value: &T) -> Result<String, serde_json::Error> {
    let bytes = canonical_json_bytes(value)?;
    Ok(String::from_utf8(bytes).expect("canonical JSON must be utf-8"))
}

fn canonicalize_value(value: &Value) -> String {
    match value {
        Value::Null => "null".to_string(),
        Value::Bool(b) => b.to_string(),
        Value::Number(n) => canonicalize_number(n),
        Value::String(s) => serde_json::to_string(s).expect("string serialization cannot fail"),
        Value::Array(items) => {
            let inner = items.iter().map(canonicalize_value).collect::<Vec<_>>().join(",");
            format!("[{inner}]")
        }
        Value::Object(map) => {
            let mut keys = map.keys().cloned().collect::<Vec<_>>();
            keys.sort_by(|a, b| a.as_bytes().cmp(b.as_bytes()));
            let inner = keys
                .into_iter()
                .map(|k| {
                    let key = serde_json::to_string(&k).expect("key serialization cannot fail");
                    let value = canonicalize_value(map.get(&k).expect("key must exist"));
                    format!("{key}:{value}")
                })
                .collect::<Vec<_>>()
                .join(",");
            format!("{{{inner}}}")
        }
    }
}

fn canonicalize_number(n: &Number) -> String {
    n.to_string()
}
