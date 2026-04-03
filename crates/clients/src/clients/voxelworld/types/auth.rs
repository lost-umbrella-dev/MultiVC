//! Типы данных для аутентификации и авторизации

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Успешный ответ при генерации одноразового токена
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OneTimeTokenGenerateResponse {
    /// Сгенерированный одноразовый токен
    pub one_time_token: String,
    /// Дата и время истечения токена
    pub expires_at: DateTime<Utc>,
}

/// Запрос для проверки одноразового токена
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OneTimeTokenCheckRequest {
    /// Одноразовый токен для проверки
    pub one_time_token: String,
}

/// Успешный ответ при проверке токена - возвращает данные пользователя
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OneTimeTokenCheckResponse {
    /// ID пользователя
    pub id: i64,
    /// Имя пользователя
    pub name: String,
    /// URL аватара или null, если не установлен
    #[serde(skip_serializing_if = "Option::is_none")]
    pub avatar: Option<String>,
    /// Флаг подтверждения email
    pub email_verified: bool,
}

/// Ответ для /me endpoint - данные текущего пользователя
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MeResponse {
    /// ID пользователя
    pub id: i64,
    /// Имя пользователя
    pub name: String,
    /// URL аватара или null, если не установлен
    #[serde(skip_serializing_if = "Option::is_none")]
    pub avatar: Option<String>,
    /// Флаг подтверждения email
    pub email_verified: bool,
}

/// Общая структура ошибки
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorResponse {
    /// Описание ошибки
    pub error: String,
}

/// Ответ при ошибке 401 - не авторизован
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnauthorizedResponse {
    /// Сообщение об ошибке
    pub message: String,
}

/// Ответ при ошибке 403 - доступ запрещен
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ForbiddenResponse {
    /// Описание ошибки
    pub error: String,
}

/// Ответ при ошибке 400 - неверный запрос
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BadRequestResponse {
    /// Описание ошибки
    pub error: String,
}
