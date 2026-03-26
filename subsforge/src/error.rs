use thiserror::Error;

#[derive(Error, Debug)]
pub enum SubsForgeError {
    #[error("ffmpeg failed: {0}")]
    Ffmpeg(String),

    #[error("whisper failed: {0}")]
    Whisper(String),

    #[error("translation failed: {0}")]
    Translation(String),

    #[error("SRT parse error: {0}")]
    SrtParse(String),

    #[error("file not found: {0}")]
    FileNotFound(String),

    #[error("path mapping failed: remote path {0} has no matching local mount")]
    PathMapping(String),

    #[error("database error: {0}")]
    Database(#[from] sqlx::Error),

    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

pub type Result<T> = std::result::Result<T, SubsForgeError>;
