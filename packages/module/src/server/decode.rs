//! Decoding helpers shared by the `/info` query and the `/vectify` upload.

use super::types::{ApiError, SwatchInfo};

/// Every base64 dialect a client might plausibly send.
///
/// URL-safe is first because it is the only one that survives a query string
/// unencoded: standard base64's `+` arrives as a space. Padding is optional in
/// both, so `=`-stripped input still decodes.
const BASE64_ENGINES: &[base64::engine::GeneralPurpose] = &[
    base64::engine::general_purpose::URL_SAFE_NO_PAD,
    base64::engine::general_purpose::URL_SAFE,
    base64::engine::general_purpose::STANDARD_NO_PAD,
    base64::engine::general_purpose::STANDARD,
];

/// Decode base64, which is how a query string carries an image.
pub(super) fn decode_base64(text: &str) -> Result<Vec<u8>, ApiError> {
    use base64::Engine as _;
    BASE64_ENGINES
        .iter()
        .find_map(|engine| engine.decode(text).ok())
        .ok_or_else(|| ApiError::bad_request("`image` is not valid base64"))
}

/// Convert a swatch into its JSON form.
pub(super) fn swatch_info(swatch: &crate::preprocess::quantize::Swatch) -> SwatchInfo {
    SwatchInfo {
        color: swatch.color.to_hex(),
        pixels: swatch.population,
    }
}
