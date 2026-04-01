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

impl From<reqwest::Response> for ClientError {
    fn from(value: reqwest::Response) -> Self {
        match value.status() {
            StatusCode::NOT_FOUND => ClientError::NotFound,
            StatusCode::UNAUTHORIZED => ClientError::InvalidToken,
            StatusCode::FORBIDDEN => ClientError::InsufficientScope,
            status => ClientError::StatusCode(status),
        }
    }
}

/// Обрабатывает HTTP ответ и преобразует статус-коды в соответствующие ошибки
///
/// # Аргументы
/// * `response` - HTTP ответ для обработки
///
/// # Возвращает
/// * `Ok(response)` - если статус код 2xx
/// * `Err(ClientError)` - с соответствующим вариантом ошибки для других кодов
pub fn response_error(response: reqwest::Response) -> Result<reqwest::Response> {
    match response.status() {
        StatusCode::NOT_FOUND => Err(ClientError::NotFound),
        StatusCode::UNAUTHORIZED => Err(ClientError::InvalidToken),
        StatusCode::FORBIDDEN => Err(ClientError::InsufficientScope),
        status if status.is_success() => Ok(response),
        status => Err(ClientError::StatusCode(status)),
    }
}
