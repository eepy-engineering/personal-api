use axum::Json;
use serde_json::{Value, json};

pub async fn root_page() -> Json<Value> {
  Json(json!({
    "hello!": "welcome to the user api",
    "here are our routes": {
      "/": "root page",
      "/users": "a summary of all the available users",
      "/user": "the information about a specific user, if the site is being accessed from a user's domain",
      "/user/<username>": "the information about a specific user"
    }
  }))
}
