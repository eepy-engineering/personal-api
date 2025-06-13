use axum::{
  Json,
  extract::{Path, State},
  http::StatusCode,
  response::{IntoResponse, Response},
};
use serde::Serialize;
use ts_rs::TS;

use crate::{
  fetchers::{discord, last_fm, steam},
  host_config::HandlerConfig,
};

#[derive(Serialize, TS)]
#[ts(export, rename = "User")]
pub struct UserAggregate<'a> {
  name: &'a str,
  aliases: &'a Vec<String>,
  pronouns: &'a Vec<String>,
  time_zone: &'a str,
  discord: Option<discord::DiscordUserInfo>,
  last_fm: Option<last_fm::UserInfo>,
  steam: Option<steam::SteamUserInfo>,
}

pub async fn get_user(
  State(handler_config): State<&'static HandlerConfig>,
  Path(path): Path<String>,
) -> Response {
  let Some(user) = handler_config.config.users.get(&path) else {
    return StatusCode::NOT_FOUND.into_response();
  };

  Json(UserAggregate {
    name: &user.name,
    aliases: &user.aliases,
    pronouns: &user.pronouns,
    time_zone: &user.time_zone,
    discord: user.discord_id.and_then(discord::fetch_user_info),
    last_fm: user
      .last_fm_username
      .as_ref()
      .map(String::as_str)
      .and_then(last_fm::fetch_lastfm_info),
    steam: user.steam_id.and_then(steam::get_user_info),
  })
  .into_response()
}
