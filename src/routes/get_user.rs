use std::sync::Arc;

use axum::{
  Json,
  extract::{Path, State},
  http::StatusCode,
  response::{IntoResponse, Response},
};
use serde::Serialize;

use crate::{
  fetchers::{discord, last_fm, steam},
  host_config::HandlerConfig,
};

use super::MinimalUser;

#[derive(Serialize)]
pub struct UserAggregate<'a> {
  name: &'a String,
  owners: Vec<MinimalUser>,
  aliases: &'a Vec<String>,
  pronouns: &'a Vec<String>,
  discord: Option<discord::SimpleUserPresence>,
  last_fm: Option<last_fm::UserInfo>,
  steam: Option<steam::SteamUserInfo>,
}

pub async fn get_user(
  State(handler_config): State<Arc<HandlerConfig>>,
  Path(path): Path<String>,
) -> Response {
  let Some(user) = handler_config.config.users.get(&path) else {
    return StatusCode::NOT_FOUND.into_response();
  };

  Json(UserAggregate {
    name: &user.name,
    owners: user
      .owner_usernames
      .iter()
      .filter_map(|username| MinimalUser::from_username(&handler_config.config, &username))
      .collect(),
    aliases: &user.aliases,
    pronouns: &user.pronouns,
    discord: user.discord_id.and_then(discord::fetch_user_presence),
    last_fm: user
      .last_fm_username
      .as_ref()
      .map(String::as_str)
      .and_then(last_fm::fetch_lastfm_info),
    steam: user.steam_id.and_then(steam::get_user_info),
  })
  .into_response()
}
