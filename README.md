# OsunbitDB

OsunbitDB is a lightweight, asynchronous DB. It provides a simple key-value interface with collections, add/get/delete operations, scanning, updates, and batch transactions.

## Features

- Async-friendly using `tokio::task::spawn_blocking`
- Strongly typed serialization via `serde` and `bincode`
- Collection-based design for organizing keys
- Add, get, delete, scan, and update operations
- Batch transactions for multiple operations

## Installation

Add this to your `Cargo.toml`:

```toml
[dependencies]
osunbitdb = "0.1.0"
Then run:

bash
Copy code
cargo build
Usage
rust
Copy code
use osunbitdb::{OsunbitDB, OsunbitDBError};
use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize, Debug)]
struct User {
    id: String,
    name: String,
}

#[tokio::main]
async fn main() -> Result<(), OsunbitDBError> {
    // Open or create the database
    let db = OsunbitDB::new("mydb")?;
    
    // Get a collection
    let users = db.collection("users");

    // Add a new user
    let user = User { id: "1".to_string(), name: "Alice".to_string() };
    users.add(&user.id, &user).await?;

    // Get a user
    if let Some(u) = users.get::<User>(&user.id).await? {
        println!("Found user: {:?}", u);
    }

    // Update a user
    users.update::<User, _>(&user.id, |u| u.name = "Bob".to_string()).await?;

    // Scan all users
    let all_users = users.scan::<User>().await?;
    println!("All users: {:?}", all_users);

    // Delete a user
    users.delete(&user.id).await?;

    Ok(())
}
License
MIT OR Apache-2.0