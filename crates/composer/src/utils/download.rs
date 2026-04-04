use chrono::Utc;
use clients::{
    hash::Hash,
    item::Item,
    prelude::{ClientDownload, ClientGeneral},
};
use tempfile::TempDir;
use tokio::io::AsyncWriteExt;

use crate::{
    item::LockItem,
    lock::Lock,
    utils::{archive, fs},
};

pub const PARALLELISM: usize = 8;

/// Скачивает и подготавливает один `Item` для последующего сохранения в `LockMap`.
///
/// Workflow:
/// 1. Создаёт staging-директорию внутри `Lock::folder_name()`.
/// 2. Скачивает архив во временный файл.
/// 3. Распаковывает архив в `extract/`.
/// 4. Вычисляет хэш распакованной директории.
/// 5. Переносит директорию в конечное хранилище lock-а.
///
/// При любой item-level ошибке возвращает исходный `Item`,
/// чтобы вызывающий код мог собрать список неуспешных элементов.
pub async fn download_item<L, C>(client: &C, item: Item) -> std::result::Result<(Hash, LockItem), Item>
where
    L: Lock,
    C: ClientDownload + ClientGeneral + Sync,
{
    let temp_dir = match TempDir::new_in(L::folder_name()) {
        Ok(temp_dir) => temp_dir,
        Err(_) => return Err(item),
    };

    let archive_path = temp_dir.path().join("archive.zip");
    let extract_path = temp_dir.path().join("extract");

    if tokio::fs::create_dir_all(&extract_path).await.is_err() {
        return Err(item);
    }

    let mut archive_file = match tokio::fs::File::create(&archive_path).await {
        Ok(file) => file,
        Err(_) => return Err(item),
    };

    if client.download(&item, &mut archive_file, None).await.is_err() {
        return Err(item);
    }

    if archive_file.flush().await.is_err() {
        return Err(item);
    }

    drop(archive_file);

    let archive_path_for_extract = archive_path.clone();
    let extract_path_for_extract = extract_path.clone();
    match tokio::task::spawn_blocking(move || {
        archive::extract_zip(&archive_path_for_extract, &extract_path_for_extract)
    })
    .await
    {
        Ok(Ok(())) => {},
        _ => return Err(item),
    }

    let extract_path_for_hash = extract_path.clone();
    let dir_hash = match tokio::task::spawn_blocking(move || fs::compute_directory_hash(&extract_path_for_hash)).await {
        Ok(Ok(hash)) => hash,
        _ => return Err(item),
    };

    if commit_extracted_dir::<L>(&extract_path, &dir_hash).await.is_err() {
        return Err(item);
    }

    Ok((
        dir_hash,
        LockItem {
            item,
            provider: client.variant(),
            timestamp: Utc::now(),
        },
    ))
}

/// Коммитит распакованную staging-директорию в итоговое хранилище lock-а.
///
/// Если другая параллельная задача уже успела сохранить такую же директорию,
/// и она валидна по тому же hash, операция считается успешной.
pub async fn commit_extracted_dir<L>(extract_path: &std::path::Path, dir_hash: &Hash) -> std::result::Result<(), ()>
where
    L: Lock,
{
    let final_path = fs::item_path::<L>(dir_hash);

    match tokio::fs::try_exists(&final_path).await {
        Ok(true) => {
            let final_path_for_check = final_path.clone();
            let dir_hash_for_check = dir_hash.clone();

            let is_valid = match tokio::task::spawn_blocking(move || {
                fs::hash_directory(&final_path_for_check, &dir_hash_for_check)
            })
            .await
            {
                Ok(Ok(valid)) => valid,
                _ => return Err(()),
            };

            if is_valid {
                return Ok(());
            }

            if tokio::fs::remove_dir_all(&final_path).await.is_err() {
                return Err(());
            }

            rename_or_accept_existing(extract_path, &final_path, dir_hash).await
        },
        Ok(false) => rename_or_accept_existing(extract_path, &final_path, dir_hash).await,
        Err(_) => Err(()),
    }
}

async fn rename_or_accept_existing(
    extract_path: &std::path::Path,
    final_path: &std::path::Path,
    dir_hash: &Hash,
) -> std::result::Result<(), ()> {
    if tokio::fs::rename(extract_path, final_path).await.is_ok() {
        return Ok(());
    }

    let final_path_for_check = final_path.to_path_buf();
    let dir_hash_for_check = dir_hash.clone();

    let moved_by_other_task = matches!(
        tokio::task::spawn_blocking(move || fs::hash_directory(&final_path_for_check, &dir_hash_for_check)).await,
        Ok(Ok(true))
    );

    if moved_by_other_task { Ok(()) } else { Err(()) }
}
