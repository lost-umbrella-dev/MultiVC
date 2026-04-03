use serde::{Deserialize, Serialize};

use crate::item::ItemLock;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContentLock {
    #[serde(flatten)]
    pub items: Vec<ItemLock>,
}
