use axum::{
  debug_handler,
  extract::{Path, State},
  response::Response,
};
use axum_extra::{extract::Host, headers::{authorization::Bearer, Authorization}, TypedHeader};

use crate::host_config::HandlerConfig;

use super::get_user::get_user;

#[debug_handler]
pub async fn get_host_user(
  State(host_config): State<&'static HandlerConfig>,
  Host(host): Host,
  auth: Option<TypedHeader<Authorization<Bearer>>>,
) -> Response {
  get_user(
    State(host_config),
    Path(
      host_config
        .domains
        .get(&host)
        .map(|domain| &domain.username)
        .cloned()
        .unwrap_or_default(),
    ),
    auth
  )
  .await
}
