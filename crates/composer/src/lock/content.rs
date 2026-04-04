use serde::{Deserialize, Serialize};

use crate::item::LockMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
/// Представялет lock файл контент-паков
pub struct ContentsLock {
    #[serde(flatten)]
    pub items: LockMap,
}
