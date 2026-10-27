//! Request and response payloads for the HTTP API, plus the error type that
//! every non-2xx response is rendered from.

use crate::pipeline::config::TraceConfig;
use crate::pipeline::trace::TraceStats;
use crate::vector::svg::SvgOptions;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde::{Deserialize, Serialize};

/// Upper bound on an uploaded image, so one request cannot exhaust memory.
pub const MAX_UPLOAD_BYTES: usize = 16 * 1024 * 1024;

/// Settings shared by every request handler.
#[derive(Debug, Clone)]
pub struct ServerConfig {
    /// Default tracing settings; individual requests may override them.
    pub defaults: TraceConfig,
    /// Decimal places on emitted coordinates.
    pub precision: u32,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            defaults: TraceConfig::default(),
            precision: SvgOptions::default().precision,
        }
    }
}

/// Liveness payload.
#[derive(Debug, Serialize)]
pub struct Health {
    pub status: &'static str,
    pub version: &'static str,
}

/// One palette entry, as reported by `/info` and returned alongside a trace.
#[derive(Debug, Serialize)]
pub struct SwatchInfo {
    pub color: String,
    pub pixels: u32,
}

/// Image facts and the palette the tracer would use.
#[derive(Debug, Serialize)]
pub struct ImageInfo {
    pub width: u32,
    pub height: u32,
    pub palette: Vec<SwatchInfo>,
}

/// The `GET /info` query string: the image plus optional overrides.
///
/// The override fields are spelled out rather than `#[serde(flatten)]`-ed,
/// because query strings are parsed by `serde_urlencoded`, which has no support
/// for flattening a struct.
#[derive(Debug, Default, Deserialize)]
pub struct InfoQuery {
    /// Base64 PNG or JPEG.
    #[serde(default)]
    pub image: Option<String>,
    #[serde(default)]
    pub colors: Option<usize>,
    #[serde(default)]
    pub threshold: Option<u8>,
    #[serde(default)]
    pub simplify_tolerance: Option<f64>,
    #[serde(default)]
    pub bezier_tolerance: Option<f64>,
    #[serde(default)]
    pub min_area: Option<usize>,
    #[serde(default)]
    pub background_index: Option<usize>,
    #[serde(default)]
    pub precision: Option<u32>,
}

impl InfoQuery {
    /// The subset of this query that adjusts tracing.
    pub(super) fn overrides(&self) -> TraceOverrides {
        TraceOverrides {
            colors: self.colors,
            threshold: self.threshold,
            simplify_tolerance: self.simplify_tolerance,
            bezier_tolerance: self.bezier_tolerance,
            min_area: self.min_area,
            background_index: self.background_index,
            precision: self.precision,
        }
    }
}

/// Per-request tracing overrides; absent fields fall back to the defaults.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct TraceOverrides {
    pub colors: Option<usize>,
    pub threshold: Option<u8>,
    pub simplify_tolerance: Option<f64>,
    pub bezier_tolerance: Option<f64>,
    pub min_area: Option<usize>,
    pub background_index: Option<usize>,
    /// Decimal places on emitted coordinates.
    pub precision: Option<u32>,
}

impl TraceOverrides {
    /// Layer the overrides onto a configuration and validate the result.
    pub fn apply(&self, base: &TraceConfig) -> crate::Result<TraceConfig> {
        let mut config = base.clone();
        if let Some(colors) = self.colors {
            config.colors = colors;
        }
        if let Some(threshold) = self.threshold {
            config.threshold = threshold;
        }
        if let Some(tolerance) = self.simplify_tolerance {
            config.simplify_tolerance = tolerance;
        }
        if let Some(tolerance) = self.bezier_tolerance {
            config.bezier_tolerance = tolerance;
        }
        if let Some(min_area) = self.min_area {
            config.min_area = min_area;
        }
        if let Some(index) = self.background_index {
            config.background_index = index;
        }
        config.validate()?;
        Ok(config)
    }
}

/// The tracing result, as JSON.
#[derive(Debug, Serialize)]
pub struct TraceResponse {
    pub width: u32,
    pub height: u32,
    pub svg: String,
    pub stats: StatsInfo,
}

/// The counters `TraceStats` carries, flattened for JSON.
#[derive(Debug, Serialize)]
pub struct StatsInfo {
    pub colors: usize,
    pub regions: usize,
    pub contours: usize,
    pub holes: usize,
    pub curves: usize,
    pub points_after_simplify: usize,
    pub bytes: usize,
}

impl From<&TraceStats> for StatsInfo {
    fn from(stats: &TraceStats) -> Self {
        Self {
            colors: stats.colors,
            regions: stats.regions,
            contours: stats.contours,
            holes: stats.holes,
            curves: stats.curves,
            points_after_simplify: stats.points_after_simplify,
            bytes: 0,
        }
    }
}

/// Error body returned for every non-2xx response.
///
/// The status is carried rather than re-derived from the message, so a new
/// error site can never disagree with its own status code.
#[derive(Debug, Serialize)]
pub struct ApiError {
    #[serde(skip)]
    status: StatusCode,
    pub error: String,
}

impl ApiError {
    pub(super) fn bad_request(message: impl Into<String>) -> Self {
        Self {
            status: StatusCode::BAD_REQUEST,
            error: message.into(),
        }
    }

    fn from_library(error: crate::Error) -> Self {
        let status = match &error {
            crate::Error::Config(_) => StatusCode::BAD_REQUEST,
            crate::Error::Decode(_, _) | crate::Error::InvalidFormat(_) => {
                StatusCode::UNSUPPORTED_MEDIA_TYPE
            }
            crate::Error::Open(_, _) => StatusCode::NOT_FOUND,
            _ => StatusCode::INTERNAL_SERVER_ERROR,
        };
        Self {
            status,
            error: error.to_string(),
        }
    }
}

impl From<crate::Error> for ApiError {
    fn from(error: crate::Error) -> Self {
        Self::from_library(error)
    }
}

/// A malformed multipart body is the client's fault, not ours.
impl From<axum::extract::multipart::MultipartError> for ApiError {
    fn from(error: axum::extract::multipart::MultipartError) -> Self {
        Self::bad_request(format!("malformed multipart body: {error}"))
    }
}

/// Turn an `ApiError` into a JSON response with the carried status.
impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        (self.status, Json(self)).into_response()
    }
}
