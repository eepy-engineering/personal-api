use std::{
  collections::HashMap,
  sync::{LazyLock, RwLock},
  time::Duration,
};

use futures::TryFutureExt;
use serde::Serialize;
use serenity::json::Value;
use steam_rs::{Steam, steam_id::SteamId};
use ts_rs::TS;

use crate::config::Config;

#[derive(Clone, Serialize, TS)]
pub struct SteamUserInfo {
  steam_id: String,
  persona_name: String,
  game: Option<SteamGameInfo>,
}

#[derive(Clone, Serialize, TS)]
pub struct SteamGameInfo {
  appid: u64,
  name: String,
  info_url: String,
}

static USER_INFO: LazyLock<RwLock<HashMap<u64, SteamUserInfo>>> = LazyLock::new(Default::default);

pub fn get_user_info(steam_id: SteamId) -> Option<SteamUserInfo> {
  USER_INFO.read().unwrap().get(&steam_id.into_u64()).cloned()
}

static GAME_NAMES: LazyLock<RwLock<HashMap<u64, String>>> = LazyLock::new(|| {
  RwLock::new(
    convert_game_list(serde_json::from_str(include_str!("./initial_steam_games.json")).unwrap())
      .expect("failed to parse steam games"),
  )
});

fn convert_game_list(value: Value) -> Option<HashMap<u64, String>> {
  let games = value.get("applist")?.get("apps")?.as_array()?;

  Some(
    games
      .into_iter()
      .filter_map(|game_entry| {
        let id = game_entry.get("appid")?.as_u64()?;
        let name = game_entry.get("name")?.as_str()?;

        Some((id, name.to_owned()))
      })
      .collect(),
  )
}

async fn fetch_games() {
  let Ok(value) = reqwest::get("https://api.steampowered.com/ISteamApps/GetAppList/v2")
    .and_then(|response| response.json::<Value>())
    .await
  else {
    return;
  };

  if let Some(list) = convert_game_list(value) {
    *GAME_NAMES.write().unwrap() = list;
  }
}

pub async fn run(config: &Config) {
  let Some(steam_api_key) = &config.steam_api_key else {
    return;
  };

  let players = config
    .users
    .values()
    .filter_map(|config| config.steam_id)
    .collect::<Vec<_>>();

  let steam = Steam::new(&steam_api_key);

  async fn perform_update(steam: &Steam, players: &Vec<SteamId>) {
    match steam.get_player_summaries(players.clone()).await {
      Ok(players) => {
        let mut user_info = USER_INFO.write().unwrap();
        let game_names = GAME_NAMES.read().unwrap();
        for player in players {
          let game_id = player
            .game_id
            .map(|id| u64::from_str_radix(&id, 10).unwrap());

          let game_name = game_id.map(|id| SteamGameInfo {
            appid: id,
            name: game_names
              .get(&id)
              .cloned()
              .unwrap_or_else(|| "unknown game".to_owned()),
            info_url: format!(
              "http://store.steampowered.com/api/appdetails?appids={id}&filters=basic"
            ),
          });

          user_info.insert(
            player.steam_id.into_u64(),
            SteamUserInfo {
              steam_id: player.steam_id.into_u64().to_string(),
              persona_name: player.persona_name,
              game: game_name,
            },
          );
        }
      }
      Err(error) => {
        tracing::error!("failed to fetch player summaries: {error:?}");
      }
    };
  }

  tokio::spawn(async move {
    let mut interval = tokio::time::interval(Duration::from_secs(21600));

    // preventing an easy rate limit in case we repeatedly restart
    interval.tick().await;

    loop {
      interval.tick().await;
      fetch_games().await;
    }
  });

  tokio::spawn(async move {
    let mut interval = tokio::time::interval(Duration::from_secs(30));

    loop {
      interval.tick().await;
      perform_update(&steam, &players).await;
    }
  });

  tracing::info!("started steam fetcher");
}
