//! Request handlers, one per route, plus the multipart field reader.

use super::decode::{decode_base64, swatch_info};
use super::types::{
    ApiError, Health, ImageInfo, InfoQuery, StatsInfo, TraceOverrides, TraceResponse,
};
use super::ServerConfig;
use crate::core::raster::Raster;
use crate::pipeline::trace::trace;
use crate::vector::svg::{to_svg, SvgOptions};
use axum::extract::{Multipart, Query, State};
use axum::http::{header, HeaderMap};
use axum::response::{IntoResponse, Response};
use axum::Json;
use std::sync::Arc;

/// `GET /health` — liveness probe for orchestrators.
pub async fn health() -> Json<Health> {
    Json(Health {
        status: "ok",
        version: env!("CARGO_PKG_VERSION"),
    })
}

/// `GET /info` — the palette an image would be traced with, without tracing.
pub async fn info(
    State(server): State<Arc<ServerConfig>>,
    Query(query): Query<InfoQuery>,
) -> Result<Json<ImageInfo>, ApiError> {
    let overrides = query.overrides();
    let image = query
        .image
        .ok_or_else(|| ApiError::bad_request("GET /info needs `?image=<base64 png-or-jpeg>`"))?;
    let bytes = decode_base64(&image)?;
    let raster = Raster::from_bytes(&bytes, "query")?;
    let config = overrides.apply(&server.defaults)?;
    let map = crate::preprocess::label(&raster, config.color_mode(), config.backdrop);
    Ok(Json(ImageInfo {
        width: raster.width(),
        height: raster.height(),
        palette: map.palette.iter().map(swatch_info).collect(),
    }))
}

/// `POST /vectify` — multipart upload traced to JSON, or SVG when asked.
pub async fn vectify(
    State(config): State<Arc<ServerConfig>>,
    accept: HeaderMap,
    multipart: Multipart,
) -> Result<Response, ApiError> {
    let (bytes, overrides) = read_upload(multipart).await?;
    let raster = Raster::from_bytes(&bytes, "upload")?;
    let precision = overrides.precision.unwrap_or(config.precision);
    let config = overrides.apply(&config.defaults)?;
    let (image, stats) = trace(&raster, &config)?;
    let svg = to_svg(
        &image,
        &SvgOptions {
            precision,
            ..SvgOptions::default()
        },
    );
    Ok(if wants_svg(&accept) {
        ([(header::CONTENT_TYPE, "image/svg+xml")], svg).into_response()
    } else {
        Json(TraceResponse {
            width: image.width,
            height: image.height,
            stats: StatsInfo {
                bytes: svg.len(),
                ..(&stats).into()
            },
            svg,
        })
        .into_response()
    })
}

/// Pull the `image` field and the JSON `options` field out of a multipart body.
async fn read_upload(mut multipart: Multipart) -> Result<(Vec<u8>, TraceOverrides), ApiError> {
    let mut bytes = None;
    let mut overrides = TraceOverrides::default();
    while let Some(field) = multipart.next_field().await? {
        match field.name() {
            Some("image") => bytes = Some(field.bytes().await?.to_vec()),
            Some("options") => {
                let raw = field.text().await?;
                overrides = serde_json::from_str(&raw).map_err(|e| {
                    ApiError::bad_request(format!("`options` is not valid JSON: {e}"))
                })?;
            }
            _ => {}
        }
    }
    let bytes =
        bytes.ok_or_else(|| ApiError::bad_request("multipart body needs an `image` field"))?;
    Ok((bytes, overrides))
}

/// True when the client prefers `image/svg+xml` over JSON.
fn wants_svg(headers: &HeaderMap) -> bool {
    headers
        .get(header::ACCEPT)
        .and_then(|value| value.to_str().ok())
        .is_some_and(|accept| accept.contains("image/svg+xml"))
}
