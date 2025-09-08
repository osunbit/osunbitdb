# OsunbitDB

OsunbitDB is a lightweight, async-first key-value database for Rust, built on **TiKV**.  
It supports **Firestore-style collections**, **JSON-friendly updates**, and **atomic transactions**.

---

## ✨ Features

- 📦 Built on top [tikv](https://tikv.org) super fast atomic transaction client 
- 🚀 Async-first with [tokio](https://tokio.rs)  
- 📦 JSON-native via `serde_json`  
- 📂 Collection & subcollection support (`users:u1:inbox`)  
- 🔄 Atomic transactions  
- ➕ Increment & field removal helpers  
- 🔍 Simple API (`add`, `get`, `update`, `delete`, `scan`)  

---

## 📦 Installation

Add to `Cargo.toml`:

```toml
[dependencies]
osunbitdb = "0.2.0"
```

---

## ⚡ Quick Start

```rust
   use osunbitdb::{OsunbitDB, json};

    // Connect to the tikv cluster
    let db = OsunbitDB::new(&["http://127.0.0.1:2379"]).await?;

    // ➕ Add a document
    let user = json!({"id": "u1", "name": "Alice", "age": 25});
    db.add("users", "u1", &user).await?;

    // 🔍 Get a document
    let fetched = db.get("users", "u1").await?.unwrap();
    println!("Fetched: {:?}", fetched);

    // ✏️ Update fields (partial update, other fields untouched or create if not exists)
    db.update("users", "u1", &json!({"age": 26, "active": true})).await?;

    // ❌ Delete document
    db.delete("users", "u1").await?;

```

---

## 📂 Collections & Subcollections

```rust

    // Add to a subcollection
    db.add("users:u1:inbox", "m1", &json!({
        "title": "Hello",
        "body": "First message"
    })).await?;

    // Fetch
    let msg = db.get("users:u1:inbox", "m1").await?.unwrap();
    println!("Inbox msg: {:?}", msg);

    // Nested sub-subcollection
    db.add("users:u1:inbox:group1", "g1msg", &json!({
        "title": "Group message"
    })).await?;

```

---

## 🔄 Increment & Remove Helpers

```rust
use osunbitdb::{OsunbitDB, json, increment, remove};

    let db = OsunbitDB::new(&["http://127.0.0.1:2379"]).await?;

    db.add("users", "u1", &json!({"balance": 100, "role": "admin"})).await?;

    // ➕ Increment field
    db.update("users", "u1", &json!({
        "balance": increment(25)
    })).await?;
    // balance = 125

    // ➖ Decrement field
    db.update("users", "u1", &json!({
        "balance": increment(-5)
    })).await?;
    // balance = 120

    // 🗑️ Remove a field
    db.update("users", "u1", &json!({
        "role": remove()
    })).await?;

```

---

## 🔒 Transactions (Atomic Ops)

```rust
use osunbitdb::{OsunbitDB, json, increment, remove};

    // Start a transaction
    let mut tx = db.transaction().await?;

    // Atomic balance transfer
    tx.update("users", "u1", &json!({"balance": increment(-100)})).await?;
    tx.update("users", "u2", &json!({"balance": increment(100)})).await?;

    // Add a notification
    tx.add("notifications:u2", "n1", &json!({
        "msg": "You received 100 from Alice"
    })).await?;

    // Commit all changes
    tx.commit().await?;

    // Rollback example
    let mut tx2 = db.transaction().await?;
    tx2.update("users", "u1", &json!({"balance": increment(9999)})).await?;
    tx2.rollback().await?;

```

---

## 🔍 Scanning Collections

```rust
let scanned = db.scan("users", 10).await?;
for (key, doc) in scanned {
    println!("User {} => {:?}", key, doc);
}
```

---

## 📝 Notes

- Collections are just logical namespaces (`users`, `users:u1:inbox`)  
- Subcollections can be nested infinitely using `:`  
- Updates only modify provided fields (others remain unchanged) 
- All operation are transaction   
- Transactions guarantee all-or-nothing execution  

---

## 📜 License

MIT OR Apache-2.0
