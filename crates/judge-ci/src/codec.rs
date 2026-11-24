//! Reject repeated JSON members before binding the scoped request, raw receipts, or host declarations.
use serde::Deserialize;
use serde_json::Value;
fn strict_json_value<'de, D: serde::Deserializer<'de>>(d: D) -> Result<Value, D::Error> {
    struct Strict;
    impl<'de> serde::de::Visitor<'de> for Strict {
        type Value = Value;
        fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
            f.write_str("JSON without duplicate members")
        }
        fn visit_bool<E: serde::de::Error>(self, v: bool) -> Result<Value, E> {
            Ok(v.into())
        }
        fn visit_i64<E: serde::de::Error>(self, v: i64) -> Result<Value, E> {
            Ok(v.into())
        }
        fn visit_u64<E: serde::de::Error>(self, v: u64) -> Result<Value, E> {
            Ok(v.into())
        }
        fn visit_f64<E: serde::de::Error>(self, v: f64) -> Result<Value, E> {
            serde_json::Number::from_f64(v)
                .map(Value::Number)
                .ok_or_else(|| E::custom("nonfinite number"))
        }
        fn visit_str<E: serde::de::Error>(self, v: &str) -> Result<Value, E> {
            Ok(v.into())
        }
        fn visit_string<E: serde::de::Error>(self, v: String) -> Result<Value, E> {
            Ok(v.into())
        }
        fn visit_unit<E: serde::de::Error>(self) -> Result<Value, E> {
            Ok(Value::Null)
        }
        fn visit_none<E: serde::de::Error>(self) -> Result<Value, E> {
            Ok(Value::Null)
        }
        fn visit_seq<A: serde::de::SeqAccess<'de>>(self, mut a: A) -> Result<Value, A::Error> {
            let mut out = vec![];
            while let Some(v) = a.next_element::<StrictValue>()? {
                out.push(v.0)
            }
            Ok(out.into())
        }
        fn visit_map<A: serde::de::MapAccess<'de>>(self, mut a: A) -> Result<Value, A::Error> {
            let mut out = serde_json::Map::new();
            while let Some(k) = a.next_key::<String>()? {
                if out.contains_key(&k) {
                    return Err(serde::de::Error::custom(format!(
                        "duplicate JSON member {k}"
                    )));
                }
                out.insert(k, a.next_value::<StrictValue>()?.0);
            }
            Ok(Value::Object(out))
        }
    }
    struct StrictValue(Value);
    impl<'de> Deserialize<'de> for StrictValue {
        fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
            d.deserialize_any(Strict).map(StrictValue)
        }
    }
    d.deserialize_any(Strict)
}
pub fn json(bytes: &[u8]) -> Result<Value, String> {
    let mut d = serde_json::Deserializer::from_slice(bytes);
    let v = strict_json_value(&mut d).map_err(|e| e.to_string())?;
    d.end().map_err(|e| e.to_string())?;
    Ok(v)
}
pub fn decode<T: serde::de::DeserializeOwned>(bytes: &[u8]) -> Result<T, String> {
    serde_json::from_value(json(bytes)?).map_err(|e| e.to_string())
}

pub fn digest<T: serde::Serialize>(value: &T) -> Result<String, String> {
    Ok(crate::files::sha256(
        &serde_jcs::to_vec(value).map_err(|e| e.to_string())?,
    ))
}
