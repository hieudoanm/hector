//! HTTP API: a self-hosted tracer with `GET /health`, `GET /info`, `POST /vectify`.
//!
//! `router()` wires the three handlers in `handlers.rs` onto the shared layer
//! stack; `types.rs` owns every payload that crosses the wire, and `decode.rs`
//! turns the two ways an image can arrive (base64 query, multipart upload) into
//! bytes.

mod decode;
mod handlers;
mod types;

pub use types::{
    ApiError, Health, ImageInfo, InfoQuery, ServerConfig, StatsInfo, SwatchInfo, TraceOverrides,
    TraceResponse, MAX_UPLOAD_BYTES,
};

use axum::routing::{get, post};
use axum::Router;
use std::sync::Arc;

use handlers::{health, info, vectify};

/// Build the router with all three routes and the shared layer stack.
pub fn router(config: ServerConfig) -> Router {
    Router::new()
        .route("/health", get(health))
        .route("/info", get(info))
        .route("/vectify", post(vectify))
        .layer(axum::extract::DefaultBodyLimit::max(MAX_UPLOAD_BYTES))
        .layer(tower_http::cors::CorsLayer::permissive())
        .with_state(Arc::new(config))
}
