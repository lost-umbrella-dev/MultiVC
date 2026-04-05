use serde::{Deserialize, Serialize};

use crate::hash::Hash;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Item {
    /// Name of the item
    pub name: String,
    /// Version, tag, etc...
    ///
    /// example version: "1.1.1",
    /// auto bump: "^1.1" => auto update for any "1.1.x" versions
    pub version: String,
    /// Download url
    pub url: String,
    /// Ссылка на source zipball (GitHub).
    ///
    /// Используется на Linux/macOS для извлечения папки `res/` из исходников.
    pub zipball_url: Option<String>,
    /// Хэш при скачивании
    ///
    /// Нужен для:
    /// - валидации скачивания
    /// - поиска по репозиторию
    pub hash: Option<Hash>,
    /// Size in bytes
    pub size: u64,
    /// Dependencies of the item
    pub dependencies: Option<Vec<ItemDependence>>,
    /// Supported engine versions
    pub supported_engine: Option<Vec<String>>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ItemDependence {
    /// Name of the dependent item
    // TODO: change to hash
    pub name: String,
    /// Version of the dependent item
    ///
    /// example version: "1.1.1",
    /// auto bump: "^1.1" => auto update for any "1.1.x" versions
    pub version: String,
}
