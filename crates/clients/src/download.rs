use digest::DynDigest;
use tokio::io::{AsyncWrite, AsyncWriteExt};

use crate::error::{ClientError, Result};
use crate::item::Item;

#[derive(Debug, Clone, Copy, Default)]
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

/// Начало валидации хэша для Item
pub fn validate_begin(item: &Item) -> Option<Box<dyn DynDigest>> {
    item.hash.as_ref().map(|h| h.hasher())
}

/// Финализация валидации хэша для Item
pub fn validate_finish(item: &Item, hasher: Option<Box<dyn DynDigest>>) -> bool {
    match (hasher, &item.hash) {
        (Some(h), Some(hash)) => hash.verify_digest(&h.finalize()),
        (None, None) => true,
        _ => false,
    }
}

/// Скачать item в writer с валидацией хэша
pub async fn download<W>(
    client: &reqwest::Client,
    item: &Item,
    writer: &mut W,
    progress: Option<&dyn ProgressSink>,
) -> Result<()>
where
    W: AsyncWrite + Unpin + Send,
{
    let mut hasher = validate_begin(item);

    let mut response = client.get(&item.url).send().await?;
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

    if !validate_finish(item, hasher) {
        return Err(ClientError::HashMismatch(item.hash.clone().unwrap()));
    }

    Ok(())
}
