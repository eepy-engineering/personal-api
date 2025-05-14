mod config;
mod fetchers;
pub mod host_config;
mod host_rerouter;
pub mod routes;

use std::{fs::read_to_string, sync::Arc};

use axum::{
  Router, ServiceExt,
  middleware::{self},
  routing::get,
};
use host_config::HandlerConfig;
use host_rerouter::host_rerouter;
use routes::{
  get_host_user::get_host_user, get_user::get_user, get_users::get_users, root::root_page,
};
use tower::Layer;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
  tracing_subscriber::fmt().init();
  let config_arg = std::env::args()
    .nth(1)
    .expect("no config was provided as an argument");
  let config = read_to_string(config_arg).expect("failed to read config");
  let config = Arc::new(toml::from_str(&config).expect("failed to parse config"));

  fetchers::discord::run_discord_bot(&config).await?;
  fetchers::last_fm::run(&config).await;
  fetchers::steam::run(&config).await;

  let handler_config = Arc::new(HandlerConfig::new(&config));
  let middleware = middleware::from_fn_with_state(handler_config.clone(), host_rerouter);

  let app = Router::new()
    .route("/", get(root_page))
    .route("/users", get(get_users))
    .route("/user", get(get_host_user))
    .route("/user/{user}", get(get_user))
    .with_state(handler_config);

  let middleware = middleware.layer(app);

  let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await?;
  axum::serve(listener, middleware.into_make_service()).await?;
  Ok(())
}
