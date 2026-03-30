use crate::clients::voxelworld::client::VoxelworldClient;
use crate::clients::voxelworld::error::{Result, VoxelworldError};
use crate::clients::voxelworld::{DataResponse, ModDetail, ModListItem, VersionDetailResource, VersionResource};
use bytes::Bytes;
use serde::Deserialize;
use tracing::{debug, instrument};

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

/// Параметры запроса для получения списка модов
#[derive(Debug, Clone, Deserialize)]
pub struct ModsQueryParams {
    /// Строка поиска (максимальная длина 255)
    pub title: Option<String>,
    /// Список тегов для фильтрации
    pub tags: Option<Vec<i64>>,
    /// Номер страницы (по умолчанию 1)
    pub page: Option<u32>,
    /// Параметр сортировки (по умолчанию "popular")
    // save as u8
    pub sort: Option<ModSort>,
}

impl VoxelworldClient {
    #[instrument(
        name = "voxelworld.get_mods",
        level = "debug",
        parent = &self.span,
        skip(self, params),
        err,
        fields(
            mods.query = ?params.title,
            mods.page = ?params.page,
            mods.sort = ?params.sort,
            mods.tags_count = params.tags.as_ref().map(Vec::len),
            http.status_code
        )
    )]
    /// Получает список модов с возможностью фильтрации и сортировки
    ///
    /// # Аргументы
    /// * `params` - Параметры запроса для фильтрации и сортировки
    ///
    /// # Возвращает
    /// Список модов, соответствующих критериям поиска
    ///
    /// # Ошибки
    /// Возвращает `VoxelworldError` при ошибках сети или API
    ///
    /// # Пример
    /// ```ignore
    /// use voxelworld::client::{ModsQueryParams, ModSort, VoxelworldClient};
    ///
    /// let client = VoxelworldClient::new("https://api.voxelworld.ru/api".to_string());
    /// let params = ModsQueryParams {
    ///     query: Some("cool".to_string()),
    ///     page: Some(1),
    ///     sort: Some(ModSort::Downloads),
    ///     tags: Some(vec!["blocks".to_string()]),
    /// };
    ///
    /// let mods = client.get_mods(&params).await?;
    /// ```
    pub async fn get_mods(&self, params: &ModsQueryParams) -> Result<Vec<ModListItem>> {
        // Формируем URL для запроса списка модов
        let url = format!("{}/mods", self.base_url);

        // Создаем запрос с параметрами
        let mut request = self.client.get(&url);

        // Добавляем параметры запроса если они заданы
        if let Some(ref query) = params.title {
            request = request.query(&[("title", query)]);
        }

        if let Some(ref tags) = params.tags {
            for tag in tags {
                request = request.query(&[("tag_id[]", tag)]);
            }
        }

        if let Some(page) = params.page {
            request = request.query(&[("page", &page)]);
        }

        if let Some(sort) = params.sort {
            request = request.query(&[("sort", sort as u8 + 1)]);
        }

        // Добавляем токен авторизации если он есть
        if let Some(token) = &self.auth_token {
            request = request.bearer_auth(token);
        }

        // Выполняем запрос
        let request = request.build()?;

        debug!(
            method = %request.method(),
            url = %request.url(),
            "sending request"
        );

        let response = self.client.execute(request).await?;

        // Проверяем статус ответа
        if !response.status().is_success() {
            return Err(self.handle_error_response(response).await);
        }

        // Десериализуем JSON ответ
        let mods: DataResponse<Vec<ModListItem>> = response.json().await?;

        Ok(mods.data)
    }

    #[instrument(
        name = "voxelworld.get_mod",
        level = "debug",
        parent = &self.span,
        skip(self),
        err,
        fields(
            mod.id_or_slug = %id_or_slug,
            http.status_code
        )
    )]
    /// Получает детальную информацию о моде по ID или slug
    ///
    /// # Аргументы
    /// * `id_or_slug` - ID или slug мода
    ///
    /// # Возвращает
    /// Детальную информацию о моде
    ///
    /// # Ошибки
    /// Возвращает `VoxelworldError::NotFound` если мод не найден
    /// Возвращает `VoxelworldError` при других ошибках сети или API
    ///
    /// # Пример
    /// ```ignore
    /// let client = VoxelworldClient::new("https://api.voxelworld.ru/api".to_string());
    /// let mod_detail = client.get_mod_by_id_or_slug("cool-mod").await?;
    /// ```
    pub async fn get_mod_by_id_or_slug(&self, id_or_slug: &str) -> Result<ModDetail> {
        // Формируем URL для получения деталей мода
        let url = format!("{}/mods/{}", self.base_url, id_or_slug);

        // Создаем запрос
        let mut request = self.client.get(&url);

        // Добавляем токен авторизации если он есть
        if let Some(ref token) = self.auth_token {
            request = request.bearer_auth(token);
        }

        // Выполняем запрос
        let response = request.send().await?;

        // Проверяем статус ответа
        if response.status().as_u16() == 404 {
            return Err(VoxelworldError::not_found(format!("Mod: {}", id_or_slug)));
        }

        if !response.status().is_success() {
            return Err(self.handle_error_response(response).await);
        }

        // Десериализуем JSON ответ
        let mod_detail: ModDetail = response.json().await?;

        Ok(mod_detail)
    }

    #[instrument(
        name = "voxelworld.get_mod_versions",
        level = "debug",
        parent = &self.span,
        skip(self),
        err,
        fields(
            mod.id_or_slug = %mod_id_or_slug,
            page = ?page,
            limit = ?limit,
            version.status = ?status,
            http.status_code
        )
    )]
    /// Получает список версий мода
    ///
    /// # Аргументы
    /// * `mod_id_or_slug` - ID или slug мода
    /// * `page` - Номер страницы (по умолчанию 1)
    /// * `limit` - Количество элементов на странице (по умолчанию 10)
    /// * `status` - Фильтр по статусу версии (опционально)
    ///
    /// # Возвращает
    /// Список версий мода
    ///
    /// # Ошибки
    /// Возвращает `VoxelworldError::NotFound` если мод не найден
    /// Возвращает `VoxelworldError` при других ошибках сети или API
    ///
    /// # Пример
    /// ```ignore
    /// let versions = client.get_mod_versions("cool-mod", Some(1), Some(10), None).await?;
    /// ```
    pub async fn get_mod_versions(
        &self,
        mod_id_or_slug: &str,
        page: Option<u32>,
        limit: Option<u32>,
        status: Option<String>,
    ) -> Result<Vec<VersionResource>> {
        // Формируем URL для получения списка версий мода
        let url = format!("{}/mods/{}/versions", self.base_url, mod_id_or_slug);

        // Создаем запрос
        let mut request = self.client.get(&url);

        // Добавляем параметры запроса
        if let Some(p) = page {
            request = request.query(&[("page", &p)]);
        }

        if let Some(l) = limit {
            request = request.query(&[("limit", &l)]);
        }

        if let Some(s) = status {
            request = request.query(&[("status", &s)]);
        }

        // Добавляем токен авторизации если он есть
        if let Some(ref token) = self.auth_token {
            request = request.bearer_auth(token);
        }

        // Выполняем запрос
        let response = request.send().await?;

        // Проверяем статус ответа
        if response.status().as_u16() == 404 {
            return Err(VoxelworldError::not_found(format!("Mod versions: {}", mod_id_or_slug)));
        }

        if !response.status().is_success() {
            return Err(self.handle_error_response(response).await);
        }

        // Десериализуем JSON ответ
        let versions: DataResponse<Vec<VersionResource>> = response.json().await?;

        Ok(versions.data)
    }

    #[instrument(
        name = "voxelworld.get_mod_version",
        level = "debug",
        parent = &self.span,
        skip(self),
        err,
        fields(
            mod.id_or_slug = %mod_id_or_slug,
            version.id = version_id,
            http.status_code
        )
    )]
    /// Получает детальную информацию о версии мода
    ///
    /// # Аргументы
    /// * `mod_id_or_slug` - ID или slug мода
    /// * `version_id` - ID версии
    ///
    /// # Возвращает
    /// Детальную информацию о версии
    ///
    /// # Ошибки
    /// Возвращает `VoxelworldError::NotFound` если мод или версия не найдены
    /// Возвращает `VoxelworldError` при других ошибках сети или API
    ///
    /// # Пример
    /// ```ignore
    /// let version = client.get_mod_version("cool-mod", 42).await?;
    /// ```
    pub async fn get_mod_version(&self, mod_id_or_slug: &str, version_id: u64) -> Result<VersionDetailResource> {
        // Формируем URL для получения деталей версии
        let url = format!("{}/mods/{}/versions/{}", self.base_url, mod_id_or_slug, version_id);

        // Создаем запрос
        let mut request = self.client.get(&url);

        // Добавляем токен авторизации если он есть
        if let Some(ref token) = self.auth_token {
            request = request.bearer_auth(token);
        }

        // Выполняем запрос
        let response = request.send().await?;

        // Проверяем статус ответа
        if response.status().as_u16() == 404 {
            return Err(VoxelworldError::not_found(format!(
                "Version: {}/{}",
                mod_id_or_slug, version_id
            )));
        }

        if !response.status().is_success() {
            return Err(self.handle_error_response(response).await);
        }

        // Десериализуем JSON ответ
        let version: VersionDetailResource = response.json().await?;

        Ok(version)
    }

    #[instrument(
        name = "voxelworld.get_mod_latest_version",
        level = "debug",
        parent = &self.span,
        skip(self),
        err,
        fields(
            mod.id_or_slug = %mod_id_or_slug,
            http.status_code
        )
    )]
    /// Получает последнюю версию мода
    ///
    /// # Аргументы
    /// * `mod_id_or_slug` - ID или slug мода
    ///
    /// # Возвращает
    /// Детальную информацию о последней версии
    ///
    /// # Ошибки
    /// Возвращает `VoxelworldError::NotFound` если мод не найден или у него нет версий
    /// Возвращает `VoxelworldError` при других ошибках сети или API
    ///
    /// # Пример
    /// ```ignore
    /// let latest_version = client.get_mod_latest_version("cool-mod").await?;
    /// ```
    pub async fn get_mod_latest_version(&self, mod_id_or_slug: &str) -> Result<VersionDetailResource> {
        // Формируем URL для получения последней версии
        let url = format!("{}/mods/{}/versions/latest", self.base_url, mod_id_or_slug);

        // Создаем запрос
        let mut request = self.client.get(&url);

        // Добавляем токен авторизации если он есть
        if let Some(ref token) = self.auth_token {
            request = request.bearer_auth(token);
        }

        // Выполняем запрос
        let response = request.send().await?;

        // Проверяем статус ответа
        if response.status().as_u16() == 404 {
            return Err(VoxelworldError::not_found(format!(
                "Latest version: {}",
                mod_id_or_slug
            )));
        }

        if !response.status().is_success() {
            return Err(self.handle_error_response(response).await);
        }

        // Десериализуем JSON ответ
        let version: VersionDetailResource = response.json().await?;

        Ok(version)
    }

    #[instrument(
        name = "voxelworld.download_mod_version",
        level = "debug",
        parent = &self.span,
        skip(self),
        err,
        fields(
            mod.id = mod_id,
            version = %version,
            http.status_code
        )
    )]
    /// Получает URL для скачивания версии мода
    ///
    /// # Аргументы
    /// * `mod_id` - ID мода
    /// * `version` - Номер версии
    ///
    /// # Возвращает
    /// Содержимое файла версии в виде байтов
    ///
    /// # Ошибки
    /// Возвращает `VoxelworldError::NotFound` если мод или версия не найдены
    /// Возвращает `VoxelworldError` при других ошибках сети или API
    ///
    /// # Пример
    /// ```ignore
    /// let bytes = client.download_mod_version(123, "1.2.0").await?;
    /// // Сохранение в файл
    /// tokio::fs::write("mod.zip", bytes).await?;
    /// ```
    pub async fn download_mod_version(&self, mod_id: u64, version: &str) -> Result<Bytes> {
        // Формируем URL для скачивания версии
        let url = format!("{}/mods/{}/versions/{}/download", self.base_url, mod_id, version);

        // Создаем запрос
        let mut request = self.client.get(&url);

        // Добавляем токен авторизации если он есть
        if let Some(ref token) = self.auth_token {
            request = request.bearer_auth(token);
        }

        // Выполняем запрос
        let response = request.send().await?;

        // Проверяем статус ответа
        if response.status().as_u16() == 404 {
            return Err(VoxelworldError::not_found(format!("Download: {}/{}", mod_id, version)));
        }

        if !response.status().is_success() {
            return Err(self.handle_error_response(response).await);
        }

        // Получаем байты ответа
        let bytes = response.bytes().await?;

        Ok(bytes)
    }
}
