use osunbitdb::{OsunbitDB, json, increment, remove};

#[tokio::test]
async fn raw_features_test() -> Result<(), Box<dyn std::error::Error>> {
    // Connect to TiKV
    let db = OsunbitDB::new(&["http://10.88.0.3:2379"]).await?;

    // --------------------------
    // Flat collection tests
    // --------------------------

    let user1 = json!({
        "id": "u1",
        "name": "Alice",
        "age": 25,
        "tags": ["admin", "tester"],
        "balance": 100
    });

    let user2 = json!({
        "id": "u2",
        "name": "Bob",
        "age": 30,
        "tags": ["dev"],
        "balance": 50
    });

    // Add docs
    db.add("users", "u1", &user1).await?;
    db.add("users", "u2", &user2).await?;

    // Get single doc
    let fetched1 = db.get("users", "u1").await?.unwrap();
    assert_eq!(fetched1, user1);

    // Update multiple fields
    db.update("users", "u1", &json!({
        "age": 26,
        "active": true
    })).await?;
    let updated1 = db.get("users", "u1").await?.unwrap();
    assert_eq!(updated1["age"], 26);
    assert_eq!(updated1["active"], true);
    assert_eq!(updated1["name"], "Alice"); // unchanged

    // ✅ Increment field test
    db.update("users", "u1", &json!({
        "balance": increment(25)
    })).await?;
    let after_inc = db.get("users", "u1").await?.unwrap();
    assert_eq!(after_inc["balance"], 125); // 100 + 25

    db.update("users", "u1", &json!({
        "balance": increment(-5)
    })).await?;
    let after_dec = db.get("users", "u1").await?.unwrap();
    assert_eq!(after_dec["balance"], 120); // 125 - 5

    // ✅ Remove field test
    db.update("users", "u1", &json!({
        "active": remove()
    })).await?;
    let after_remove = db.get("users", "u1").await?.unwrap();
    assert!(after_remove.get("active").is_none());

    // Scan collection
    let scanned = db.scan("users", 10).await?;
    assert!(scanned.get("u1").is_some());
    assert!(scanned.get("u2").is_some());
    assert_eq!(scanned["u1"]["name"], "Alice");

    // Delete one doc
    db.delete("users", "u2").await?;
    let deleted2 = db.get("users", "u2").await?;
    assert!(deleted2.is_none());

    // Auto-create on update
    db.update("users", "u3", &json!({
        "name": "Charlie",
        "age": 22
    })).await?;
    let created = db.get("users", "u3").await?.unwrap();
    assert_eq!(created["name"], "Charlie");
    assert_eq!(created["age"], 22);

    // --------------------------
    // Subcollection tests
    // --------------------------

    // Add to subcollection: inbox under u1
    db.add("users:u1:inbox", "m1", &json!({
        "title": "Hello",
        "body": "First message"
    })).await?;
    db.add("users:u1:inbox", "m2", &json!({
        "title": "Hi again",
        "body": "Second message"
    })).await?;

    // Fetch one inbox message
    let m1 = db.get("users:u1:inbox", "m1").await?.unwrap();
    assert_eq!(m1["title"], "Hello");

    // Scan subcollection
    let inbox = db.scan("users:u1:inbox", 10).await?;
    assert!(inbox.get("m1").is_some());
    assert!(inbox.get("m2").is_some());

    // Update inside subcollection
    db.update("users:u1:inbox", "m1", &json!({
        "read": true
    })).await?;
    let m1_updated = db.get("users:u1:inbox", "m1").await?.unwrap();
    assert_eq!(m1_updated["read"], true);
    assert_eq!(m1_updated["title"], "Hello"); // unchanged

    // --------------------------
    // Sub-subcollection tests
    // --------------------------

    db.add("users:u1:inbox:group1", "g1msg", &json!({
        "title": "Group msg",
        "body": "Nested level test"
    })).await?;

    let g1msg = db.get("users:u1:inbox:group1", "g1msg").await?.unwrap();
    assert_eq!(g1msg["title"], "Group msg");

    let group1 = db.scan("users:u1:inbox:group1", 10).await?;
    assert!(group1.get("g1msg").is_some());

    // --------------------------
    // Cleanup
    // --------------------------
    db.delete("users", "u1").await?;
    db.delete("users", "u3").await?;
    db.delete("users:u1:inbox", "m1").await?;
    db.delete("users:u1:inbox", "m2").await?;
    db.delete("users:u1:inbox:group1", "g1msg").await?;

    Ok(())
}
