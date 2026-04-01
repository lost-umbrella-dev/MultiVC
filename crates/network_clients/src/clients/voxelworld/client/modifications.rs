use crate::clients::voxelworld::client::{VoxelworldClient, VoxelworldClientListOptions};
use crate::clients::voxelworld::{DataResponse, ModDetail, ModListItem, VersionDetailResource, VersionResource};
use crate::error::{Result, response_error};
use bytes::Bytes;
use tracing::instrument;

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
    pub async fn get_mods(&self, params: &VoxelworldClientListOptions) -> Result<Vec<ModListItem>> {
        let url = format!("{}/mods", self.base_url);

        let mut request = self.client.get(&url);

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

        if let Some(token) = &self.auth_token {
            request = request.bearer_auth(token);
        }

        Ok(serde_json::from_str::<DataResponse<Vec<ModListItem>>>(
            &response_error(request.send().await?)?.text().await?,
        )
        .map(|x| x.data)?)
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
    pub async fn get_mod_by_id_or_slug(&self, id_or_slug: &str) -> Result<ModDetail> {
        let url = format!("{}/mods/{}", self.base_url, id_or_slug);

        let mut request = self.client.get(&url);

        if let Some(ref token) = self.auth_token {
            request = request.bearer_auth(token);
        }

        Ok(serde_json::from_str::<ModDetail>(
            &response_error(request.send().await?)?.text().await?,
        )?)
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
    pub async fn get_mod_versions(
        &self,
        mod_id_or_slug: &str,
        page: Option<u32>,
        limit: Option<u32>,
        status: Option<String>,
    ) -> Result<Vec<VersionResource>> {
        let url = format!("{}/mods/{}/versions", self.base_url, mod_id_or_slug);

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

        if let Some(ref token) = self.auth_token {
            request = request.bearer_auth(token);
        }

        Ok(serde_json::from_str::<DataResponse<Vec<VersionResource>>>(
            &response_error(request.send().await?)?.text().await?,
        )
        .map(|x| x.data)?)
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
    pub async fn get_mod_version(&self, mod_id_or_slug: &str, version_id: u64) -> Result<VersionDetailResource> {
        let url = format!("{}/mods/{}/versions/{}", self.base_url, mod_id_or_slug, version_id);

        let mut request = self.client.get(&url);

        if let Some(ref token) = self.auth_token {
            request = request.bearer_auth(token);
        }

        Ok(serde_json::from_str::<VersionDetailResource>(
            &response_error(request.send().await?)?.text().await?,
        )?)
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
    pub async fn get_mod_latest_version(&self, mod_id_or_slug: &str) -> Result<VersionDetailResource> {
        let url = format!("{}/mods/{}/versions/latest", self.base_url, mod_id_or_slug);

        let mut request = self.client.get(&url);

        if let Some(ref token) = self.auth_token {
            request = request.bearer_auth(token);
        }

        Ok(serde_json::from_str::<VersionDetailResource>(
            &response_error(request.send().await?)?.text().await?,
        )?)
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
    pub async fn download_mod_version(&self, mod_id: u64, version: &str) -> Result<Bytes> {
        let url = format!("{}/mods/{}/versions/{}/download", self.base_url, mod_id, version);

        let mut request = self.client.get(&url);

        if let Some(ref token) = self.auth_token {
            request = request.bearer_auth(token);
        }

        let bytes = response_error(request.send().await?)?.bytes().await?;

        Ok(bytes)
    }
}
