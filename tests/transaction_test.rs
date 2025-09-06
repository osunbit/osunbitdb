use osunbitdb::OsunbitDB;
use osunbitdb::json;

#[tokio::test]
async fn transaction_test() {
    // Connect to TiKV
    let db = OsunbitDB::new(&["http://10.88.0.4:2379"]).await.unwrap();

    // --------------------------
    // Add a user (Transaction)
    // --------------------------
    let mut tx = db.transaction().await.unwrap();
    let user = json!({
        "id": "u1",
        "name": "Alice",
        "age": 25,
        "tags": ["admin", "tester"]
    });
    tx.add("users", "u1", &user).await.unwrap();
    tx.commit().await.unwrap(); // commit to persist

    // --------------------------
    // Read the user (Transaction)
    // --------------------------
    let mut tx_read = db.transaction().await.unwrap();
    let fetched = tx_read.get("users", "u1").await.unwrap().unwrap();
    assert_eq!(fetched, user);
    tx_read.rollback().await.unwrap(); // rollback since we only read

    // --------------------------
    // Update a field
    // --------------------------
    let mut tx_update = db.transaction().await.unwrap();
    tx_update.update("users", "u1", "age", &json!(26)).await.unwrap();
    tx_update.commit().await.unwrap();

    let mut tx_check = db.transaction().await.unwrap();
    let updated = tx_check.get("users", "u1").await.unwrap().unwrap();
    assert_eq!(updated["age"], 26);
    tx_check.rollback().await.unwrap();

    // --------------------------
    // Delete the user
    // --------------------------
    let mut tx_delete = db.transaction().await.unwrap();
    tx_delete.delete("users", "u1").await.unwrap();
    tx_delete.commit().await.unwrap();

    let mut tx_verify = db.transaction().await.unwrap();
    let deleted = tx_verify.get("users", "u1").await.unwrap();
    assert!(deleted.is_none());
    tx_verify.rollback().await.unwrap();
}
