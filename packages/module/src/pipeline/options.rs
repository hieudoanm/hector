//! Map CLI flags onto a validated `TraceConfig`.

use crate::cli::args::{Cli, Detail};
use crate::core::color::Rgba;
use crate::core::error::{Error, Result};
use crate::pipeline::config::TraceConfig;

/// Areas below which `Detail::Balanced` and `Detail::Bold` drop a path.
const BALANCED_MIN_AREA: usize = 12;
const BOLD_MIN_AREA: usize = 96;

/// Parse `#rgb`, `#rrggbb`, `#rrggbbaa`, or a bare hex string.
pub fn parse_color(text: &str) -> Option<Rgba> {
    let hex = text.trim().trim_start_matches('#');
    if !hex.chars().all(|c| c.is_ascii_hexdigit()) {
        return None;
    }
    match hex.len() {
        3 | 4 => expand_short(hex),
        6 | 8 => expand_long(hex),
        _ => None,
    }
}

/// Expand `#rgb` or `#rgba` by doubling each digit.
fn expand_short(hex: &str) -> Option<Rgba> {
    let mut digits = hex.chars().map(|c| c.to_digit(16).map(|v| (v * 17) as u8));
    let mut next = || digits.next().flatten();
    Some(Rgba::new(next()?, next()?, next()?, next().unwrap_or(255)))
}

/// Expand `#rrggbb` or `#rrggbbaa` by parsing byte pairs.
fn expand_long(hex: &str) -> Option<Rgba> {
    let pairs: Vec<u8> = (0..hex.len() / 2)
        .map(|i| u8::from_str_radix(&hex[i * 2..i * 2 + 2], 16).ok())
        .collect::<Option<Vec<u8>>>()?;
    Some(Rgba::new(
        pairs[0],
        pairs[1],
        pairs[2],
        pairs.get(3).copied().unwrap_or(255),
    ))
}

/// Minimum region area implied by the detail preset.
fn min_area_for(detail: Detail, requested: usize) -> usize {
    let floor = match detail {
        Detail::Full => 0,
        Detail::Balanced => BALANCED_MIN_AREA,
        Detail::Bold => BOLD_MIN_AREA,
    };
    floor.max(requested)
}

/// Source of the tracing flags, shared by the top-level command and `info`.
pub struct TracingArgs<'a> {
    pub colors: usize,
    pub threshold: u8,
    pub simplify_tolerance: f64,
    pub bezier_tolerance: f64,
    pub min_area: usize,
    pub backdrop: &'a str,
    pub background_index: usize,
    pub detail: Detail,
    pub precision: u32,
}

impl TracingArgs<'_> {
    /// Translate the flags into a validated configuration.
    pub fn to_config(&self) -> Result<TraceConfig> {
        let backdrop = parse_color(self.backdrop).ok_or_else(|| {
            Error::Config(format!(
                "--backdrop `{}` is not a hex color like #ffffff",
                self.backdrop
            ))
        })?;
        let config = TraceConfig {
            colors: self.colors,
            threshold: self.threshold,
            simplify_tolerance: self.simplify_tolerance,
            bezier_tolerance: self.bezier_tolerance,
            min_area: min_area_for(self.detail, self.min_area),
            backdrop,
            background_index: self.background_index,
            ..TraceConfig::default()
        };
        config.validate()?;
        Ok(config)
    }
}

impl<'a> TracingArgs<'a> {
    /// Same flags, viewed through a parsed command line.
    pub fn from_cli(cli: &'a Cli) -> Self {
        Self {
            colors: cli.colors,
            threshold: cli.threshold,
            simplify_tolerance: cli.simplify_tolerance,
            bezier_tolerance: cli.bezier_tolerance,
            min_area: cli.min_area,
            backdrop: &cli.backdrop,
            background_index: cli.background_index,
            detail: cli.detail,
            precision: cli.precision,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;

    fn config_from(args: &[&str]) -> TraceConfig {
        let cli = Cli::try_parse_from(args).expect("valid arguments");
        TracingArgs::from_cli(&cli)
            .to_config()
            .expect("valid config")
    }

    #[test]
    fn parses_six_digit_hex() {
        assert_eq!(parse_color("#ff8000"), Some(Rgba::new(255, 128, 0, 255)));
    }

    #[test]
    fn parses_short_hex() {
        assert_eq!(parse_color("#f80"), Some(Rgba::new(255, 136, 0, 255)));
    }

    #[test]
    fn parses_eight_digit_hex_with_alpha() {
        assert_eq!(parse_color("#00000080"), Some(Rgba::new(0, 0, 0, 128)));
    }

    #[test]
    fn rejects_non_hex_text() {
        assert_eq!(parse_color("rebeccapurple"), None);
        assert_eq!(parse_color("#12345"), None);
    }

    #[test]
    fn detail_full_keeps_default_min_area() {
        let config = config_from(&["vectify", "a.png", "--detail", "full"]);
        assert_eq!(config.min_area, TraceConfig::default().min_area);
    }

    #[test]
    fn detail_bold_raises_min_area() {
        let config = config_from(&["vectify", "a.png", "--detail", "bold"]);
        assert_eq!(config.min_area, BOLD_MIN_AREA);
    }

    #[test]
    fn explicit_min_area_wins_over_detail_floor() {
        let config = config_from(&["vectify", "a.png", "--detail", "bold", "--min-area", "500"]);
        assert_eq!(config.min_area, 500);
    }

    #[test]
    fn invalid_colors_is_reported() {
        let cli = Cli::try_parse_from(["vectify", "a.png", "--colors", "0"]).expect("valid");
        assert!(TracingArgs::from_cli(&cli).to_config().is_err());
    }

    #[test]
    fn backdrop_flag_is_parsed_into_config() {
        let config = config_from(&["vectify", "a.png", "--backdrop", "#101010"]);
        assert_eq!(config.backdrop, Rgba::rgb(16, 16, 16));
    }

    #[test]
    fn malformed_backdrop_is_rejected() {
        let cli = Cli::try_parse_from(["vectify", "a.png", "--backdrop", "nope"]).expect("valid");
        assert!(TracingArgs::from_cli(&cli).to_config().is_err());
    }
}
