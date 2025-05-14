use std::time::Duration;

use axum::{
  extract::{Request, State},
  middleware::Next,
  response::IntoResponse,
};
use axum_extra::{TypedHeader, headers::CacheControl};

pub async fn age_caching(
  State(max_age): State<u64>,
  request: Request,
  next: Next,
) -> impl IntoResponse {
  let (parts, body) = next.run(request).await.into_parts();

  (
    parts,
    TypedHeader(CacheControl::new().with_max_age(Duration::from_secs(max_age))),
    body,
  )
}
