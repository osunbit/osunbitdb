use rocksdb::{DB, Options, WriteBatch, Error as RocksError};
use serde::{Serialize, de::DeserializeOwned};
use bincode;
use tokio::task;
use std::sync::Arc;
use tokio::task::JoinError;

/// Custom error wrapper
#[derive(Debug)]
pub enum OsunbitDBError {
    RocksDB(RocksError),
    Bincode(Box<bincode::ErrorKind>),
    Join(JoinError),
}

impl From<RocksError> for OsunbitDBError {
    fn from(err: RocksError) -> Self {
        OsunbitDBError::RocksDB(err)
    }
}

impl From<Box<bincode::ErrorKind>> for OsunbitDBError {
    fn from(err: Box<bincode::ErrorKind>) -> Self {
        OsunbitDBError::Bincode(err)
    }
}

impl From<JoinError> for OsunbitDBError {
    fn from(err: JoinError) -> Self {
        OsunbitDBError::Join(err)
    }
}

/// Main DB wrapper
#[derive(Clone)]
pub struct OsunbitDB {
    db: Arc<DB>,
}

impl OsunbitDB {
    pub fn new(path: &str) -> Result<Self, OsunbitDBError> {
        let mut opts = Options::default();
        opts.create_if_missing(true);
        let db = DB::open(&opts, path)?;
        Ok(Self { db: Arc::new(db) })
    }

    pub fn collection(&self, name: &str) -> Collection {
        Collection {
            name: name.to_string(),
            db: self.db.clone(),
        }
    }
}

/// Represents a “table”/collection in DB
#[derive(Clone)]
pub struct Collection {
    name: String,
    db: Arc<DB>,
}

impl Collection {
    fn key(&self, id: &str) -> String {
        format!("{}:{}", self.name, id)
    }

    pub async fn add<T>(&self, id: &str, value: &T) -> Result<(), OsunbitDBError>
    where
        T: Serialize + Send + Sync + 'static,
    {
        let encoded = bincode::serialize(value)?;
        let key = self.key(id);
        let db = self.db.clone();

        task::spawn_blocking(move || db.put(key, encoded))
            .await? // Convert JoinError
            .map_err(OsunbitDBError::from)?; // Convert RocksDB error
        Ok(())
    }

    pub async fn get<T>(&self, id: &str) -> Result<Option<T>, OsunbitDBError>
    where
        T: DeserializeOwned + Send + Sync + 'static,
    {
        let key = self.key(id);
        let db = self.db.clone();

        let result = task::spawn_blocking(move || db.get(key))
            .await??; // First ? converts JoinError, second ? converts RocksDB error

        if let Some(bytes) = result {
            Ok(Some(bincode::deserialize(&bytes)?))
        } else {
            Ok(None)
        }
    }

    pub async fn delete(&self, id: &str) -> Result<(), OsunbitDBError> {
        let key = self.key(id);
        let db = self.db.clone();
        task::spawn_blocking(move || db.delete(key)).await??;
        Ok(())
    }

    pub async fn scan<T>(&self) -> Result<Vec<T>, OsunbitDBError>
    where
        T: DeserializeOwned + Send + Sync + 'static,
    {
        let prefix = format!("{}:", self.name);
        let db = self.db.clone();

        let items = task::spawn_blocking(move || -> Result<Vec<T>, OsunbitDBError> {
            let mut result = vec![];
            let iter = db.iterator(rocksdb::IteratorMode::Start);

            for item in iter {
                let (k, v) = item.map_err(OsunbitDBError::RocksDB)?;
                if k.starts_with(prefix.as_bytes()) {
                    result.push(bincode::deserialize(&v)?);
                }
            }
            Ok(result)
        })
        .await??;

        Ok(items)
    }

    pub async fn update<T, F>(&self, id: &str, mut f: F) -> Result<(), OsunbitDBError>
    where
        T: Serialize + DeserializeOwned + Send + Sync + 'static,
        F: FnMut(&mut T),
    {
        if let Some(mut obj) = self.get::<T>(id).await? {
            f(&mut obj);
            self.add(id, &obj).await?;
        }
        Ok(())
    }

    pub async fn transaction<F>(&self, f: F) -> Result<(), OsunbitDBError>
    where
        F: FnOnce(&mut WriteBatch) + Send + 'static,
    {
        let db = self.db.clone();
        task::spawn_blocking(move || -> Result<(), OsunbitDBError> {
            let mut batch = WriteBatch::default();
            f(&mut batch);
            db.write(batch)?;
            Ok(())
        })
        .await??;

        Ok(())
    }
}
