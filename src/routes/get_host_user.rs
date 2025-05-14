use std::sync::Arc;

use axum::{
  debug_handler,
  extract::{Path, State},
  response::Response,
};
use axum_extra::extract::Host;

use crate::host_config::HandlerConfig;

use super::get_user::get_user;

#[debug_handler]
pub async fn get_host_user(
  State(host_config): State<Arc<HandlerConfig>>,
  Host(host): Host,
) -> Response {
  get_user(
    State(host_config.clone()),
    Path(
      host_config
        .domains
        .get(&host)
        .map(|domain| &domain.username)
        .cloned()
        .unwrap_or_default(),
    ),
  )
  .await
}
