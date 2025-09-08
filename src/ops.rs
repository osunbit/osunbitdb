use serde_json::json;
use serde_json::Value as Json;

pub fn increment(amount: i64) -> Json {
    json!({ "__op": "inc", "amount": amount })
}

pub fn remove() -> Json {
    json!({ "__op": "remove" })
}

pub fn array_union(values: Json) -> Json {
    json!({ "__op": "array_union", "values": values })
}
