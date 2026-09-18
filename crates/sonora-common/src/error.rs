use thiserror::Error;

/// Core error types for the Sonora audio platform.
#[derive(Debug, Error)]
pub enum SonoraError {
    #[error("Audio subsystem error: {0}")]
    Audio(String),

    #[error("DSP processing error: {0}")]
    Dsp(String),

    #[error("Library error: {0}")]
    Library(String),

    #[error("Lyrics error: {0}")]
    Lyrics(String),

    #[error("Database error: {0}")]
    Database(String),

    #[error("Configuration error: {0}")]
    Config(String),

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Internal error: {0}")]
    Internal(String),
}

pub type Result<T> = std::result::Result<T, SonoraError>;
