use std::{
  collections::HashMap,
  sync::{Arc, LazyLock, RwLock},
  time::Duration,
};

use futures::{FutureExt, TryFutureExt, future::join_all};
use reqwest::Response;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::config::Config;

#[derive(Clone, Serialize)]
pub struct UserInfo {
  username: String,
  currently_playing: Option<Value>,
}

static PLAYING_TRACKS: LazyLock<RwLock<HashMap<String, UserInfo>>> =
  LazyLock::new(Default::default);

pub fn fetch_lastfm_info(username: &str) -> Option<UserInfo> {
  PLAYING_TRACKS.read().unwrap().get(username).cloned()
}

struct User {
  username: String,
  recent_tracks_url: String,
}

pub async fn run(config: &Arc<Config>) {
  let config = config.clone();
  let Some(ref last_fm_key) = config.last_fm_key else {
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

  let users = config.users.values().filter_map(|config| {
    let last_fm_username = config.last_fm_username.clone()?;

    let url = format!("https://ws.audioscrobbler.com/2.0/?method=user.getrecenttracks&format=json&user={last_fm_username}&api_key={last_fm_key}");

    Some(User {
      username: last_fm_username.clone(),
        recent_tracks_url: url,
      })
  }).collect::<Vec<_>>();

  let perform_update = async move || {
    join_all(
      users
        .iter()
        .map(|user| update_currently_listening(&user.username, &user.recent_tracks_url)),
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
}

#[derive(Deserialize)]
struct RecentTracksBase {
  #[serde(rename = "recenttracks")]
  recent_tracks: RecentTracks,
}
#[derive(Deserialize)]
struct RecentTracks {
  track: Vec<Value>,
}

pub async fn update_currently_listening(username: &str, url: &str) {
  match reqwest::get(url)
    .and_then(Response::json::<RecentTracksBase>)
    .await
  {
    Ok(mut response) => {
      let mut users = PLAYING_TRACKS.write().unwrap();
      let playing_track = response
        .recent_tracks
        .track
        .iter_mut()
        .find(|track| {
          track
            .get("@attr")
            .and_then(|attr| attr.get("nowplaying"))
            .iter()
            .eq(&["true"])
        })
        .map(Value::take);

      if let Some(user) = users.get_mut(username) {
        user.currently_playing = playing_track
      }
    }
    Err(error) => {
      tracing::error!(
        "failed to request listening status from last.fm for user {username}: {error}"
      );
    }
  }
}
