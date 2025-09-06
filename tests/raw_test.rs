use osunbitdb::OsunbitDB;
use osunbitdb::json;

#[tokio::test]
async fn firestore_style_test() -> Result<(), Box<dyn std::error::Error>> {
    // Connect to TiKV
    let db = OsunbitDB::new(&["http://10.88.0.4:2379"]).await?;

    // Define a JSON object
    let user = json!({
        "id": "u1",
        "name": "Alice",
        "age": 25,
        "tags": ["admin", "tester"]
    });

    // --------------------------
    // Add a user
    // --------------------------
    db.add("users", "u1", &user).await?;

    // --------------------------
    // Get the user
    // --------------------------
    let fetched = db.get("users", "u1").await?.unwrap();
    assert_eq!(fetched, user);

    // --------------------------
    // Update a field
    // --------------------------
    db.update("users", "u1", "age", &json!(30)).await?;
    let updated = db.get("users", "u1").await?.unwrap();
    assert_eq!(updated["age"], 30);

    // --------------------------
    // Delete the user
    // --------------------------
    db.delete("users", "u1").await?;
    let deleted = db.get("users", "u1").await?;
    assert!(deleted.is_none());

    Ok(())
}
