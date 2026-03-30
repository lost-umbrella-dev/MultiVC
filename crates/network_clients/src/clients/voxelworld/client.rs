//! Модуль клиента для взаимодействия с VoxelWorld API

pub mod modifications;

use crate::clients::USER_AGENT;
use crate::clients::voxelworld::error::VoxelworldError;
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
    ///

    /// # Пример
    /// ```ignore
    /// use voxelworld::client::VoxelworldClient;
    ///
    /// let client = VoxelworldClient::new("https://api.voxelworld.ru/api".to_string());
    /// ```
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
    ///
    /// # Аргументы
    /// * `auth_token` - Токен авторизации для Bearer аутентификации
    ///
    /// # Пример
    /// ```ignore
    /// client.set_auth_token("new_token".to_string());
    /// ```
    pub fn set_auth_token(&mut self, auth_token: String) {
        self.auth_token = Some(auth_token);
    }

    /// Обрабатывает ошибочный ответ от API и преобразует его в VoxelworldError
    ///
    /// # Аргументы
    /// * `response` - HTTP ответ с ошибкой
    ///
    /// # Возвращает
    /// `VoxelworldError` соответствующий статусу ответа
    async fn handle_error_response(&self, response: reqwest::Response) -> VoxelworldError {
        let status = response.status();

        // Пытаемся получить тело ответа
        let body = match response.text().await {
            Ok(text) => text,
            Err(_) => return VoxelworldError::HttpError(status.to_string()),
        };

        // Преобразуем статус в соответствующую ошибку
        match status.as_u16() {
            400 => VoxelworldError::bad_request(body),
            401 => VoxelworldError::unauthorized(body),
            403 => VoxelworldError::forbidden(body),
            404 => VoxelworldError::not_found(body),
            500 => VoxelworldError::server_error(body),
            _ => VoxelworldError::api_error(body),
        }
    }
}
