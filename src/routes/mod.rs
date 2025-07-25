use serde::Serialize;
use ts_rs::TS;

use crate::config::Config;

pub mod get_host_user;
pub mod get_user;
pub mod get_users;
pub mod root;

#[derive(Serialize, TS)]
struct MinimalUser {
  username: String,
  name: String,
}

impl MinimalUser {
  #[allow(unused)]
  pub fn from_username(config: &Config, username: &str) -> Option<Self> {
    let user = config.users.get(username)?;

    Some(Self {
      name: user.name.clone(),
      username: username.to_owned(),
    })
  }
}
