use axum::{
  Json,
  extract::{Path, State},
  http::StatusCode,
  response::{IntoResponse, Response},
};
use axum_extra::{
  TypedHeader,
  headers::{Authorization, authorization::Bearer},
};
use serde::Serialize;
use ts_rs::TS;

use crate::{
  config::scopes_from_bearer,
  fetchers::{discord, icloud, last_fm, steam},
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
  location: Option<icloud::Location>,
}

pub async fn get_user(
  State(handler_config): State<&'static HandlerConfig>,
  Path(path): Path<String>,
  bearer: Option<TypedHeader<Authorization<Bearer>>>,
) -> Response {
  let Some(user) = handler_config.config.users.get(&path) else {
    return StatusCode::NOT_FOUND.into_response();
  };

  let auth_scopes = scopes_from_bearer(bearer, &handler_config.config);
  let mut location = user
    .icloud_device_id
    .as_ref()
    .map(String::as_str)
    .and_then(|id| icloud::get_user_info(id, auth_scopes));

  Json(UserAggregate {
    name: &user.name,
    aliases: &user.aliases,
    pronouns: &user.pronouns,
    time_zone: location
      .as_mut()
      .and_then(|location| location.time_zone.take())
      .unwrap_or(&user.time_zone),
    discord: user.discord_id.and_then(discord::fetch_user_info),
    last_fm: user
      .last_fm_username
      .as_ref()
      .map(String::as_str)
      .and_then(last_fm::fetch_lastfm_info),
    steam: user.steam_id.and_then(steam::get_user_info),
    location,
  })
  .into_response()
}
