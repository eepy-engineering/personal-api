use std::{borrow::Cow, collections::HashMap, sync::LazyLock, time::Duration};

use chrono::{DateTime, Local, SubsecRound, Utc};
use futures::TryFutureExt;
use lastfm::{
  artist::Artist,
  imageset::ImageSet,
  track::{NowPlayingTrack, Track},
};
use reqwest::Response;
use serde::{Deserialize, Serialize};
use tokio::sync::Mutex;
use tracing::debug;
use ts_rs::TS;

use crate::config::{Config, has_scope};

#[allow(unused)]
#[derive(Clone, Serialize, TS)]
#[ts(rename = "LastFmImageSet")]
pub struct TypescriptImageSet {
  pub small: Option<String>,
  pub medium: Option<String>,
  pub large: Option<String>,
  pub extralarge: Option<String>,
}

#[allow(unused)]
#[derive(Clone, Serialize, TS)]
#[ts(rename = "LastFmArtist")]
pub struct TypescriptArtist {
  #[ts(as = "TypescriptImageSet")]
  pub image: ImageSet,
  pub name: String,
  pub url: String,
}
#[allow(unused)]
#[derive(Clone, Serialize, TS)]
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

impl PartialEq<NowPlayingTrack> for TypescriptTrack {
  fn eq(&self, other: &NowPlayingTrack) -> bool {
    self.artist == other.artist
      && self.name == other.name
      && self.image == other.image
      && self.album == other.album
      && self.url == other.url
  }
}

#[derive(Clone, Serialize, TS)]
#[ts(rename = "LastFmUserInfo")]
pub struct UserInfo {
  username: String,
  last_song_time: Option<DateTime<Local>>,
  currently_playing: Option<TypescriptTrack>,
}

static PLAYING_TRACKS: LazyLock<Mutex<HashMap<String, UserInfo>>> = LazyLock::new(Default::default);

pub async fn fetch_lastfm_info(username: &str, auth_scopes: &Cow<'static, [String]>) -> Option<UserInfo> {
  PLAYING_TRACKS
    .lock()
    .await
    .get(username)
    .cloned()
    .map(|mut user| {
      user
        .last_song_time
        .take_if(|_| !has_scope(&auth_scopes, "lastfm.lasttime"));
      user
    })
}

struct User {
  username: String,
}

pub async fn run(config: &'static Config) {
  let Some(last_fm_key) = &config.last_fm_key else {
    return;
  };

  *PLAYING_TRACKS.lock().await = config
    .users
    .values()
    .filter_map(|config| {
      let last_fm_username = config.last_fm_username.clone()?;
      Some((
        last_fm_username.to_owned(),
        UserInfo {
          username: last_fm_username,
          last_song_time: None,
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
    for user in &users {
      update_currently_listening(&user.username, &last_fm_key).await;
    }
  };

  perform_update().await;

  tokio::spawn(async move {
    loop {
      tokio::time::sleep(Duration::from_secs(15)).await;
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
          debug!("{username} is not listening to music");
          return None;
        };
        Some(now_playing)
      });
      let mut users = PLAYING_TRACKS.lock().await;
      if let Some(user) = users.get_mut(username) {
        user.currently_playing = currently_playing.map(|track| {
          let start_time = user
            .currently_playing
            .as_ref()
            .filter(|previous| **previous == track)
            .map(|track| track.start_time)
            .unwrap_or_else(|| Utc::now().round_subsecs(0));

          TypescriptTrack {
            start_time,
            name: track.name,
            album: track.album,
            url: track.url,
            artist: track.artist,
            image: track.image,
          }
        });
        user.last_song_time = Some(Local::now());
      }
    }
    Err(error) => {
      tracing::error!(
        "failed to request listening status from last.fm for user {username}: {error}"
      );
    }
  }
}
