use serde::{Deserialize, Serialize};
use tracing::Span;

use crate::item::LockMap;
use crate::lock::Lock;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
/// Представялет lock файл cores
pub struct CoresLock {
    #[serde(flatten)]
    pub items: LockMap,
}

impl Lock for CoresLock {
    fn span() -> Span {
        tracing::info_span!("lock", type = "cores")
    }

    fn file_name() -> &'static std::path::Path {
        std::path::Path::new("cores/lock.toml")
    }

    fn folder_name() -> &'static std::path::Path {
        std::path::Path::new("cores")
    }

    fn items(&self) -> &LockMap {
        &self.items
    }
}
