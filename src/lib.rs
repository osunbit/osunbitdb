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
}

// =================== Transaction Handle ===================

pub struct TransactionHandle {
    pub(crate) tx: Transaction,
}

impl TransactionHandle {
    fn key(collection: &str, id: &str) -> Key {
        Key::from(format!("{}:{}", collection, id))
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

    pub async fn update(&mut self, collection: &str, id: &str, field: &str, value: &Json) -> Result<(), OsunbitDBError> {
        let mut data = self.get(collection, id).await?.unwrap_or(Json::Object(Map::new()));

        if let Json::Object(ref mut obj) = data {
            obj.insert(field.to_string(), value.clone());
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
    pub async fn scan(&mut self, collection: &str, limit: u32) -> Result<Vec<(String, JsonValue)>, OsunbitDBError> {
        let start_key = Self::key(collection, "");
        let range: BoundRange = (start_key.clone()..).into();
        let kvs: Vec<KvPair> = self.tx.scan(range, limit).await?.collect();
        let mut out = vec![];
        for kv in kvs {
            let k = String::from_utf8_lossy(kv.key().as_ref().into()).to_string();
            let v = serde_json::from_slice(&kv.value().to_vec()).unwrap_or(JsonValue::Null);
            out.push((k, v));
        }
        Ok(out)
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

    pub async fn update(&self, collection: &str, id: &str, field: &str, value: &JsonValue) -> Result<(), OsunbitDBError> {
        let mut tx = self.transaction().await?;
        let mut data = tx.get(collection, id).await?.unwrap_or(JsonValue::Object(Map::new()));

        if let JsonValue::Object(ref mut obj) = data {
            obj.insert(field.to_string(), value.clone());
        }

        tx.add(collection, id, &data).await?;
        tx.commit().await?;
        Ok(())
    }

    pub async fn scan(&self, collection: &str, limit: u32) -> Result<Vec<(String, JsonValue)>, OsunbitDBError> {
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
