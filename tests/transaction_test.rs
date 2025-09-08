use osunbitdb::{OsunbitDB, json, increment, remove};

#[tokio::test]
async fn transaction_atomic_ops_test() {
    let db = OsunbitDB::new(&["http://10.88.0.3:2379"]).await.unwrap();

    // --------------------------
    // Seed user
    // --------------------------
    db.add("users", "u1", &json!({
        "id": "u1",
        "name": "Alice",
        "balance": 100,
        "role": "admin"
    })).await.unwrap();

    // --------------------------
    // Transaction with increment & remove
    // --------------------------
    let mut tx = db.transaction().await.unwrap();

    // Increment balance by +50
    tx.update("users", "u1", &json!({
        "balance": increment(50)
    })).await.unwrap();

    // Decrement balance by -20
    tx.update("users", "u1", &json!({
        "balance": increment(-20)
    })).await.unwrap();

    // Remove role field
    tx.update("users", "u1", &json!({
        "role": remove()
    })).await.unwrap();

    // Add a notification atomically
    tx.add("notifications:u1", "n1", &json!({
        "msg": "Your balance was updated"
    })).await.unwrap();

    // Commit all at once
    tx.commit().await.unwrap();

    // --------------------------
    // Verify committed changes
    // --------------------------
    let final_user = db.get("users", "u1").await.unwrap().unwrap();
    assert_eq!(final_user["balance"], 130); // 100 + 50 - 20
    assert!(final_user.get("role").is_none());

    let notif = db.get("notifications:u1", "n1").await.unwrap().unwrap();
    assert!(notif["msg"].as_str().unwrap().contains("updated"));

    // --------------------------
    // Rollback scenario
    // --------------------------
    let mut tx_rollback = db.transaction().await.unwrap();

    // Increment again but rollback later
    tx_rollback.update("users", "u1", &json!({
        "balance": increment(9999)
    })).await.unwrap();

    tx_rollback.rollback().await.unwrap();

    let after_rollback = db.get("users", "u1").await.unwrap().unwrap();
    assert_eq!(after_rollback["balance"], 130); // unchanged

    // --------------------------
    // Cleanup
    // --------------------------
    db.delete("users", "u1").await.unwrap();
    db.delete("notifications:u1", "n1").await.unwrap();
}
