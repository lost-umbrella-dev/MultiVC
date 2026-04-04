use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
/// Представляет метаданные инстанса
pub struct InstancesItem {
    /// Название инстенса
    pub name: String,
    /// Иконка (base64)
    pub icon: String,
    /// Баннер (base64)
    pub banner: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
/// Представляет метаданные всех инстансов
pub struct InstancesLock {
    #[serde(flatten)]
    /// Список инстансов
    pub items: Vec<InstancesItem>,
}
