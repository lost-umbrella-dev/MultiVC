use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstanceMeta {
    /// Название инстенса
    pub name: String,
    /// Иконка (base64)
    pub icon: String,
    /// Баннер (base64)
    pub banner: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstancesMeta {
    #[serde(flatten)]
    /// Список инстансов
    pub instances: Vec<InstanceMeta>,
}
