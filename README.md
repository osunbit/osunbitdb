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

## 🔄 Increment & Remove & Array Union & Array Remove & ExpiryAt Helpers

```rust
use osunbitdb::{OsunbitDB, json, increment, remove, array_union, array_remove};

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

   // 🗑️ Remove a field (top-level)
    db.update("users", "u1", &json!({
        "role": remove()
    })).await?;

    // ➕ Increment nested field
    db.update("users", "u1", &json!({
        "profile.points": increment(5)
    })).await?;
    // profile.points = 15

    // 🗑️ Remove nested field
    db.update("users", "u1", &json!({
        "profile.badges": remove()
    })).await?;

    // 🔗 Array Union (top-level)
    db.update("users", "u1", &json!({
        "tags": array_union(json!(["rust", "db"]))
    })).await?;

    // ✅ Array array_remove (top-level)
    db.update("users", "u1", &json!({
        "tags": array_remove(json!(["rust"]))
    })).await?;

     // ✅ ExpiryAt test
    let exp_doc = json!({
        "id": "exp1",
        "name": "WillExpire",
        "expiryAt": "02-10-2015"
    });

    db.update("sessions", "exp1", &exp_doc).await?;


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
// scan first 10
let scanned = db.scan("users", 10, "").await?;

// scan 10 from id
let scanned_from_id = db.scan("users", 10, "id").await?;

let batch_docs = json!({
    "tx1": {"amount": 100, "type": "send"},
    "tx2": {"amount": 200, "type": "receive"}
});

db.batch_add("transactions:u123", &batch_docs).await?;

let ids_json = json!(["tx1", "tx2"]);
let docs = db.batch_get("transactions:u123", &ids_json).await?;
 

 let ids_to_delete = json!(["tx1", "tx2"]);
db.batch_delete("transactions:u123", &ids_to_delete).await?;


 
```

---

## 📝 Notes

- Collections are just logical namespaces (`users`, `users:u1:inbox`)  
- Subcollections can be nested infinitely using `:`  
- Updates only modify provided fields (others remain unchanged) 
- All operation are transaction   
- Transactions guarantee all-or-nothing execution  
- All helpers support dot notation for nested fields
- increment() works with positive or negative numbers.
- remove() deletes the field entirely.
- array_union() merges arrays without duplicates.

---

## 📜 License

MIT OR Apache-2.0
