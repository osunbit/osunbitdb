OsunbitDB

OsunbitDB is a lightweight, asynchronous key-value database for Rust. It supports collection-based data storage, JSON-friendly operations, and batch transactions.

Features

Async-first design using tokio

JSON-friendly, strongly typed with serde and bincode

Collections for organizing keys

Add, get, delete, scan, and update operations

Transaction support for multiple operations in one atomic block

Firestore-style simple API (db.add, db.get, etc.)

Installation

Add this to your Cargo.toml:

[dependencies]
osunbitdb = "0.1.0"
tokio = { version = "1.0", features = ["full"] }
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"

Quick Start
use osunbitdb::{OsunbitDB, OsunbitDBError};
use serde::{Serialize, Deserialize};
use osunbitdb::json; // for Firestore-style JSON objects

#[derive(Serialize, Deserialize, Debug)]
struct User {
    id: String,
    name: String,
    age: u32,
}

#[tokio::main]
async fn main() -> Result<(), OsunbitDBError> {
    // Connect to the DB
    let db = OsunbitDB::new(&["http://127.0.0.1:2379"]).await?;


    // Add a user
    let user = json!({"id": "u1", "name": "Alice", "age": 25});
    db.add("users", "u1", &user).await?;

    // Get a user
    let fetched = db.get("users", "u1").await?.unwrap();
    println!("Fetched user: {:?}", fetched);

    // Update a field
    db.update("users", "u1", "age", &json!(30)).await?;
    let updated = db.get("users", "u1").await?.unwrap();
    println!("Updated user: {:?}", updated);

    // Delete a user
    db.delete("users", "u1").await?;
    let deleted = db.get("users", "u1").await?;
    println!("Deleted? {:?}", deleted.is_none());

    Ok(())
}

Using Transactions

Transactions allow multiple operations to be executed atomically:

use osunbitdb::{OsunbitDB, json};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let db = OsunbitDB::new(&["http://127.0.0.1:2379"]).await?;

    // Start a transaction
    let mut tx = db.transaction().await?;

    // Add multiple fields
    let user = json!({"id": "u2", "name": "Bob", "age": 28});
    tx.add("users", "u2", &user).await?;
    
    // Commit transaction to persist
    tx.commit().await?;

    // Read inside another transaction
    let mut tx_read = db.transaction().await?;
    let fetched = tx_read.get("users", "u2").await?.unwrap();
    println!("Fetched in transaction: {:?}", fetched);
    tx_read.rollback().await?; // rollback if only reading

    // Update a field in transaction
    let mut tx_update = db.transaction().await?;
    tx_update.update("users", "u2", "age", &json!(29)).await?;
    tx_update.commit().await?;

    // Delete user
    let mut tx_delete = db.transaction().await?;
    tx_delete.delete("users", "u2").await?;
    tx_delete.commit().await?;

    Ok(())
}

Scanning Keys

You can scan a collection for keys with a prefix:

let users = db.collection("users");
let scanned = users.scan("u", 10).await?;
for (key, value) in scanned {
    println!("Key: {}, Value: {:?}", key, value);
}

Notes

Collections: Logical namespaces for keys (e.g., "users", "posts")

Firestore-style API: db.add, db.get, db.update, db.delete are all backed by transactions internally.

Transactions: Use db.transaction() when you need multiple operations to be atomic.

License

MIT OR Apache-2.0