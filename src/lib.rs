// src/lib.rs
use tikv_client::{TransactionClient, Transaction, Key, Value, KvPair, BoundRange, Error as TiKVError};
use serde_json::{Value as JsonValue, Map};
use serde::{Serialize, de::DeserializeOwned};
use thiserror::Error;
use tokio::task::JoinError;
use bincode;

pub use serde_json::{json, Value as Json};

// =================== Error Handling ===================

#[derive(Debug, Error)]
pub enum OsunbitDBError {
    #[error("TiKV client error: {0}")]
    TiKV(#[from] TiKVError),

    #[error("Serialization error: {0}")]
    Bincode(#[from] Box<bincode::ErrorKind>),

    #[error("Join error: {0}")]
    Join(#[from] JoinError),

    #[error("Serde JSON error: {0}")]
    SerdeJsonError(#[from] serde_json::Error),

    #[error("Invalid update: {0}")]
    InvalidUpdate(String),
}

// =================== Transaction Handle ===================

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
    // Load current doc (or create new empty object)
    let mut data = self.get(collection, id).await?.unwrap_or(JsonValue::Object(Map::new()));

    // Only merge if target is an object
    if let JsonValue::Object(ref mut obj) = data {
        if let JsonValue::Object(new_fields) = fields {
            for (k, v) in new_fields {
                if let Some(op) = v.get("__op") {
                    match op.as_str().unwrap_or("") {
                        "inc" => {
                            let delta = v["amount"].as_i64().unwrap_or(0);
                            let current = obj.get(k).and_then(|val| val.as_i64()).unwrap_or(0);
                            obj.insert(k.clone(), json!(current + delta));
                        }
                        "remove" => {
                            obj.remove(k);
                        }
                        _ => {
                            obj.insert(k.clone(), v.clone());
                        }
                    }
                } else {
                    obj.insert(k.clone(), v.clone());
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

    /// Scan all keys with a prefix (collection)
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

        // Strip "users:" → keep only "u1"
        let doc_id = k.strip_prefix(&prefix).unwrap_or(&k).to_string();

        let v = serde_json::from_slice(&kv.value().to_vec()).unwrap_or(JsonValue::Null);
        out.insert(doc_id, v);
    }

    Ok(JsonValue::Object(out))
}

}

// =================== Main Client ===================

#[derive(Clone)]
pub struct OsunbitDB {
    client: TransactionClient, 
}

impl OsunbitDB {
    pub async fn new<S: Into<String> + Clone>(pds: &[S]) -> Result<Self, OsunbitDBError> {
        let client = TransactionClient::new(pds.to_vec()).await?;
        Ok(Self { client })
    }

    /// Start a manual transaction for multi-step atomic operations
    pub async fn transaction(&self) -> Result<TransactionHandle, OsunbitDBError> {
        let tx = self.client.begin_optimistic().await?;
        Ok(TransactionHandle { tx })
    }

    // --------------------------
    // Firestore-style convenience
    // --------------------------

    pub async fn add(&self, collection: &str, id: &str, value: &JsonValue) -> Result<(), OsunbitDBError> {
        let mut tx = self.transaction().await?;
        tx.add(collection, id, value).await?;
        tx.commit().await?;
        Ok(())
    }

    pub async fn get(&self, collection: &str, id: &str) -> Result<Option<JsonValue>, OsunbitDBError> {
        let mut tx = self.transaction().await?;
        let result = tx.get(collection, id).await?;
        tx.rollback().await?;
        Ok(result)
    }

    pub async fn delete(&self, collection: &str, id: &str) -> Result<(), OsunbitDBError> {
        let mut tx = self.transaction().await?;
        tx.delete(collection, id).await?;
        tx.commit().await?;
        Ok(())
    }

    pub async fn update(
        &self,
        collection: &str,
        id: &str,
        fields: &JsonValue,
    ) -> Result<(), OsunbitDBError> {
        let mut tx = self.transaction().await?;
        tx.update(collection, id, fields).await?;
        tx.commit().await?;
        Ok(())
    }

 pub async fn scan(
    &self,
    collection: &str,
    limit: u32,
) -> Result<JsonValue, OsunbitDBError> {
    let mut tx = self.transaction().await?;
    let result = tx.scan(collection, limit).await?;
    tx.rollback().await?;
    Ok(result)
}

}

// =================== Utils ===================

pub fn encode<T: Serialize>(value: &T) -> Result<Vec<u8>, OsunbitDBError> {
    Ok(bincode::serialize(value)?)
}

pub fn decode<T: DeserializeOwned>(bytes: &[u8]) -> Result<T, OsunbitDBError> {
    Ok(bincode::deserialize(bytes)?)
}

pub fn make_key(collection: &str, id: &str) -> Vec<u8> {
    format!("{}:{}", collection, id).into_bytes()
}

pub fn increment(amount: i64) -> Json {
    json!({ "__op": "inc", "amount": amount })
}

pub fn remove() -> Json {
    json!({ "__op": "remove" })
}

