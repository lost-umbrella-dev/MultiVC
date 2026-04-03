//! Типы данных для версий проектов

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Статус версии (например: release, beta, alpha)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VersionStatusResource {
    /// ID статуса
    pub id: i64,
    /// Название статуса (например, "release", "beta", "alpha")
    pub title: String,
}

/// Версия игрового движка, с которой совместима версия проекта
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EngineVersionResource {
    /// ID версии движка
    pub id: i64,
    /// Номер версии движка (например, "0.21")
    pub version_number: String,
}

/// Сокращённая информация о проекте в зависимости
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectShortInfo {
    /// ID проекта
    pub id: i64,
    /// Slug проекта
    pub slug: String,
    /// Название проекта
    pub title: String,
    /// Тип проекта (например, "mod", "texturepack", "world")
    #[serde(rename = "type")]
    pub type_: String,
}

/// Сокращённая информация о проекте в детальной версии
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectMinimalInfo {
    /// ID проекта
    pub id: i64,
    /// Название проекта
    pub title: String,
    /// Тип проекта (например, "mod", "texturepack", "world")
    #[serde(rename = "type")]
    pub type_: String,
}

/// Зависимость версии от другого проекта (версии)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VersionDependenceResource {
    /// ID зависимости
    pub id: i64,
    /// Информация о проекте-зависимости
    pub project: ProjectShortInfo,
    /// Номер версии
    pub version_number: String,
    /// Путь к файлу версии
    pub path: String,
    /// Список изменений
    pub changelog: String,
    /// Список совместимых версий движка
    pub engine: Vec<EngineVersionResource>,
}

/// Одна версия проекта
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VersionResource {
    /// ID версии
    pub id: i64,
    /// Статус версии
    pub status: VersionStatusResource,
    /// Номер версии (например, "1.2.0")
    pub version_number: String,
    /// Путь к файлу версии
    pub path: String,
    /// Список изменений
    pub changelog: String,
    /// Список совместимых версий движка
    pub engine: Vec<EngineVersionResource>,
    /// Дата создания версии
    pub created_at: DateTime<Utc>,
}

/// Полная информация о версии проекта
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VersionDetailResource {
    /// ID версии
    pub id: i64,
    /// Информация о проекте
    pub project: ProjectMinimalInfo,
    /// Статус версии
    pub status: VersionStatusResource,
    /// Номер версии (например, "1.2.3")
    pub version_number: String,
    /// Путь к файлу версии
    pub path: String,
    /// Краткий список изменений
    pub changelog: String,
    /// Подробный список изменений
    pub detail_changelog: String,
    /// Список зависимостей
    pub dependencies: Vec<VersionDependenceResource>,
    /// Список совместимых версий движка
    pub engine: Vec<EngineVersionResource>,
    /// Дата создания версии
    pub created_at: DateTime<Utc>,
}
