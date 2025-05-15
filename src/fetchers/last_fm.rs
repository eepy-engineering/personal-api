use std::{
  collections::HashMap,
  sync::{LazyLock, RwLock},
  time::Duration,
};

use chrono::{DateTime, Utc};
use futures::{FutureExt, StreamExt, TryFutureExt, future::join_all};
use lastfm::{artist::Artist, imageset::ImageSet, track::RecordedTrack};
use serde::Serialize;
use ts_rs::TS;

use crate::config::Config;

#[allow(unused)]
#[derive(Serialize, TS)]
#[ts(rename = "LastFmImageSet")]
pub struct TypescriptImageSet {
  pub small: Option<String>,
  pub medium: Option<String>,
  pub large: Option<String>,
  pub extralarge: Option<String>,
}

#[allow(unused)]
#[derive(Serialize, TS)]
#[ts(rename = "LastFmArtist")]
pub struct TypescriptArtist {
  #[ts(as = "TypescriptImageSet")]
  pub image: ImageSet,
  pub name: String,
  pub url: String,
}
#[allow(unused)]
#[derive(Serialize, TS)]
#[ts(rename = "LastFmTrack")]
pub struct TypescriptRecordedTrack {
  #[ts(as = "TypescriptArtist")]
  pub artist: Artist,
  pub name: String,
  #[ts(as = "TypescriptImageSet")]
  pub image: ImageSet,
  pub album: String,
  pub url: String,
  pub date: DateTime<Utc>,
}

#[derive(Clone, Serialize, TS)]
#[ts(rename = "LastFmUserInfo")]
pub struct UserInfo {
  username: String,
  #[ts(as = "Option<TypescriptRecordedTrack>")]
  currently_playing: Option<RecordedTrack>, // todo: start time on now playing track
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
  let a = client
    .now_playing()
    .and_then(async |current_track| {
      if let Some(current_track) = current_track {
        let recorded_tracks = client.clone().recent_tracks(None, None).await?;
        let mut tracks = std::pin::pin!(recorded_tracks.into_stream());
        if let Some(Ok(track)) = tracks.next().await {
          if current_track.url == track.url {
            return Ok(Some(track));
          }
        }
      }

      return Ok(None);
    })
    .await;
  match a {
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
