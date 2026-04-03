//! Общие типы данных для VoxelWorld API

use serde::{Deserialize, Serialize};

/// Информация о пользователе
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserResource {
    /// ID пользователя
    pub id: i64,
    /// Имя пользователя
    pub name: String,
    /// URL аватара или null, если не установлен
    #[serde(skip_serializing_if = "Option::is_none")]
    pub avatar: Option<String>,
}

/// Тег, связанный с контент паком
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TagResource {
    /// ID тега
    pub id: i64,
    /// Название тега
    pub title: String,
}

/// Ссылка, прикреплённая к проекту (например, GitHub, Discord и т.д.)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LinkResource {
    /// ID ссылки
    pub id: i64,
    /// Тип ссылки (например, github, discord)
    #[serde(rename = "type")]
    pub type_: String,
    /// URL ссылки
    pub url: String,
}
