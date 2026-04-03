use clients::item::ItemLock;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContentLock {
    #[serde(flatten)]
    pub items: Vec<ItemLock>,
}
