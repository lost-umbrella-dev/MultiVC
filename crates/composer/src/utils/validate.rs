use std::path::Path;

use clients::hash::Hash;
use tracing::instrument;

use crate::{error::ValidationError, item::LockItem, lock::ValidateReason};

use super::{fs, hash};

pub const PARALLELISM: usize = 32;

/// Валидирует одну локальную директорию по хэшу дерева из lock-файла.
#[instrument(
    name = "lock.validate_item",
    level = "debug",
    skip(item, base_dir),
    fields(
        item.name = %item.item.name,
        item.version = %item.item.version,
        hash = %hash_value,
    ),
)]
pub async fn validate_dir_item(
    base_dir: &Path,
    hash_value: &Hash,
    item: &LockItem,
) -> Result<Option<ValidateReason>, ValidationError> {
    let path = hash::item_path(base_dir, hash_value);

    match tokio::fs::metadata(&path).await {
        Ok(metadata) => {
            if !metadata.is_dir() {
                tracing::warn!(path = %path.display(), "expected directory, found file — treating as not found");
                return Ok(Some(ValidateReason::NotFound(hash_value.clone(), item.clone())));
            }

            let path_for_hash = path;
            let hash_for_check = hash_value.clone();
            let bd = base_dir.to_path_buf();
            let hv = hash_value.clone();
            let is_match = tokio::task::spawn_blocking(move || {
                fs::hash_directory(&path_for_hash, &hash_for_check)
            })
            .await
            .map_err(|e| ValidationError::Read {
                path: hash::item_path(&bd, &hv),
                source: std::io::Error::other(e),
            })??;

            if is_match {
                tracing::debug!("hash matched");
                Ok(None)
            } else {
                tracing::warn!("hash mismatch");
                Ok(Some(ValidateReason::HashNotMatcher(hash_value.clone(), item.clone())))
            }
        },
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            tracing::debug!(path = %path.display(), "directory not found");
            Ok(Some(ValidateReason::NotFound(hash_value.clone(), item.clone())))
        },
        Err(error) => {
            tracing::error!(path = %path.display(), %error, "failed to read metadata");
            Err(ValidationError::Read {
                path,
                source: error,
            })
        },
    }
}
