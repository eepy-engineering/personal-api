use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use steam_rs::steam_id::SteamId;

#[derive(Serialize, Deserialize)]
pub struct Config {
  pub discord_bot_token: Option<String>,
  #[serde(default)]
  pub discord_initial_search_guilds: Vec<u64>,
  pub last_fm_key: Option<String>,
  pub steam_api_key: Option<String>,

  pub users: HashMap<String, UserConfig>,
}

#[derive(Serialize, Deserialize)]
pub struct UserConfig {
  pub name: String,
  #[serde(default)]
  pub owner_usernames: Vec<String>,
  pub aliases: Vec<String>,
  pub pronouns: Vec<String>,
  pub time_zone: String,
  pub domain: Option<String>,

  pub discord_id: Option<u64>,
  pub last_fm_username: Option<String>,
  pub steam_id: Option<SteamId>,
}
