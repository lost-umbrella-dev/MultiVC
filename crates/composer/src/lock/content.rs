use serde::{Deserialize, Serialize};

use crate::item::LockMap;
use crate::lock::Lock;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
/// Представялет lock файл контент-паков
pub struct ContentsLock {
    #[serde(flatten)]
    pub items: LockMap,
}

impl Lock for ContentsLock {
    fn span() -> tracing::Span {
        tracing::info_span!("lock", type = "contents")
    }

    fn file_name() -> &'static std::path::Path {
        std::path::Path::new("contents/lock.toml")
    }

    fn folder_name() -> &'static std::path::Path {
        std::path::Path::new("contents")
    }

    fn items(&self) -> &LockMap {
        &self.items
    }
}
