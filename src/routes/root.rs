use axum::Json;
use serde_json::{Value, json};

pub async fn root_page() -> Json<Value> {
  Json(json!({
    "hello!": "welcome to the user api",
    "here are our routes": {
      "/": "root page",
      "/users": "a summary of all the available users",
      "/users/<username>": "the information about a specific user"
    }
  }))
}
