use std::sync::Arc;

use axum::{
  Json,
  extract::{Path, State},
  http::StatusCode,
  response::{IntoResponse, Response},
};
use serde::Serialize;

use crate::{
  fetchers::{discord, last_fm},
  host_config::HandlerConfig,
};

use super::MinimalUser;

#[derive(Serialize)]
pub struct UserAggregate {
  name: String,
  owners: Vec<MinimalUser>,
  discord: Option<discord::SimpleUserPresence>,
  last_fm: Option<last_fm::UserInfo>,
}

pub async fn get_user(
  State(handler_config): State<Arc<HandlerConfig>>,
  Path(path): Path<String>,
) -> Response {
  let Some(user) = handler_config.config.users.get(&path) else {
    return StatusCode::NOT_FOUND.into_response();
  };

  Json(UserAggregate {
    name: user.name.clone(),
    owners: user
      .owner_usernames
      .iter()
      .filter_map(|username| MinimalUser::from_username(&handler_config.config, &username))
      .collect(),
    discord: user.discord_id.and_then(discord::fetch_user_presence),
    last_fm: user
      .last_fm_username
      .as_ref()
      .map(String::as_str)
      .and_then(last_fm::fetch_lastfm_info),
  })
  .into_response()
}
