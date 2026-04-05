use chrono::Utc;
use clients::ProgressSink;
use clients::github::GithubClient;
use clients::hash::Hash;
use clients::item::Item;
use tempfile::TempDir;
use tokio::io::AsyncWriteExt;
use tracing::Instrument;

use crate::downloads::{DownloadRequest, commit_extracted_dir};
use crate::error::{ComposerError, Result};
use crate::item::LockItem;
use crate::lock::Lock;
use crate::lock::core::CoresLock;
use crate::utils::{archive, fs};

use super::executable;

/// Pipeline установки одного ядра.
///
/// Полный цикл: скачивание архива → распаковка → переименование
/// исполняемого файла в `core.{ext}` → хэширование → commit в хранилище.
///
/// Возвращает `(Hash, LockItem)` при успехе или `(Item, ComposerError)` при ошибке.
pub async fn download_and_prepare(
    client: &GithubClient,
    request: DownloadRequest,
) -> std::result::Result<(Hash, LockItem), (Item, ComposerError)> {
    let DownloadRequest { item, progress } = request;

    let span = tracing::debug_span!(
        "install.core",
        item.name = %item.name,
        item.version = %item.version,
        item.size = item.size,
    );

    match prepare_inner(client, &item, progress.as_deref()).instrument(span).await {
        Ok(dir_hash) => {
            tracing::debug!(
                name = %item.name,
                version = %item.version,
                hash = %dir_hash,
                "core prepared",
            );
            Ok((
                dir_hash,
                LockItem {
                    item,
                    timestamp: Utc::now(),
                },
            ))
        },
        Err(error) => {
            tracing::warn!(
                name = %item.name,
                version = %item.version,
                error = %error,
                "core preparation failed",
            );
            Err((item, error))
        },
    }
}

/// Внутренняя реализация pipeline установки ядра.
///
/// Шаги:
/// 1. Создание staging-директории
/// 2. Скачивание архива
/// 3. Распаковка
/// 4. Переименование исполняемого файла → `core.{ext}`
/// 5. Хэширование (после переименования — хэш включает каноническое имя)
/// 6. Commit в хранилище lock-а
async fn prepare_inner(client: &GithubClient, item: &Item, progress: Option<&dyn ProgressSink>) -> Result<Hash> {
    // 1. Staging directory
    tracing::debug!("creating staging directory");
    let folder = CoresLock::folder_name().to_path_buf();
    let temp_dir = tokio::task::spawn_blocking(move || TempDir::new_in(folder))
        .await
        .map_err(|e| ComposerError::Io(std::io::Error::other(e)))??;

    let archive_path = temp_dir.path().join("archive.zip");
    let extract_path = temp_dir.path().join("extract");

    tokio::fs::create_dir_all(&extract_path).await?;

    // 2. Download
    let mut archive_file = tokio::fs::File::create(&archive_path).await?;
    tracing::debug!(url = %item.url, "downloading archive");
    clients::download(&client.client, item, &mut archive_file, progress)
        .instrument(client.span())
        .await?;
    archive_file.flush().await?;
    drop(archive_file);

    // 3. Extract
    tracing::debug!("extracting archive");
    let archive_for_extract = archive_path.clone();
    let extract_for_extract = extract_path.clone();
    tokio::task::spawn_blocking(move || archive::extract_zip(&archive_for_extract, &extract_for_extract))
        .await
        .map_err(|e| ComposerError::Io(std::io::Error::other(e)))??;

    // 4. Rename executable → core.{ext}
    tracing::debug!("renaming core executable");
    executable::rename(&extract_path).await?;

    // 5. Hash (after rename, so hash includes canonical name)
    tracing::debug!("computing directory hash");
    let extract_for_hash = extract_path.clone();
    let dir_hash = tokio::task::spawn_blocking(move || fs::compute_directory_hash(&extract_for_hash))
        .await
        .map_err(|e| ComposerError::Io(std::io::Error::other(e)))??;

    // 6. Commit
    tracing::debug!(hash = %dir_hash, "committing to storage");
    commit_extracted_dir::<CoresLock>(&extract_path, &dir_hash).await?;

    Ok(dir_hash)
}
