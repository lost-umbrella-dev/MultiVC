use std::path::Path;

use chrono::Utc;
use clients::{
    hash::Hash,
    item::Item,
    prelude::{ClientDownload, ClientGeneral, ProgressSink},
};
use tempfile::TempDir;
use tokio::io::AsyncWriteExt;

use crate::{
    error::{ComposerError, Result},
    item::LockItem,
    lock::Lock,
    utils::{archive, fs, hash},
};

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

/// Скачивает и подготавливает один `Item` для последующего сохранения в `LockMap`.
///
/// Workflow:
/// 1. Создаёт staging-директорию внутри `Lock::folder_name()`.
/// 2. Скачивает архив во временный файл.
/// 3. Распаковывает архив в `extract/`.
/// 4. Вычисляет хэш распакованной директории.
/// 5. Переносит директорию в конечное хранилище lock-а.
///
/// Возвращает `(Hash, LockItem)` при успехе или `(Item, ComposerError)` при ошибке,
/// чтобы вызывающий код мог собрать список неуспешных элементов с причиной.
pub async fn download_item<L, C>(
    client: &C,
    request: DownloadRequest,
) -> std::result::Result<(Hash, LockItem), (Item, ComposerError)>
where
    L: Lock,
    C: ClientDownload + ClientGeneral + Sync,
{
    let DownloadRequest { item, progress } = request;

    match download_item_inner::<L, C>(client, &item, progress.as_deref()).await {
        Ok(dir_hash) => Ok((
            dir_hash,
            LockItem {
                item,
                provider: client.variant(),
                timestamp: Utc::now(),
            },
        )),
        Err(error) => Err((item, error)),
    }
}

/// Внутренняя реализация скачивания одного элемента.
/// Возвращает хэш распакованной директории.
async fn download_item_inner<L, C>(client: &C, item: &Item, progress: Option<&dyn ProgressSink>) -> Result<Hash>
where
    L: Lock,
    C: ClientDownload + ClientGeneral + Sync,
{
    let folder = L::folder_name().to_path_buf();
    let temp_dir = tokio::task::spawn_blocking(move || TempDir::new_in(folder))
        .await
        .map_err(|e| ComposerError::Io(std::io::Error::other(e)))??;

    let archive_path = temp_dir.path().join("archive.zip");
    let extract_path = temp_dir.path().join("extract");

    tokio::fs::create_dir_all(&extract_path).await?;

    let mut archive_file = tokio::fs::File::create(&archive_path).await?;

    client.download(item, &mut archive_file, progress).await?;

    archive_file.flush().await?;
    drop(archive_file);

    let archive_for_extract = archive_path.clone();
    let extract_for_extract = extract_path.clone();
    tokio::task::spawn_blocking(move || archive::extract_zip(&archive_for_extract, &extract_for_extract))
        .await
        .map_err(|e| ComposerError::Io(std::io::Error::other(e)))??;

    let extract_for_hash = extract_path.clone();
    let dir_hash = tokio::task::spawn_blocking(move || fs::compute_directory_hash(&extract_for_hash))
        .await
        .map_err(|e| ComposerError::Io(std::io::Error::other(e)))??;

    commit_extracted_dir::<L>(&extract_path, &dir_hash).await?;

    Ok(dir_hash)
}

/// Коммитит распакованную staging-директорию в итоговое хранилище lock-а.
///
/// Если другая параллельная задача уже успела сохранить такую же директорию,
/// и она валидна по тому же hash, операция считается успешной.
pub async fn commit_extracted_dir<L>(extract_path: &Path, dir_hash: &Hash) -> Result<()>
where
    L: Lock,
{
    let final_path = hash::item_path::<L>(dir_hash);

    if tokio::fs::try_exists(&final_path).await? {
        let fp = final_path.clone();
        let dh = dir_hash.clone();

        let is_valid = tokio::task::spawn_blocking(move || fs::hash_directory(&fp, &dh))
            .await
            .map_err(|e| ComposerError::Io(std::io::Error::other(e)))??;

        if is_valid {
            return Ok(());
        }

        tokio::fs::remove_dir_all(&final_path).await?;
    }

    rename_or_accept_existing(extract_path, &final_path, dir_hash).await
}

async fn rename_or_accept_existing(extract_path: &Path, final_path: &Path, dir_hash: &Hash) -> Result<()> {
    if tokio::fs::rename(extract_path, final_path).await.is_ok() {
        return Ok(());
    }

    let fp = final_path.to_path_buf();
    let dh = dir_hash.clone();

    let is_valid = tokio::task::spawn_blocking(move || fs::hash_directory(&fp, &dh))
        .await
        .map_err(|e| ComposerError::Io(std::io::Error::other(e)))??;

    if is_valid {
        Ok(())
    } else {
        Err(ComposerError::Io(std::io::Error::new(
            std::io::ErrorKind::Other,
            format!("failed to commit directory to `{}`", final_path.display()),
        )))
    }
}
