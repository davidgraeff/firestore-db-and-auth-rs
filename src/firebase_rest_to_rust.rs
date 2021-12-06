//! # Low Level API to convert between rust types and the Firebase REST API
//! Low level API to convert between generated rust types (see [`crate::dto`]) and
//! the data types of the Firebase REST API. Those are 1:1 translations of the grpc API
//! and deeply nested and wrapped.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use bytes::Bytes;

use super::dto;
use super::errors::{FirebaseError, Result};

#[derive(Debug, Serialize, Deserialize)]
struct Wrapper {
    #[serde(flatten)]
    extra: HashMap<String, Value>,
}

use serde_json::{map::Map, Number};

/// Converts a firebase google-rpc-api inspired heavily nested and wrapped response value
/// of the Firebase REST API into a flattened serde json value.
///
/// This is a low level API. You probably want to use [`crate::documents`] instead.
///
/// This method works recursively!
pub(crate) fn firebase_value_to_serde_value(v: &dto::Value) -> serde_json::Value {
    if let Some(timestamp_value) = v.timestamp_value.as_ref() {
        return Value::String(timestamp_value.clone());
    } else if let Some(integer_value) = v.integer_value.as_ref() {
        if let Ok(four) = integer_value.parse::<i64>() {
            return Value::Number(four.into());
        }
    } else if let Some(double_value) = v.double_value {
        if let Some(dd) = Number::from_f64(double_value) {
            return Value::Number(dd);
        }
    } else if let Some(map_value) = v.map_value.as_ref() {
        let mut map: Map<String, serde_json::value::Value> = Map::new();
        if let Some(map_fields) = &map_value.fields {
            for (map_key, map_v) in map_fields {
                map.insert(map_key.clone(), firebase_value_to_serde_value(&map_v));
            }
        }
        return Value::Object(map);
    } else if let Some(string_value) = v.string_value.as_ref() {
        return Value::String(string_value.clone());
    } else if let Some(boolean_value) = v.boolean_value {
        return Value::Bool(boolean_value);
    } else if let Some(array_value) = v.array_value.as_ref() {
        let mut vec: Vec<Value> = Vec::new();
        if let Some(values) = &array_value.values {
            for k in values {
                vec.push(firebase_value_to_serde_value(&k));
            }
        }
        return Value::Array(vec);
    }
    Value::Null
}

/// Converts a flat serde json value into a firebase google-rpc-api inspired heavily nested and wrapped type
/// to be consumed by the Firebase REST API.
///
/// This is a low level API. You probably want to use [`crate::documents`] instead.
///
/// This method works recursively!
pub(crate) fn serde_value_to_firebase_value(v: &serde_json::Value) -> dto::Value {
    if v.is_f64() {
        return dto::Value {
            double_value: Some(v.as_f64().unwrap()),
            ..Default::default()
        };
    } else if let Some(integer_value) = v.as_i64() {
        return dto::Value {
            integer_value: Some(integer_value.to_string()),
            ..Default::default()
        };
    } else if let Some(map_value) = v.as_object() {
        let mut map: HashMap<String, dto::Value> = HashMap::new();
        for (map_key, map_v) in map_value {
            map.insert(map_key.to_owned(), serde_value_to_firebase_value(&map_v));
        }
        return dto::Value {
            map_value: Some(dto::MapValue { fields: Some(map) }),
            ..Default::default()
        };
    } else if let Some(string_value) = v.as_str() {
        return dto::Value {
            string_value: Some(string_value.to_owned()),
            ..Default::default()
        };
    } else if let Some(boolean_value) = v.as_bool() {
        return dto::Value {
            boolean_value: Some(boolean_value),
            ..Default::default()
        };
    } else if let Some(array_value) = v.as_array() {
        let mut vec: Vec<dto::Value> = Vec::new();
        for k in array_value {
            vec.push(serde_value_to_firebase_value(&k));
        }
        return dto::Value {
            array_value: Some(dto::ArrayValue { values: Some(vec) }),
            ..Default::default()
        };
    }
    Default::default()
}

/// Converts a firebase google-rpc-api inspired heavily nested and wrapped response document
/// of the Firebase REST API into a given custom type.
///
/// This is a low level API. You probably want to use [`crate::documents`] instead.
///
/// Internals:
///
/// This method uses recursion to decode the given firebase type.
pub fn document_to_pod<T>(input_doc: &Bytes, document: &dto::Document) -> Result<T>
where
    for<'de> T: Deserialize<'de>,
{
    // The firebase document has a field called "fields" that contain all top-level fields.
    // We want those to be flattened to our custom data structure. To not reinvent the wheel,
    // perform the firebase-value to serde-values conversion for all fields first and wrap those
    // Wrapper struct with a HashMap. Use #[serde(flatten)] on that map.
    let r = Wrapper {
        extra: document
            .fields
            .as_ref()
            .unwrap()
            .iter()
            .map(|(k, v)| {
                return (k.to_owned(), firebase_value_to_serde_value(&v));
            })
            .collect(),
    };

    let v = serde_json::to_value(r)?;
    let r: T = serde_json::from_value(v).map_err(|e| FirebaseError::SerdeVerbose {
        doc: Some(document.name.clone()),
        input_doc: format!("{:?}", input_doc),
        ser: e,
    })?;
    Ok(r)
}

/// Converts a custom data type into a firebase google-rpc-api inspired heavily nested and wrapped type
/// to be consumed by the Firebase REST API.
///
/// This is a low level API. You probably want to use [`crate::documents`] instead.
///
/// Internals:
///
/// This method uses recursion to decode the given firebase type.
pub fn pod_to_document<T>(pod: &T) -> Result<dto::Document>
where
    T: Serialize,
{
    let v = serde_json::to_value(pod)?;
    Ok(dto::Document {
        fields: serde_value_to_firebase_value(&v).map_value.unwrap().fields,
        ..Default::default()
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    use super::Result;
    use serde::{Deserialize, Serialize};
    use std::collections::HashMap;

    #[derive(Serialize, Deserialize)]
    struct DemoPod {
        integer_test: u32,
        boolean_test: bool,
        string_test: String,
    }

    #[test]
    fn test_document_to_pod() -> Result<()> {
        let mut map: HashMap<String, dto::Value> = HashMap::new();
        map.insert(
            "integer_test".to_owned(),
            dto::Value {
                integer_value: Some("12".to_owned()),
                ..Default::default()
            },
        );
        map.insert(
            "boolean_test".to_owned(),
            dto::Value {
                boolean_value: Some(true),
                ..Default::default()
            },
        );
        map.insert(
            "string_test".to_owned(),
            dto::Value {
                string_value: Some("abc".to_owned()),
                ..Default::default()
            },
        );
        let t = dto::Document {
            fields: Some(map),
            ..Default::default()
        };
        let firebase_doc: DemoPod = document_to_pod(&t)?;
        assert_eq!(firebase_doc.string_test, "abc");
        assert_eq!(firebase_doc.integer_test, 12);
        assert_eq!(firebase_doc.boolean_test, true);

        Ok(())
    }

    #[test]
    fn test_pod_to_document() -> Result<()> {
        let t = DemoPod {
            integer_test: 12,
            boolean_test: true,
            string_test: "abc".to_owned(),
        };
        let firebase_doc = pod_to_document(&t)?;
        let map = firebase_doc.fields;
        assert_eq!(
            map.unwrap()
                .get("integer_test")
                .expect("a value in the map for integer_test")
                .integer_value
                .as_ref()
                .expect("an integer value"),
            "12"
        );

        Ok(())
    }
}
