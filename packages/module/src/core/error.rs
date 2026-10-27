use thiserror::Error;

/// Errors surfaced by the tracing library.
#[derive(Debug, Error)]
pub enum Error {
    #[error("cannot open `{0}`: {1}")]
    Open(String, std::io::Error),

    #[error("cannot decode `{0}` as PNG or JPEG: {1}")]
    Decode(String, String),

    #[error("`{0}` is not a supported image format (expected PNG or JPEG)")]
    InvalidFormat(String),

    #[error("pixel buffer holds {actual} samples but the dimensions require {expected}")]
    InvalidBuffer { expected: usize, actual: usize },

    #[error("cannot write `{0}`: {1}")]
    Write(String, String),

    #[error("cannot render `{0}` as SVG: {1}")]
    Render(String, String),

    #[error("cannot compare `{0}` ({1}x{2}) with `{3}` ({4}x{5}): the sizes differ")]
    SizeMismatch(String, u32, u32, String, u32, u32),

    #[error("invalid configuration: {0}")]
    Config(String),
}

pub type Result<T> = std::result::Result<T, Error>;
