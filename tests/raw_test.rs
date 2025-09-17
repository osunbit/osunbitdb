use osunbitdb::{OsunbitDB, json};

#[tokio::test]
async fn batch_operations_test() -> Result<(), Box<dyn std::error::Error>> {
    let db = OsunbitDB::new(&["http://10.88.0.3:2379"]).await?;

    // --------------------------
    // 1️⃣ Batch Add
    // --------------------------
    let batch_docs = json!({
        "tx1": { "amount": 100, "type": "send", "status": "success" },
        "tx2": { "amount": 200, "type": "receive", "status": "success" },
        "tx3": { "amount": 50,  "type": "withdraw", "status": "pending" }
    });

    db.batch_add("transactions:u1", &batch_docs).await?;

    // --------------------------
    // 2️⃣ Batch Get
    // --------------------------
    let ids_json = json!(["tx1", "tx2", "tx3"]);
    let fetched = db.batch_get("transactions:u1", &ids_json).await?;

    println!("Fetched batch: {:?}", fetched);
    let fetched_obj = fetched.as_object().unwrap();
    assert_eq!(fetched_obj.len(), 3);
    assert_eq!(fetched_obj["tx1"]["amount"], 100);
    assert_eq!(fetched_obj["tx2"]["type"], "receive");
    assert_eq!(fetched_obj["tx3"]["status"], "pending");

    // --------------------------
    // 3️⃣ Batch Delete
    // --------------------------
    let ids_to_delete = json!(["tx1", "tx2", "tx3"]);
    db.batch_delete("transactions:u1", &ids_to_delete).await?;

    // Confirm deletion
    let after_delete = db.batch_get("transactions:u1", &ids_to_delete).await?;
    assert!(after_delete.as_object().unwrap().is_empty());

    Ok(())
}
