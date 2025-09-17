use tikv_client::{Transaction, Key, Value, KvPair, BoundRange};
use serde_json::{Value as JsonValue, Map};
use crate::errors::OsunbitDBError;
use crate::utils::{set_deep, get_deep, remove_deep};
use serde_json::json;


pub struct TransactionHandle {
    pub(crate) tx: Transaction,
}

impl TransactionHandle {
    fn key(base: &str, id: &str) -> Key {
        let key = format!("{}:{}", base, id);
        Key::from(key)
    }

    pub async fn add(&mut self, collection: &str, id: &str, value: &JsonValue) -> Result<(), OsunbitDBError> {
        let bytes = serde_json::to_vec(value)?;
        self.tx.put(Self::key(collection, id), Value::from(bytes)).await?;
        Ok(())
    }

    pub async fn get(&mut self, collection: &str, id: &str) -> Result<Option<JsonValue>, OsunbitDBError> {
        let bytes_opt = self.tx.get(Self::key(collection, id)).await?;
        if let Some(bytes) = bytes_opt {
            let json: JsonValue = serde_json::from_slice(&bytes)?;
            Ok(Some(json))
        } else {
            Ok(None)
        }
    }

    pub async fn delete(&mut self, collection: &str, id: &str) -> Result<(), OsunbitDBError> {
        self.tx.delete(Self::key(collection, id)).await?;
        Ok(())
    }


pub async fn update(
    &mut self,
    collection: &str,
    id: &str,
    fields: &JsonValue,
) -> Result<(), OsunbitDBError> {
    let mut data = self.get(collection, id).await?.unwrap_or(JsonValue::Object(Map::new()));

    if let JsonValue::Object(ref mut obj) = data {
        if let JsonValue::Object(new_fields) = fields {
            for (k, v) in new_fields {
                if let Some(op) = v.get("__op") {
                    match op.as_str().unwrap_or("") {
                        "inc" => {
                            let delta = v["amount"].as_i64().unwrap_or(0);
                            let mut current_val = 0;
                            if let Some(existing) = get_deep(obj, k).and_then(|val| val.as_i64()) {
                                current_val = existing;
                            }
                            set_deep(obj, k, json!(current_val + delta));
                        }
                        "remove" => {
                            remove_deep(obj, k);
                        }
                        "array_union" => {
                            let new_vals = v["values"].as_array().cloned().unwrap_or_default();
                            let mut existing = get_deep(obj, k)
                                .and_then(|val| val.as_array().cloned())
                                .unwrap_or_default();

                            for nv in new_vals {
                                if !existing.contains(&nv) {
                                    existing.push(nv);
                                }
                            }

                            set_deep(obj, k, JsonValue::Array(existing));
                        }
                        "array_remove" => {
                            let rem_vals = v["values"].as_array().cloned().unwrap_or_default();
                            let mut existing = get_deep(obj, k)
                                .and_then(|val| val.as_array().cloned())
                                .unwrap_or_default();

                            existing.retain(|item| !rem_vals.contains(item));

                            set_deep(obj, k, JsonValue::Array(existing));
                        }
                        _ => {
                            set_deep(obj, k, v.clone());
                        }
                    }
                } else {
                    set_deep(obj, k, v.clone());
                }
            }
        } else {
            return Err(OsunbitDBError::InvalidUpdate(
                "update fields must be an object".to_string(),
            ));
        }
    }

    // Persist the updated document
    self.add(collection, id, &data).await?;

    // ===== TTL handling: look for expiryAt directly (string value)
    if let Some(expiry_val) = get_deep(
        data.as_object().expect("doc must be object"),
        "expiryAt",
    ) {
        if let Some(expiry_str) = expiry_val.as_str() {
            // key format: expire:<expiryAt>:<docid>
            let expire_key = Key::from(format!("expire:{}:{}", expiry_str, id));
            let expire_val = Value::from(id.as_bytes().to_vec());
            self.tx.put(expire_key, expire_val).await?;
        }
    }

    Ok(())
}

    pub async fn commit(mut self) -> Result<(), OsunbitDBError> {
        self.tx.commit().await?;
        Ok(())
    }

    pub async fn rollback(mut self) -> Result<(), OsunbitDBError> {
        self.tx.rollback().await?;
        Ok(())
    }
pub async fn scan(
    &mut self,
    collection: &str,
    limit: u32,
    cursor: &str,
) -> Result<JsonValue, OsunbitDBError> {
    let prefix = format!("{}:", collection);

    // If no cursor: start from very end of collection
    let start_key: Key = if cursor.is_empty() {
        Key::from(format!("{}:\u{10FFFF}", collection))
    } else {
        Key::from(format!("{}:{}\u{10FFFF}", collection, cursor))
    };

    // Lowest possible key for this collection
    let end_key: Key = Key::from(format!("{}:", collection));

    // Range covers full collection space
    let range: BoundRange = (end_key..=start_key).into();

    // Fetch limit + 1 so we can safely drop the cursor
    let kvs: Vec<KvPair> = self
        .tx
        .scan_reverse(range, (limit + 1) as u32)
        .await?
        .collect();

    let mut out = serde_json::Map::new();
    let mut count = 0;

    for kv in kvs {
        let k_bytes = kv.key().as_ref();
        let k = String::from_utf8_lossy(k_bytes.into()).to_string();
        let doc_id = k.strip_prefix(&prefix).unwrap_or(&k).to_string();

        // Skip the cursor itself
        if !cursor.is_empty() && doc_id == cursor {
            continue;
        }

        let v = serde_json::from_slice(&kv.value().to_vec()).unwrap_or(JsonValue::Null);
        out.insert(doc_id, v);

        count += 1;
        if count == limit {
            break;
        }
    }

    Ok(JsonValue::Object(out))
}

pub async fn batch_add(&mut self, collection: &str, items_json: &JsonValue) -> Result<(), OsunbitDBError> {
        if let JsonValue::Object(map) = items_json {
            for (id, value) in map {
                self.add(collection, id, value).await?;
            }
        } else {
            return Err(OsunbitDBError::InvalidUpdate("batch_add expects a JSON object".to_string()));
        }
        Ok(())
    }

    pub async fn batch_get(&mut self, collection: &str, ids_json: &JsonValue) -> Result<JsonValue, OsunbitDBError> {
        let mut out = serde_json::Map::new();
        if let JsonValue::Array(arr) = ids_json {
            for id_val in arr {
                if let Some(id) = id_val.as_str() {
                    if let Some(doc) = self.get(collection, id).await? {
                        out.insert(id.to_string(), doc);
                    }
                }
            }
        } else {
            return Err(OsunbitDBError::InvalidUpdate("batch_get expects a JSON array of ids".to_string()));
        }
        Ok(JsonValue::Object(out))
    }

    pub async fn batch_delete(&mut self, collection: &str, ids_json: &JsonValue) -> Result<(), OsunbitDBError> {
        if let JsonValue::Array(arr) = ids_json {
            for id_val in arr {
                if let Some(id) = id_val.as_str() {
                    self.delete(collection, id).await?;
                }
            }
        } else {
            return Err(OsunbitDBError::InvalidUpdate("batch_delete expects a JSON array of ids".to_string()));
        }
        Ok(())
    }

}
