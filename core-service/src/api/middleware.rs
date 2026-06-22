use axum::{
    extract::Request,
    http::{header, StatusCode},
    middleware::Next,
    response::Response,
};
use governor::{Quota, RateLimiter};
use std::num::NonZeroU32;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, LazyLock};

use crate::api::AppState;
use crate::auth;

static REST_LIMITER: LazyLock<
    Arc<
        RateLimiter<
            governor::state::NotKeyed,
            governor::state::InMemoryState,
            governor::clock::DefaultClock,
        >,
    >,
> = LazyLock::new(|| {
    Arc::new(RateLimiter::direct(Quota::per_minute(
        NonZeroU32::new(100).expect("100 > 0"),
    )))
});

static WS_CONNECTIONS: AtomicU64 = AtomicU64::new(0);
const MAX_WS_CONNECTIONS: u64 = 5;

pub async fn rate_limit(req: Request, next: Next) -> Result<Response, StatusCode> {
    REST_LIMITER
        .check()
        .map_err(|_| StatusCode::TOO_MANY_REQUESTS)?;
    Ok(next.run(req).await)
}

pub fn ws_connection_acquired() -> Result<(), StatusCode> {
    let current = WS_CONNECTIONS.fetch_add(1, Ordering::SeqCst);
    if current >= MAX_WS_CONNECTIONS {
        WS_CONNECTIONS.fetch_sub(1, Ordering::SeqCst);
        return Err(StatusCode::TOO_MANY_REQUESTS);
    }
    Ok(())
}

pub fn ws_connection_released() {
    WS_CONNECTIONS.fetch_sub(1, Ordering::SeqCst);
}

pub async fn require_bearer(
    axum::extract::State(state): axum::extract::State<Arc<AppState>>,
    req: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    let token = extract_bearer(req.headers());
    if auth::validate_token(state.deps.api_token.read().as_str(), token).is_err() {
        return Err(StatusCode::UNAUTHORIZED);
    }
    Ok(next.run(req).await)
}

pub fn extract_bearer(headers: &axum::http::HeaderMap) -> Option<&str> {
    headers
        .get(header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
        .or_else(|| headers.get("X-API-Token").and_then(|v| v.to_str().ok()))
}

#[allow(dead_code)]
pub fn extract_bearer_from_query(query: &str) -> Option<String> {
    query.split('&').find_map(|pair| {
        let (key, val) = pair.split_once('=')?;

        if key == "token" {
            Some(val.to_string())
        } else {
            None
        }
    })
}
