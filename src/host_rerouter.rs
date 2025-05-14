use std::sync::Arc;

use axum::{
  extract::{Request, State},
  http::Uri,
  middleware::Next,
  response::Response,
};
use axum_extra::extract::Host;
use replace_with::replace_with;
use tracing::warn;

use crate::host_config::HandlerConfig;

fn rewrite_uri(uri: Uri) -> Uri {
  let mut parts = uri.into_parts();

  parts.path_and_query = parts.path_and_query.map(|path| {
    warn!(
      "path: {path:?}, start_with: {}",
      path.as_str().starts_with("/api")
    );
    if path.as_str().starts_with("/api") {
      let mut new_path = &path.as_str()[4..];
      if new_path.is_empty() {
        new_path = "/"
      }
      match path.query() {
        Some(query) => format!("{new_path}?{query}").try_into().unwrap(),
        None => new_path.try_into().unwrap(),
      }
    } else {
      path
    }
  });

  Uri::from_parts(parts).expect("failed to rebuild uri")
}

pub async fn host_rerouter(
  State(host_state): State<Arc<HandlerConfig>>,
  Host(host): Host,
  mut request: Request,
  next: Next,
) -> Response {
  let Some(hostname) = host.split(":").next() else {
    unreachable!("handled by axum_extra::extract::Host");
  };

  println!("handling before: {}", request.uri());

  if host_state.domains.contains_key(hostname) {
    replace_with(
      request.uri_mut(),
      || Uri::from_static("https://docs.rs/ahv/latest/ahv"),
      |uri| rewrite_uri(uri),
    );
  }

  println!("handling next: {:?}", request.uri());

  next.run(request).await
}
