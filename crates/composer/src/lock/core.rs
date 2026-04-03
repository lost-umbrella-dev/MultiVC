use clients::item::ItemLock;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoreLock {
    #[serde(flatten)]
    pub items: Vec<ItemLock>,
}
