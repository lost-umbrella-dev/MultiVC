use clients::hash::Hash;
use clients::item::ItemDependence;
use serde::{Deserialize, Serialize};

/// Конфигурация одного инстанса (хранится в `instance.toml` внутри папки инстанса).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Instance {
    /// Описание инстанса
    pub description: Option<String>,
    /// Хэш ядра
    pub core_version: Hash,
    /// Зависимости (моды / контент-паки)
    pub dependencies: Vec<ItemDependence>,
}
