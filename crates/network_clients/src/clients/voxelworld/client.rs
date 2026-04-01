//! Модуль клиента для взаимодействия с VoxelWorld API

pub mod modifications;

use crate::clients::USER_AGENT;
use reqwest::{
    Client as ReqwestClient,
    header::{ACCEPT, HeaderMap},
};
use serde::Deserialize;
use tracing::Span;

const BASE_URL: &str = "https://api.voxelworld.ru/v2";

/// Клиент для взаимодействия с VoxelWorld API
pub struct VoxelworldClient {
    /// HTTP клиент для выполнения запросов
    client: ReqwestClient,
    /// Базовый URL API
    base_url: &'static str,
    /// Опциональный токен авторизации
    auth_token: Option<String>,
    span: Span,
}

impl VoxelworldClient {
    /// Создает новый экземпляр клиента VoxelWorld API
    pub fn new(auth_token: Option<String>) -> Self {
        let mut headers = HeaderMap::new();
        // TODO: add error for json return's errors
        headers.insert(ACCEPT, "application/json".parse().unwrap());
        Self {
            client: ReqwestClient::builder()
                .user_agent(USER_AGENT)
                .connection_verbose(true)
                .https_only(true)
                .default_headers(headers)
                .build()
                .expect("Failed to create HTTP client"),
            base_url: BASE_URL,
            auth_token,
            span: tracing::info_span!("network_client", client = "voxelworld"),
        }
    }

    /// Устанавливает токен авторизации для клиента
    pub fn set_auth_token(&mut self, auth_token: String) {
        self.auth_token = Some(auth_token);
    }
}

/// Параметры для сортировки списка модов
#[derive(Debug, Clone, Copy, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum ModSort {
    #[default]
    /// Сортировка по популярности
    Popular,
    /// Сортировка по подпискам
    Subscribe,
    /// Сортировка по дате добавления
    DateAdd,
    /// Сортировка по дате обновления
    DateUpdate,
}
#[derive(Debug, Clone, Deserialize, Default)]
pub struct VoxelworldClientListOptions {
    /// Строка поиска (максимальная длина 255)
    pub title: Option<String>,
    /// Список тегов для фильтрации
    pub tags: Option<Vec<i64>>,
    /// Номер страницы (по умолчанию 1)
    pub page: Option<u32>,
    /// Параметр сортировки (по умолчанию "popular")
    pub sort: Option<ModSort>,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct VoxelworldClientGetOptions {
    #[serde(rename = "slugOrId")]
    pub search: String,
}
