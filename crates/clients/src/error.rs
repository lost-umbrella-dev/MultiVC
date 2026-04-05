use reqwest::StatusCode;
use thiserror::Error;

use crate::hash::Hash;

#[derive(Error, Debug)]
pub enum ClientError {
    #[error(transparent)]
    Json(#[from] serde_json::Error),

    #[error(transparent)]
    Reqwest(#[from] reqwest::Error),

    #[error("Resource not found")]
    NotFound,

    #[error("Invalid or expired token")]
    InvalidToken,

    #[error("Insufficient scope")]
    InsufficientScope,

    #[error("HTTP Status code: {0}")]
    StatusCode(StatusCode),

    #[error("API error: {0}")]
    ApiError(String),

    #[error("Tokio error: {0}")]
    TokioIoError(#[from] tokio::io::Error),

    #[error("Hash mismatch, expected: {0:?}")]
    HashMismatch(Hash),
}

pub type Result<T> = std::result::Result<T, ClientError>;
