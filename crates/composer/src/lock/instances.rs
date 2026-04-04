use std::path::Path;

use serde::{Deserialize, Serialize};
use tracing::Span;

use crate::item::LockMap;
use crate::lock::Lock;

#[derive(Debug, Clone, Serialize, Deserialize)]
/// Представляет метаданные инстанса
pub struct InstancesItem {
    /// Название инстенса
    pub name: String,
    /// Иконка (base64)
    pub icon: String,
    /// Баннер (base64)
    pub banner: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
/// Представляет метаданные всех инстансов
pub struct InstancesLock {
    #[serde(flatten)]
    /// Список инстансов
    pub items: LockMap,
}

impl Lock for InstancesLock {
    fn span() -> Span {
        tracing::info_span!("lock", type = "instances")
    }

    fn file_name() -> &'static Path {
        Path::new("instances/lock.toml")
    }

    fn folder_name() -> &'static Path {
        Path::new("instances")
    }

    fn items(&self) -> &LockMap {
        &self.items
    }
}
