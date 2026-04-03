use serde::{Deserialize, Serialize};

use crate::item::ItemDependence;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Instance {
    /// Описание инстенса
    pub deciption: Option<String>,
    /// Версия ядра
    pub core_version: String,
    /// Зависимости
    pub dependecies: Vec<ItemDependence>,
}
