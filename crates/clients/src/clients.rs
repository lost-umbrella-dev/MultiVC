use digest::DynDigest;

use tokio::io::{AsyncWrite, AsyncWriteExt};
use tracing::instrument;

use crate::{
    clients::github::client::GithubClient,
    error::{ClientError, Result},
    item::Item,
};

pub mod github;
pub mod voxelworld;

use serde::{Deserialize, Serialize};
use strum::VariantArray;

pub static USER_AGENT: &str = "MultiVC/0.0 (discord@towinok)";

#[derive(Debug, Clone, Copy, PartialEq, VariantArray, Serialize, Deserialize)]
/// Перечисление вариантов клиентов
pub enum ClientVariant {
    Github,
}

/// Представляет набор клиентов для различных сервисов
pub struct Clients {
    pub core: GithubClient,
    // outhers next
}

impl Clients {
    pub fn new(core: GithubClient) -> Self {
        Self { core }
    }
}

#[allow(async_fn_in_trait)]
pub trait Client
where
    Self: ClientGeneral + ClientDownload + ClientValidation,
{
    type ListOptions;
    type GetOptions;

    /// Получить список доступных элементов
    async fn list(&self, options: Self::ListOptions) -> Result<Vec<Item>>;

    /// Получить элемент по фильтрам
    async fn get(&self, options: Self::GetOptions) -> Result<Option<Item>>;
}

#[derive(Debug, Clone, Default)]
pub struct DownloadProgress {
    pub downloaded: u64,
    pub total: Option<u64>,
}

impl DownloadProgress {
    pub fn fraction(&self) -> Option<f32> {
        self.total.map(|total| {
            if total == 0 {
                0.0
            } else {
                self.downloaded as f32 / total as f32
            }
        })
    }
}

pub trait ProgressSink: Send + Sync {
    fn update(&self, progress: DownloadProgress);
}

pub enum DownloadType {
    Zip,
}

pub trait ClientGeneral {
    fn span(&self) -> tracing::Span;
    fn client(&self) -> reqwest::Client;
    fn download_type(&self) -> DownloadType;
    fn variant(&self) -> ClientVariant;
}

pub trait ClientValidation
where
    Self: ClientGeneral,
{
    #[instrument(
        level = "debug",
        parent = &self.span(),
        skip(self),
    )]
    /// Начало валидации [Item]
    fn validate_begin(&self, item: &Item) -> Option<Box<dyn DynDigest>> {
        item.hash.as_ref().map(|h| h.hasher())
    }

    #[instrument(
        level = "debug",
        parent = &self.span(),
        skip(self, hasher),
    )]
    /// Фиксация [Item]
    fn validate_finish(&self, item: &Item, hasher: Option<Box<dyn DynDigest>>) -> bool {
        match (hasher, &item.hash) {
            (Some(h), Some(hash)) => hash.verify_digest(&h.finalize()),
            (None, None) => true,
            _ => false,
        }
    }
}

#[allow(async_fn_in_trait)]
pub trait ClientDownload
where
    Self: ClientGeneral + ClientValidation,
{
    #[instrument(
        level = "debug",
        parent = &self.span(),
        skip(self, writer, progress),
        err,
    )]
    /// Скачать элемент в writer
    async fn download<W>(&self, item: &Item, writer: &mut W, progress: Option<&dyn ProgressSink>) -> Result<()>
    where
        W: AsyncWrite + Unpin + Send,
    {
        let client = self.client();

        let link = &(item).url;

        let mut hasher = self.validate_begin(item);

        let mut response = client.get(link).send().await?;
        let total = response.content_length();

        let mut downloaded = 0;

        while let Some(chunk) = response.chunk().await? {
            if let Some(h) = &mut hasher {
                h.update(&chunk);
            }
            writer.write_all(&chunk).await?;
            downloaded += chunk.len() as u64;
            if let Some(progress) = progress {
                progress.update(DownloadProgress { downloaded, total });
            }
        }
        if !self.validate_finish(item, hasher) {
            return Err(ClientError::HashMismatch(item.hash.clone().unwrap()));
        }

        Ok(())
    }
}
