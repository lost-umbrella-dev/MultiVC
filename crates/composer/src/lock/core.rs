use serde::{Deserialize, Serialize};

use crate::item::LockMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
/// Представялет lock файл cores
pub struct CoresLock {
    #[serde(flatten)]
    pub items: LockMap,
}
