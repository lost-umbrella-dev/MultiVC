use chrono::{DateTime, Utc};
use clients::{
    hash::Hash,
    item::{CoreOrigin, Item},
};
use dashmap::DashMap;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LockItem {
    /// Meta
    pub item: Item,
    /// Когда был скачан/обновлён
    pub timestamp: DateTime<Utc>,
    /// Способ установки
    #[serde(default)]
    pub origin: CoreOrigin,
}
/// Хэш формируется при сохранении в fs
/// Нужен для поиска по fs
/// Хэш используется для создания имени папки
pub type LockMap = DashMap<Hash, LockItem>;
