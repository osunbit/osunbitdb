use serde_json::{Value as JsonValue, Map};
use serde::{Serialize, de::DeserializeOwned};
use crate::errors::OsunbitDBError;
use bincode;

pub fn encode<T: Serialize>(value: &T) -> Result<Vec<u8>, OsunbitDBError> {
    Ok(bincode::serialize(value)?)
}

pub fn decode<T: DeserializeOwned>(bytes: &[u8]) -> Result<T, OsunbitDBError> {
    Ok(bincode::deserialize(bytes)?)
}

/// Dot-notation: set deeply
pub fn set_deep(obj: &mut Map<String, JsonValue>, path: &str, value: JsonValue) {
    let mut parts = path.split('.').peekable();
    let mut current = obj;

    while let Some(part) = parts.next() {
        if parts.peek().is_none() {
            // Last segment → insert (clone to avoid move issues)
            current.insert(part.to_string(), value.clone());
        } else {
            // Ensure object exists
            current = current
                .entry(part)
                .or_insert_with(|| JsonValue::Object(Map::new()))
                .as_object_mut()
                .unwrap();
        }
    }
}


/// Dot-notation: get deeply
pub fn get_deep<'a>(obj: &'a Map<String, JsonValue>, path: &str) -> Option<&'a JsonValue> {
    let mut current = obj;
    let mut parts = path.split('.').peekable();

    while let Some(part) = parts.next() {
        if parts.peek().is_none() {
            return current.get(part);
        } else {
            match current.get(part) {
                Some(JsonValue::Object(map)) => current = map,
                _ => return None,
            }
        }
    }
    None
}

/// Dot-notation: remove a nested field safely
pub fn remove_deep(obj: &mut Map<String, JsonValue>, path: &str) {
    let mut parts: Vec<&str> = path.split('.').collect();
    if parts.is_empty() {
        return;
    }

    let last = parts.pop().unwrap();
    let mut current = obj;

    for part in parts {
        match current.get_mut(part) {
            Some(JsonValue::Object(map)) => current = map,
            _ => return, // intermediate key missing or not an object → stop
        }
    }

    // Remove the final key if it exists
    current.remove(last);
}

