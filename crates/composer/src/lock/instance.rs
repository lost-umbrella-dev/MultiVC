use clients::item::ItemDependence;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Instance {
    /// Описание инстенса
    pub deciption: Option<String>,
    /// Версия ядра
    pub core_version: String,
    /// Зависимости
    pub dependecies: Vec<ItemDependence>,
}
