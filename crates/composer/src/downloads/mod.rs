//! Модули установки: скачивание, подготовка и регистрация элементов.
//!
//! Каждый подмодуль реализует специфичный pipeline для своего типа lock-а.

use std::path::Path;

use clients::{ProgressSink, hash::Hash, item::Item};
use tracing::instrument;

use crate::{
    error::{ComposerError, Result},
    lock::Lock,
    utils::{fs, hash},
};

pub mod core;
pub const PARALLELISM: usize = 8;

/// Пара Item + per-item progress для параллельного скачивания.
pub struct DownloadRequest {
    pub item: Item,
    pub progress: Option<Box<dyn ProgressSink>>,
}

impl DownloadRequest {
    /// Создаёт запрос без отслеживания прогресса.
    pub fn new(item: Item) -> Self {
        Self { item, progress: None }
    }

    /// Создаёт запрос с per-item прогрессом.
    pub fn with_progress(item: Item, progress: Box<dyn ProgressSink>) -> Self {
        Self {
            item,
            progress: Some(progress),
        }
    }
}

/// Коммитит распакованную staging-директорию в итоговое хранилище lock-а.
///
/// Если другая параллельная задача уже успела сохранить такую же директорию,
/// и она валидна по тому же hash, операция считается успешной.
#[instrument(
    name = "download.commit",
    level = "debug",
    skip(extract_path, dir_hash),
    fields(
        hash = %dir_hash,
        final_path,
    ),
)]
pub async fn commit_extracted_dir<L>(extract_path: &Path, dir_hash: &Hash) -> Result<()>
where
    L: Lock,
{
    let final_path = hash::item_path::<L>(dir_hash);
    tracing::Span::current().record("final_path", final_path.display().to_string());

    if tokio::fs::try_exists(&final_path).await? {
        tracing::debug!("target already exists, verifying hash");

        let fp = final_path.clone();
        let dh = dir_hash.clone();

        let is_valid = tokio::task::spawn_blocking(move || fs::hash_directory(&fp, &dh))
            .await
            .map_err(|e| ComposerError::Io(std::io::Error::other(e)))??;

        if is_valid {
            tracing::debug!("existing directory is valid, skipping");
            return Ok(());
        }

        tracing::warn!("existing directory hash mismatch, replacing");
        tokio::fs::remove_dir_all(&final_path).await?;
    }

    rename_or_accept_existing(extract_path, &final_path, dir_hash).await
}

/// Перемещает staging-директорию в итоговый путь, либо принимает результат параллельной задачи.
///
/// Пытается выполнить `rename`. Если rename не удался (другая задача уже создала директорию),
/// проверяет хэш существующей директории. Если хэш совпадает — операция считается успешной.
async fn rename_or_accept_existing(extract_path: &Path, final_path: &Path, dir_hash: &Hash) -> Result<()> {
    if tokio::fs::rename(extract_path, final_path).await.is_ok() {
        tracing::debug!(path = %final_path.display(), "directory committed via rename");
        return Ok(());
    }

    tracing::debug!("rename failed, checking if another task committed");

    let fp = final_path.to_path_buf();
    let dh = dir_hash.clone();

    let is_valid = tokio::task::spawn_blocking(move || fs::hash_directory(&fp, &dh))
        .await
        .map_err(|e| ComposerError::Io(std::io::Error::other(e)))??;

    if is_valid {
        tracing::debug!("directory already committed by another task");
        Ok(())
    } else {
        tracing::error!(path = %final_path.display(), "failed to commit directory");
        Err(ComposerError::Io(std::io::Error::other(format!(
            "failed to commit directory to `{}`",
            final_path.display()
        ))))
    }
}
