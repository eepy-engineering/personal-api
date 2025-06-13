use std::{borrow::Cow, collections::HashMap};

use axum_extra::{headers::{authorization::Bearer, Authorization}, TypedHeader};
use serde::{Deserialize, Serialize};
use steam_rs::steam_id::SteamId;

#[derive(Serialize, Deserialize)]
pub struct Config {
  pub discord_bot_token: Option<String>,
  pub discord_initial_search_guilds: Vec<u64>,
  pub last_fm_key: Option<String>,
  pub steam_api_key: Option<String>,
  pub bluebubbles_server: Option<String>,
  pub bluebubbles_server_password: Option<String>,

  pub auth: HashMap<String, AuthConfig>,
  pub users: HashMap<String, UserConfig>,
}

#[derive(Serialize, Deserialize)]
pub struct UserConfig {
  pub name: String,
  pub aliases: Vec<String>,
  pub pronouns: Vec<String>,
  pub time_zone: String,
  pub domain: Option<String>,

  pub discord_id: Option<u64>,
  pub last_fm_username: Option<String>,
  pub steam_id: Option<SteamId>,
  pub icloud_device_id: Option<String>,
}

#[derive(Serialize, Deserialize)]
pub struct AuthConfig {
  pub scopes: Vec<String>,
}

pub fn scopes_from_bearer(bearer: Option<TypedHeader<Authorization<Bearer>>>, config: &'static Config) -> Cow<'static, [String]> {
  bearer
    .and_then(|auth| config.auth.get(auth.0.token()))
    .map(|auth| Cow::<'static, [String]>::Borrowed(&auth.scopes))
    .unwrap_or_default()
}

pub fn has_scope(auth_scopes: &Cow<[String]>, scope: &'static str) -> bool{
  auth_scopes.iter().any(|s| scope == s)
}
