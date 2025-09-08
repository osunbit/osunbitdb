use tikv_client::TransactionClient;
use serde_json::Value as JsonValue;
use crate::errors::OsunbitDBError;
use crate::transaction::TransactionHandle;

#[derive(Clone)]
pub struct OsunbitDB {
    client: TransactionClient, 
}

impl OsunbitDB {
    pub async fn new<S: Into<String> + Clone>(pds: &[S]) -> Result<Self, OsunbitDBError> {
        let client = TransactionClient::new(pds.to_vec()).await?;
        Ok(Self { client })
    }

    pub async fn transaction(&self) -> Result<TransactionHandle, OsunbitDBError> {
        let tx = self.client.begin_optimistic().await?;
        Ok(TransactionHandle { tx })
    }

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

    pub async fn update(&self, collection: &str, id: &str, fields: &JsonValue) -> Result<(), OsunbitDBError> {
        let mut tx = self.transaction().await?;
        tx.update(collection, id, fields).await?;
        tx.commit().await?;
        Ok(())
    }

    pub async fn scan(&self, collection: &str, limit: u32) -> Result<JsonValue, OsunbitDBError> {
        let mut tx = self.transaction().await?;
        let result = tx.scan(collection, limit).await?;
        tx.rollback().await?;
        Ok(result)
    }
}
