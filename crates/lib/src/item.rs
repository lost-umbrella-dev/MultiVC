use serde::{Deserialize, Serialize};

use crate::hash::Hash;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Item {
    /// Name of the item
    pub name: String,
    /// Version, tag, etc...
    ///
    /// example version: "1.1.1",
    /// auto bump: "^1.1" => auto update for any "1.1.x" versions
    pub version: String,
    /// Download url
    pub url: String,
    /// Hash
    pub hash: Option<Hash>,
    /// Size in bytes
    pub size: u64,
    /// Dependencies of the item
    pub dependencies: Option<Vec<ItemDependence>>,
    /// Supported engine versions
    pub supported_engine: Option<Vec<String>>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ItemDependence {
    /// Name of the dependent item
    pub name: String,
    /// Version of the dependent item
    ///
    /// example version: "1.1.1",
    /// auto bump: "^1.1" => auto update for any "1.1.x" versions
    pub version: String,
}
