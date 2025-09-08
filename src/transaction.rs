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

        self.add(collection, id, &data).await?;
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
    ) -> Result<JsonValue, OsunbitDBError> {
        let start_key = Self::key(collection, "");
        let range: BoundRange = (start_key.clone()..).into();
        let kvs: Vec<KvPair> = self.tx.scan(range, limit).await?.collect();

        let mut out = serde_json::Map::new();
        let prefix = format!("{}:", collection);

        for kv in kvs {
            let k = String::from_utf8_lossy(kv.key().as_ref().into()).to_string();
            let doc_id = k.strip_prefix(&prefix).unwrap_or(&k).to_string();
            let v = serde_json::from_slice(&kv.value().to_vec()).unwrap_or(JsonValue::Null);
            out.insert(doc_id, v);
        }

        Ok(JsonValue::Object(out))
    }
}
