//! Модуль ошибок для VoxelWorld API

use thiserror::Error;

/// Основной тип ошибки для VoxelWorld API
#[derive(Error, Debug)]
pub enum VoxelworldError {
    /// Неверный запрос (400)
    #[error("Bad request: {message}")]
    BadRequest { message: String },

    /// Не авторизован (401)
    #[error("Unauthorized: {message}")]
    Unauthorized { message: String },

    /// Доступ запрещен (403)
    #[error("Forbidden: {message}")]
    Forbidden { message: String },

    /// Ресурс не найден (404)
    #[error("Not found: {resource}")]
    NotFound { resource: String },

    /// Ошибка сервера (500)
    #[error("Server error: {message}")]
    ServerError { message: String },

    /// Ошибка десериализации JSON
    #[error("Failed to deserialize JSON: {0}")]
    DeserializationError(String),

    /// Ошибка сериализации JSON
    #[error("Failed to serialize JSON: {0}")]
    SerializationError(String),

    /// Ошибка HTTP запроса
    #[error("HTTP request failed: {0}")]
    HttpError(String),

    /// Ошибка сети
    #[error("Network error: {0}")]
    NetworkError(String),

    /// Недействительный или истёкший токен
    #[error("Invalid or expired token")]
    InvalidToken,

    /// Недостаточно прав (отсутствует scope)
    #[error("Insufficient scope")]
    InsufficientScope,

    /// Общая ошибка API
    #[error("API error: {message}")]
    ApiError { message: String },
}

impl VoxelworldError {
    /// Создает ошибку BadRequest с сообщением
    pub fn bad_request(message: impl Into<String>) -> Self {
        Self::BadRequest {
            message: message.into(),
        }
    }

    /// Создает ошибку Unauthorized с сообщением
    pub fn unauthorized(message: impl Into<String>) -> Self {
        Self::Unauthorized {
            message: message.into(),
        }
    }

    /// Создает ошибку Forbidden с сообщением
    pub fn forbidden(message: impl Into<String>) -> Self {
        Self::Forbidden {
            message: message.into(),
        }
    }

    /// Создает ошибку NotFound для ресурса
    pub fn not_found(resource: impl Into<String>) -> Self {
        Self::NotFound {
            resource: resource.into(),
        }
    }

    /// Создает ошибку ServerError с сообщением
    pub fn server_error(message: impl Into<String>) -> Self {
        Self::ServerError {
            message: message.into(),
        }
    }

    /// Создает общую ошибку API с сообщением
    pub fn api_error(message: impl Into<String>) -> Self {
        Self::ApiError {
            message: message.into(),
        }
    }
}

// Реализация From для serde_json ошибок
impl From<serde_json::Error> for VoxelworldError {
    fn from(err: serde_json::Error) -> Self {
        Self::DeserializationError(err.to_string())
    }
}

// Реализация From для reqwest ошибок (если используется)
impl From<reqwest::Error> for VoxelworldError {
    fn from(err: reqwest::Error) -> Self {
        if err.is_timeout() {
            Self::NetworkError("Request timeout".to_string())
        } else if err.is_connect() {
            Self::NetworkError("Connection failed".to_string())
        } else if err.is_request() {
            Self::HttpError(err.to_string())
        } else {
            Self::NetworkError(err.to_string())
        }
    }
}

// Реализация From для chrono ошибок (ParseError)
impl From<chrono::ParseError> for VoxelworldError {
    fn from(err: chrono::ParseError) -> Self {
        Self::DeserializationError(format!("Failed to parse date: {}", err))
    }
}

/// Тип результата для VoxelWorld API
pub type Result<T> = std::result::Result<T, VoxelworldError>;
