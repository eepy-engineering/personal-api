use std::sync::{Arc, OnceLock};

use axum::{Json, extract::State};
use serde_json::Value;

use crate::{config::Config, host_config::HandlerConfig};

use super::MinimalUser;

fn create_users_response(config: &Config) -> Value {
  let users = config
    .users
    .iter()
    .map(|(username, config)| MinimalUser {
      username: username.clone(),
      name: config.name.clone(),
    })
    .collect::<Vec<_>>();

  serde_json::to_value(&users).unwrap()
}

pub async fn get_users(State(handler_config): State<Arc<HandlerConfig>>) -> Json<&'static Value> {
  static USERS_RESPONSE: OnceLock<Value> = OnceLock::new();

  axum::Json(USERS_RESPONSE.get_or_init(|| create_users_response(&handler_config.config)))
}
