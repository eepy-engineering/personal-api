mod config;
mod fetchers;
mod host_config;
mod middleware;
mod routes;

use std::fs::read_to_string;

use axum::{Router, ServiceExt, handler::Handler, middleware as mw, routing::get};
use host_config::HandlerConfig;
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
  let config = &*Box::leak(toml::from_str(&config).expect("failed to parse config"));

  fetchers::discord::run_discord_bot(&config).await?;
  fetchers::last_fm::run(&config).await;
  fetchers::steam::run(&config).await;
  fetchers::icloud::run(&config);

  let handler_config = &*Box::leak(Box::new(HandlerConfig::new(&config)));
  let middleware = mw::from_fn_with_state(handler_config, middleware::host_rerouter);

  let app = Router::new()
    .route(
      "/",
      get(root_page.layer(mw::from_fn_with_state(259200, middleware::age_caching))),
    )
    .route(
      "/users",
      get(get_users.layer(mw::from_fn_with_state(60, middleware::age_caching))),
    )
    .route(
      "/user",
      get(get_host_user.layer(mw::from_fn_with_state(10, middleware::age_caching))),
    )
    .route(
      "/user/{user}",
      get(get_user.layer(mw::from_fn_with_state(10, middleware::age_caching))),
    )
    .layer(mw::from_fn(middleware::cors))
    .with_state(handler_config);

  let middleware = middleware.layer(app);

  let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await?;
  axum::serve(listener, middleware.into_make_service()).await?;
  Ok(())
}
