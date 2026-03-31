//! Модуль клиента для взаимодействия с VoxelWorld API

pub mod modifications;

use crate::clients::USER_AGENT;
use reqwest::Client;
use tracing::Span;

const BASE_URL: &str = "https://api.voxelworld.ru/v2";

/// Клиент для взаимодействия с VoxelWorld API
pub struct VoxelworldClient {
    /// HTTP клиент для выполнения запросов
    client: Client,
    /// Базовый URL API
    base_url: &'static str,
    /// Опциональный токен авторизации
    auth_token: Option<String>,
    span: Span,
}

impl VoxelworldClient {
    /// Создает новый экземпляр клиента VoxelWorld API
    pub fn new(auth_token: Option<String>) -> Self {
        Self {
            client: Client::builder()
                .user_agent(USER_AGENT)
                .connection_verbose(true)
                .https_only(true)
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
