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
use crate::utils::fs;

use super::executable;

/// Pipeline установки одного ядра.
///
/// Полный цикл зависит от платформы:
/// - **Windows**: скачивание zip → распаковка → поиск `VoxelCore.exe` → переименование в `core.exe` → хэширование → commit.
/// - **Linux / macOS**: скачивание файла напрямую → переименование в `core.{ext}` → хэширование → commit.
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
/// Платформозависимое поведение:
///
/// - **Windows**: скачивает zip-архив, распаковывает, ищет `VoxelCore.exe`,
///   переименовывает в `core.exe`, хэширует директорию, коммитит.
///
/// - **Linux / macOS**: скачивает файл напрямую (`.AppImage` / `.dmg`),
///   переименовывает в `core.{ext}`, хэширует директорию, коммитит.
async fn prepare_inner(client: &GithubClient, item: &Item, progress: Option<&dyn ProgressSink>) -> Result<Hash> {
    // 1. Staging directory
    tracing::debug!("creating staging directory");
    let folder = CoresLock::folder_name().to_path_buf();
    tokio::fs::create_dir_all(&folder).await?;
    let temp_dir = tokio::task::spawn_blocking(move || TempDir::new_in(folder))
        .await
        .map_err(|e| ComposerError::Io(std::io::Error::other(e)))??;

    let content_dir = temp_dir.path().join("content");
    tokio::fs::create_dir_all(&content_dir).await?;

    // 2–4. Платформозависимая часть: скачивание + подготовка содержимого
    //
    // После этого блока в `content_dir` лежит готовое содержимое
    // с переименованным исполняемым файлом (`core.exe` / `core.AppImage` / `core.dmg`).

    #[cfg(target_os = "windows")]
    {
        use crate::utils::archive;

        let archive_path = temp_dir.path().join("archive.zip");

        // 2. Download archive
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
        let extract_for_extract = content_dir.clone();
        tokio::task::spawn_blocking(move || archive::extract_zip(&archive_for_extract, &extract_for_extract))
            .await
            .map_err(|e| ComposerError::Io(std::io::Error::other(e)))??;

        // 4. Rename VoxelCore.exe → core.exe
        tracing::debug!("renaming VoxelCore.exe → core.exe");
        executable::rename(&content_dir, std::path::Path::new("")).await?;
    }

    #[cfg(not(target_os = "windows"))]
    {
        // 2. Download file directly
        let original_name = item.url.rsplit('/').next().unwrap_or("downloaded_core");
        let download_path = content_dir.join(original_name);

        let mut file = tokio::fs::File::create(&download_path).await?;
        tracing::debug!(url = %item.url, "downloading file (direct)");
        clients::download(&client.client, item, &mut file, progress)
            .instrument(client.span())
            .await?;
        file.flush().await?;
        drop(file);

        // 3. Rename → core.{ext}
        tracing::debug!("renaming → {}", executable::name());
        executable::rename(&content_dir, &download_path).await?;
    }

    // 5. Hash (after rename — хэш включает каноническое имя)
    tracing::debug!("computing directory hash");
    let hash_dir = content_dir.clone();
    let dir_hash = tokio::task::spawn_blocking(move || fs::compute_directory_hash(&hash_dir))
        .await
        .map_err(|e| ComposerError::Io(std::io::Error::other(e)))??;

    // 6. Commit
    tracing::debug!(hash = %dir_hash, "committing to storage");
    commit_extracted_dir::<CoresLock>(&content_dir, &dir_hash).await?;

    Ok(dir_hash)
}
