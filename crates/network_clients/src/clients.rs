use bytes::Bytes;
use tokio::io::AsyncWrite;

use crate::error::Result;

pub mod github;
pub mod voxelworld;

pub static USER_AGENT: &str = "MultiVC/0.0 (discord@towinok)";

#[allow(async_fn_in_trait)]
pub trait Client {
    type ListOptions;
    type GetOptions;
    type Item: Sync;

    /// Получить список доступных элементов
    async fn list(&self, options: Self::ListOptions) -> Result<Vec<Self::Item>>;

    /// Получить элемент по фильтрам
    async fn get(&self, options: Self::GetOptions) -> Result<Self::Item>;

    /// Скачать элемент в writer
    async fn download<W>(&self, item: &Self::Item, writer: &mut W, progress: Option<&dyn ProgressSink>) -> Result<()>
    where
        W: AsyncWrite + Unpin + Send;

    /// Проверка элемента на соответствие хэшу провайдера
    ///
    /// **note**: `Item` хранит хэш
    fn validate(&self, item: &Self::Item, data: Bytes) -> Result<bool>;
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
