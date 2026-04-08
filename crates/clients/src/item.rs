use serde::{Deserialize, Serialize};

use crate::{hash::Hash, version::Version};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Item {
    /// Name of the item
    pub name: String,
    /// Version, tag, etc...
    ///
    /// example version: "1.1.1",
    /// auto bump: "^1.1" => auto update for any "1.1.x" versions
    pub version: Version,
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ItemDependence {
    /// Name of the dependent item
    pub name: Hash,
    /// Version of the dependent item
    ///
    /// example version: "1.1.1",
    /// auto bump: "^1.1" => auto update for any "1.1.x" versions
    pub version: Version,
}

/// Origin of an installed core — downloaded release or built from source.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum CoreOrigin {
    #[default]
    Release,
    Build,
}

/// Whether prebuilt release downloads are available on this platform.
/// macOS releases don't work, so this is false on macOS.
pub const RELEASE_AVAILABLE: bool = cfg!(not(target_os = "macos"));

/// Whether building from source is available on this platform.
pub const BUILD_AVAILABLE: bool = true;
