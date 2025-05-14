use std::{
  collections::HashMap,
  sync::{LazyLock, RwLock},
  time::Duration,
};

use futures::{FutureExt, future::join_all};
use lastfm::track::NowPlayingTrack;
use serde::Serialize;

use crate::config::Config;

#[derive(Clone, Serialize)]
pub struct UserInfo {
  username: String,
  currently_playing: Option<NowPlayingTrack>,
}

static PLAYING_TRACKS: LazyLock<RwLock<HashMap<String, UserInfo>>> =
  LazyLock::new(Default::default);

pub fn fetch_lastfm_info(username: &str) -> Option<UserInfo> {
  PLAYING_TRACKS.read().unwrap().get(username).cloned()
}

struct User {
  username: String,
  client: lastfm::Client<String, String>,
}

pub async fn run(config: &'static Config) {
  let config = config;
  let Some(last_fm_key) = &config.last_fm_key else {
    return;
  };

  *PLAYING_TRACKS.write().unwrap() = config
    .users
    .values()
    .filter_map(|config| {
      let last_fm_username = config.last_fm_username.clone()?;
      Some((
        last_fm_username.to_owned(),
        UserInfo {
          username: last_fm_username,
          currently_playing: None,
        },
      ))
    })
    .collect();

  let users = config
    .users
    .values()
    .filter_map(|config| {
      let last_fm_username = config.last_fm_username.clone()?;

      let client = lastfm::Client::builder()
        .api_key(last_fm_key.clone())
        .username(last_fm_username.clone())
        .build();

      Some(User {
        username: last_fm_username,
        client,
      })
    })
    .collect::<Vec<_>>();

  let perform_update = async move || {
    join_all(
      users
        .iter()
        .map(|user| update_currently_listening(&user.username, &user.client)),
    )
    .map(drop)
    .await;
  };

  perform_update().await;

  tokio::spawn(async move {
    loop {
      tokio::time::sleep(Duration::from_secs(10)).await;
      perform_update().await
    }
  });

  tracing::info!("started last.fm fetcher");
}

pub async fn update_currently_listening(username: &str, client: &lastfm::Client<String, String>) {
  // match reqwest::get(url)
  //   .and_then(Response::json::<RecentTracksBase>)
  //   .await
  match client.now_playing().await {
    Ok(currently_playing) => {
      let mut users = PLAYING_TRACKS.write().unwrap();
      if let Some(user) = users.get_mut(username) {
        user.currently_playing = currently_playing;
      }
    }
    Err(error) => {
      tracing::error!(
        "failed to request listening status from last.fm for user {username}: {error}"
      );
    }
  }
}
