use axum::{extract::Request, middleware::Next, response::IntoResponse};
use axum_extra::{TypedHeader, headers::AccessControlAllowOrigin};

pub async fn cors(request: Request, next: Next) -> impl IntoResponse {
  let (parts, body) = next.run(request).await.into_parts();

  (parts, TypedHeader(AccessControlAllowOrigin::ANY), body)
}
