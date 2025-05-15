use std::{
  collections::HashMap,
  sync::{LazyLock, RwLock},
  time::Duration,
};

use chrono::{DateTime, SubsecRound, Utc};
use futures::{FutureExt, TryFutureExt, future::join_all};
use lastfm::{artist::Artist, imageset::ImageSet, track::Track};
use reqwest::Response;
use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::config::Config;

#[allow(unused)]
#[derive(Clone, Serialize, TS, PartialEq, Eq)]
#[ts(rename = "LastFmImageSet")]
pub struct TypescriptImageSet {
  pub small: Option<String>,
  pub medium: Option<String>,
  pub large: Option<String>,
  pub extralarge: Option<String>,
}

#[allow(unused)]
#[derive(Clone, Serialize, TS, PartialEq, Eq)]
#[ts(rename = "LastFmArtist")]
pub struct TypescriptArtist {
  #[ts(as = "TypescriptImageSet")]
  pub image: ImageSet,
  pub name: String,
  pub url: String,
}
#[allow(unused)]
#[derive(Clone, Serialize, TS, PartialEq, Eq)]
#[ts(rename = "LastFmTrack")]
pub struct TypescriptTrack {
  #[ts(as = "TypescriptArtist")]
  pub artist: Artist,
  pub name: String,
  #[ts(as = "TypescriptImageSet")]
  pub image: ImageSet,
  pub album: String,
  pub url: String,
  pub start_time: DateTime<Utc>,
}

#[derive(Clone, Serialize, TS)]
#[ts(rename = "LastFmUserInfo")]
pub struct UserInfo {
  username: String,
  currently_playing: Option<TypescriptTrack>,
}

static PLAYING_TRACKS: LazyLock<RwLock<HashMap<String, UserInfo>>> =
  LazyLock::new(Default::default);

pub fn fetch_lastfm_info(username: &str) -> Option<UserInfo> {
  PLAYING_TRACKS.read().unwrap().get(username).cloned()
}

struct User {
  username: String,
}

pub async fn run(config: &'static Config) {
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
          // currently_playing_recorded: None,
        },
      ))
    })
    .collect();

  let users = config
    .users
    .values()
    .filter_map(|config| {
      let last_fm_username = config.last_fm_username.clone()?;

      Some(User {
        username: last_fm_username,
      })
    })
    .collect::<Vec<_>>();

  let perform_update = async move || {
    join_all(
      users
        .iter()
        .map(|user| update_currently_listening(&user.username, &last_fm_key)),
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

#[derive(Deserialize)]
struct RecentTracksBase {
  #[serde(rename = "recenttracks")]
  recent_tracks: RecentTracks,
}

#[derive(Deserialize)]
struct RecentTracks {
  track: Vec<Track>,
}

pub async fn update_currently_listening(username: &str, api_key: &str) {
  let result = reqwest::get(format!("https://ws.audioscrobbler.com/2.0/?method=user.getrecenttracks&extended=1&user={username}&format=json&api_key={api_key}&limit=1"))
  .and_then(Response::json::<RecentTracksBase>)
  .await;
  match result {
    Ok(response) => {
      let currently_playing = response.recent_tracks.track.into_iter().find_map(|track| {
        let Track::NowPlaying(now_playing) = track else {
          return None;
        };
        Some(now_playing)
      });
      let mut users = PLAYING_TRACKS.write().unwrap();
      if let Some(user) = users.get_mut(username) {
        user.currently_playing = currently_playing.map(|track| {
          let start_time = user
            .currently_playing
            .as_ref()
            .map(|track| track.start_time)
            .unwrap_or(Utc::now().round_subsecs(0));
          TypescriptTrack {
            start_time,
            name: track.name,
            album: track.album,
            url: track.url,
            artist: track.artist,
            image: track.image,
          }
        });
      }
    }
    Err(error) => {
      tracing::error!(
        "failed to request listening status from last.fm for user {username}: {error}"
      );
    }
  }
}
