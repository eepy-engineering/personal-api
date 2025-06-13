use std::{
  borrow::Cow,
  collections::HashMap,
  sync::{LazyLock, RwLock},
  time::Duration,
};

use serde::{Deserialize, Serialize};
use tracing::{info, warn};
use ts_rs::TS;
use tzf_rs::DefaultFinder;

use crate::config::{Config, has_scope};

#[derive(TS, Clone, Serialize)]
pub struct Location {
  country: String,
  #[serde(skip_serializing_if = "Option::is_none")]
  locality: Option<String>,
  #[serde(skip_serializing_if = "Option::is_none")]
  latitude: Option<f64>,
  #[serde(skip_serializing_if = "Option::is_none")]
  longitude: Option<f64>,
  #[ts(skip)]
  #[serde(skip_serializing_if = "Option::is_none")]
  pub time_zone: Option<&'static str>,
}

struct DeviceInfo {
  country: String,
  locality: String,
  latitude: f64,
  longitude: f64,
}

static DEVICE_INFO: LazyLock<RwLock<HashMap<String, DeviceInfo>>> = LazyLock::new(RwLock::default);
static FINDER: LazyLock<DefaultFinder> = LazyLock::new(DefaultFinder::new);

pub fn get_user_info(device_id: &str, auth_scopes: Cow<'static, [String]>) -> Option<Location> {
  DEVICE_INFO
    .read()
    .unwrap()
    .get(device_id)
    .map(|location| Location {
      country: location.country.clone(),
      locality: has_scope(&auth_scopes, "icloud.city")
        .then_some(&location.locality)
        .cloned(),
      latitude: has_scope(&auth_scopes, "icloud.latlong").then_some(location.latitude),
      longitude: has_scope(&auth_scopes, "icloud.latlong").then_some(location.longitude),
      time_zone: has_scope(&auth_scopes, "icloud.latlong")
        .then(|| FINDER.get_tz_name(location.longitude, location.latitude)),
    })
}

pub fn run(config: &'static Config) {
  let Some((server, password)) = config
    .bluebubbles_server
    .as_ref()
    .zip(config.bluebubbles_server_password.as_ref())
  else {
    warn!("icloud fetcher not set up");
    return;
  };

  info!("started icloud fetcher");

  tokio::spawn(async move {
    let mut interval = tokio::time::interval(Duration::from_secs(5));

    loop {
      interval.tick().await;

      let Ok(response) = reqwest::get(format!(
        "{server}/api/v1/icloud/findmy/devices?password={password}"
      ))
      .await
      else {
        // info!("failed to fetch");
        continue;
      };

      let Ok(devices): Result<Response, _> = response.json().await else {
        // info!("failed to deserialize json");
        continue;
      };

      let mut info = DEVICE_INFO.write().unwrap();
      for device in devices.data {
        if let Some((address, location)) = Option::zip(device.address, device.location) {
          info.insert(
            device.id,
            DeviceInfo {
              country: address.country,
              locality: address.locality,
              latitude: location.latitude,
              longitude: location.longitude,
            },
          );
        } else {
          info.remove(&device.id);
          continue;
        }
      }
    }
  });
}

#[derive(Debug, Deserialize)]
struct Response {
  data: Vec<Device>,
}

#[derive(Debug, Deserialize)]

struct Device {
  id: String,
  location: Option<DeviceLocation>,
  address: Option<DeviceAddress>,
}

#[derive(Debug, Deserialize)]
struct DeviceLocation {
  latitude: f64,
  longitude: f64,
  // timestamp: u64,
}

#[derive(Debug, Deserialize)]
struct DeviceAddress {
  country: String,
  locality: String,
}
