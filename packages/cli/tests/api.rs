//! HTTP API tests: the three routes, their error paths, and their status codes.

use axum::body::Body;
use axum::http::{Request, StatusCode};
use hector::{router, to_svg, trace, Raster, ServerConfig, SvgOptions, TraceConfig};
use http_body_util::BodyExt as _;
use serde_json::{json, Value};
use tower::ServiceExt as _;

/// 8x8 image: a red square on white, encoded as PNG bytes.
fn png_bytes() -> Vec<u8> {
    let image = image::DynamicImage::ImageRgba8(image::RgbaImage::from_fn(8, 8, |x, y| {
        if (2..6).contains(&x) && (2..6).contains(&y) {
            image::Rgba([220, 30, 30, 255])
        } else {
            image::Rgba([255, 255, 255, 255])
        }
    }));
    encode(image::ImageFormat::Png, &image)
}

/// Encode to an in-memory buffer.
fn encode(format: image::ImageFormat, image: &image::DynamicImage) -> Vec<u8> {
    let mut buffer = Vec::new();
    image
        .write_to(&mut std::io::Cursor::new(&mut buffer), format)
        .expect("encode image");
    buffer
}

/// URL-safe base64 of the test PNG; `+` and `/` cannot survive a query string.
fn png_base64() -> String {
    use base64::Engine as _;
    base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(png_bytes())
}

/// A multipart body with one `image` field and an optional `options` field.
fn multipart_body(image: &[u8], options: Option<&str>) -> Vec<u8> {
    let mut body = Vec::new();
    for (name, value, filename) in [("image", image.to_vec(), Some("logo.png"))]
        .into_iter()
        .chain(options.map(|o| ("options", o.as_bytes().to_vec(), None)))
    {
        body.extend_from_slice(b"--BOUNDARY\r\n");
        match filename {
            Some(file) => body.extend_from_slice(
                format!("Content-Disposition: form-data; name=\"{name}\"; filename=\"{file}\"\r\nContent-Type: image/png\r\n\r\n")
                    .as_bytes(),
            ),
            None => body.extend_from_slice(
                format!("Content-Disposition: form-data; name=\"{name}\"\r\n\r\n").as_bytes(),
            ),
        }
        body.extend_from_slice(&value);
        body.extend_from_slice(b"\r\n");
    }
    body.extend_from_slice(b"--BOUNDARY--\r\n");
    body
}

/// POST the multipart body and return the status and parsed JSON.
async fn post_vectify(options: Option<&str>, accept: &str) -> (StatusCode, Value) {
    let body = multipart_body(&png_bytes(), options);
    let request = Request::builder()
        .method("POST")
        .uri("/vectify")
        .header("content-type", "multipart/form-data; boundary=BOUNDARY")
        .header("accept", accept)
        .body(Body::from(body))
        .expect("build request");
    let response = router(ServerConfig::default())
        .oneshot(request)
        .await
        .expect("route");
    let status = response.status();
    let bytes = response
        .into_body()
        .collect()
        .await
        .expect("read body")
        .to_bytes();
    (
        status,
        serde_json::from_slice(&bytes).unwrap_or(Value::Null),
    )
}

#[tokio::test]
async fn health_reports_ok_and_the_version() {
    let response = router(ServerConfig::default())
        .oneshot(
            Request::builder()
                .uri("/health")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("route");
    assert_eq!(response.status(), StatusCode::OK);
    let body = response
        .into_body()
        .collect()
        .await
        .expect("read body")
        .to_bytes();
    let json: Value = serde_json::from_slice(&body).expect("json");
    assert_eq!(json["status"], "ok");
    assert_eq!(json["version"], env!("CARGO_PKG_VERSION"));
}

#[tokio::test]
async fn info_returns_dimensions_and_a_palette() {
    let request = Request::builder()
        .uri(format!("/info?image={}", png_base64()))
        .body(Body::empty())
        .expect("request");
    let response = router(ServerConfig::default())
        .oneshot(request)
        .await
        .expect("route");
    assert_eq!(response.status(), StatusCode::OK);
    let body = response
        .into_body()
        .collect()
        .await
        .expect("read body")
        .to_bytes();
    let json: Value = serde_json::from_slice(&body).expect("json");
    assert_eq!(json["width"], 8);
    assert_eq!(json["height"], 8);
    assert_eq!(json["palette"].as_array().expect("palette").len(), 2);
}

#[tokio::test]
async fn info_without_an_image_is_a_bad_request() {
    let response = router(ServerConfig::default())
        .oneshot(
            Request::builder()
                .uri("/info")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("route");
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn info_rejects_base64_that_is_not_an_image() {
    let request = Request::builder()
        .uri("/info?image=aGVsbG8gd29ybGQ")
        .body(Body::empty())
        .expect("request");
    let response = router(ServerConfig::default())
        .oneshot(request)
        .await
        .expect("route");
    assert_eq!(response.status(), StatusCode::UNSUPPORTED_MEDIA_TYPE);
}

#[tokio::test]
async fn vectify_returns_svg_and_stats_as_json() {
    let (status, json) = post_vectify(None, "application/json").await;
    assert_eq!(status, StatusCode::OK);
    assert!(json["svg"].as_str().expect("svg").contains("<svg"));
    assert_eq!(json["width"], 8);
    assert_eq!(
        json["stats"]["bytes"].as_u64().expect("bytes"),
        json["svg"].as_str().unwrap().len() as u64
    );
    assert_eq!(json["stats"]["colors"], 2);
}

#[tokio::test]
async fn vectify_honours_the_accept_header() {
    let body = multipart_body(&png_bytes(), None);
    let request = Request::builder()
        .method("POST")
        .uri("/vectify")
        .header("content-type", "multipart/form-data; boundary=BOUNDARY")
        .header("accept", "image/svg+xml")
        .body(Body::from(body))
        .expect("request");
    let response = router(ServerConfig::default())
        .oneshot(request)
        .await
        .expect("route");
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(response.headers()["content-type"], "image/svg+xml");
    let bytes = response
        .into_body()
        .collect()
        .await
        .expect("read body")
        .to_bytes();
    assert!(bytes.starts_with(b"<?xml"));
}

#[tokio::test]
async fn vectify_applies_json_overrides() {
    let (_, plain) = post_vectify(None, "application/json").await;
    assert!(plain["svg"].as_str().expect("svg").contains("<path"));
    // A min-area larger than the whole image must drop every region.
    let (status, json) = post_vectify(
        Some(&json!({ "min_area": 1000 }).to_string()),
        "application/json",
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert!(
        !json["svg"].as_str().expect("svg").contains("<path"),
        "override reaches the tracer"
    );
}

#[tokio::test]
async fn vectify_rejects_an_invalid_override() {
    let (status, json) = post_vectify(
        Some(&json!({ "colors": 0 }).to_string()),
        "application/json",
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert!(json["error"].as_str().expect("error").contains("--colors"));
}

#[tokio::test]
async fn vectify_rejects_malformed_options_json() {
    let (status, json) = post_vectify(Some("{not json"), "application/json").await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert!(json["error"]
        .as_str()
        .expect("error")
        .contains("not valid JSON"));
}

#[tokio::test]
async fn vectify_without_an_image_field_is_a_bad_request() {
    let request = Request::builder()
        .method("POST")
        .uri("/vectify")
        .header("content-type", "multipart/form-data; boundary=BOUNDARY")
        .body(Body::from("--BOUNDARY--\r\n"))
        .expect("request");
    let response = router(ServerConfig::default())
        .oneshot(request)
        .await
        .expect("route");
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn vectify_rejects_bytes_that_are_not_an_image() {
    let body = multipart_body(b"definitely not a png", None);
    let request = Request::builder()
        .method("POST")
        .uri("/vectify")
        .header("content-type", "multipart/form-data; boundary=BOUNDARY")
        .body(Body::from(body))
        .expect("request");
    let response = router(ServerConfig::default())
        .oneshot(request)
        .await
        .expect("route");
    assert_eq!(response.status(), StatusCode::UNSUPPORTED_MEDIA_TYPE);
}

#[tokio::test]
async fn get_on_vectify_is_not_allowed() {
    let response = router(ServerConfig::default())
        .oneshot(
            Request::builder()
                .uri("/vectify")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("route");
    assert_eq!(response.status(), StatusCode::METHOD_NOT_ALLOWED);
}

#[tokio::test]
async fn unknown_routes_are_not_found() {
    let response = router(ServerConfig::default())
        .oneshot(
            Request::builder()
                .uri("/nope")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("route");
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

#[test]
fn overrides_only_change_the_fields_they_set() {
    let base = hector::TraceConfig::default();
    let overrides = hector::server::TraceOverrides {
        colors: Some(4),
        ..Default::default()
    };
    let config = overrides.apply(&base).expect("valid");
    assert_eq!(config.colors, 4);
    assert_eq!(config.threshold, base.threshold);
    assert_eq!(config.bezier_tolerance, base.bezier_tolerance);
}

#[test]
fn an_empty_override_set_leaves_the_base_untouched() {
    let base = hector::TraceConfig::default();
    let config = hector::server::TraceOverrides::default()
        .apply(&base)
        .expect("valid");
    assert_eq!(config, base);
}

#[test]
fn overrides_are_validated_after_being_applied() {
    let base = hector::TraceConfig::default();
    let overrides = hector::server::TraceOverrides {
        bezier_tolerance: Some(0.0),
        ..Default::default()
    };
    assert!(overrides.apply(&base).is_err());
}

#[tokio::test]
async fn jpeg_uploads_are_accepted_too() {
    let jpeg = encode(
        image::ImageFormat::Jpeg,
        &image::DynamicImage::ImageRgba8(image::RgbaImage::from_fn(8, 8, |_, _| {
            image::Rgba([10, 120, 200, 255])
        })),
    );
    let body = multipart_body(&jpeg, None);
    let request = Request::builder()
        .method("POST")
        .uri("/vectify")
        .header("content-type", "multipart/form-data; boundary=BOUNDARY")
        .body(Body::from(body))
        .expect("request");
    let response = router(ServerConfig::default())
        .oneshot(request)
        .await
        .expect("route");
    assert_eq!(response.status(), StatusCode::OK);
}

/// The `precision` option must reach `to_svg`, so the response has to match what
/// the library produces for the same precision.
#[tokio::test]
async fn precision_option_reaches_the_svg_backend() {
    for precision in [0, 4] {
        let (_, json) = post_vectify(
            Some(&json!({ "precision": precision }).to_string()),
            "application/json",
        )
        .await;
        let raster = Raster::from_bytes(&png_bytes(), "test").expect("raster");
        let (image, _) = trace(&raster, &TraceConfig::default()).expect("trace");
        let expected = to_svg(
            &image,
            &SvgOptions {
                precision,
                ..SvgOptions::default()
            },
        );
        assert_eq!(json["svg"].as_str().expect("svg"), expected);
    }
}

/// `serde_urlencoded` does not support `flatten`, so each override has to be an
/// ordinary query field. An unknown field would fail deserialisation outright,
/// which is what makes this a meaningful check.
#[tokio::test]
async fn info_query_overrides_are_applied() {
    let palette_len = |extra: &str| {
        let uri = format!("/info?image={}{extra}", png_base64());
        async move {
            let response = router(ServerConfig::default())
                .oneshot(
                    Request::builder()
                        .uri(uri)
                        .body(Body::empty())
                        .expect("request"),
                )
                .await
                .expect("route");
            assert_eq!(response.status(), StatusCode::OK);
            let json: Value = serde_json::from_slice(
                &response
                    .into_body()
                    .collect()
                    .await
                    .expect("read body")
                    .to_bytes(),
            )
            .expect("json");
            json["palette"].as_array().expect("palette").len()
        }
    };
    // The fixture is two-colour, so the default 8-colour palette already yields
    // two swatches; asking for 1 forces binary mode, which is also two tones.
    assert_eq!(palette_len("").await, 2);
    assert_eq!(palette_len("&colors=1").await, 2);
    // An unparseable override must be a client error, not a silent default.
    let response = router(ServerConfig::default())
        .oneshot(
            Request::builder()
                .uri(format!("/info?image={}&colors=lots", png_base64()))
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("route");
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}
