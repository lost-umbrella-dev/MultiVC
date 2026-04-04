use clients::hash::Hash;

use crate::{
    error::ValidationError,
    item::LockItem,
    lock::{Lock, ValidateReason},
};

use super::{fs, hash};

pub const PARALLELISM: usize = 32;

/// Валидирует одну локальную директорию по хэшу дерева из lock-файла.
pub async fn validate_dir_item<L>(hash_value: &Hash, item: &LockItem) -> Result<Option<ValidateReason>, ValidationError>
where
    L: Lock,
{
    let path = hash::item_path::<L>(hash_value);

    match tokio::fs::metadata(&path).await {
        Ok(metadata) => {
            if !metadata.is_dir() {
                return Ok(Some(ValidateReason::NotFound(hash_value.clone(), item.clone())));
            }

            let path_for_hash = path;
            let hash_for_check = hash_value.clone();
            let is_match = tokio::task::spawn_blocking(move || fs::hash_directory(&path_for_hash, &hash_for_check))
                .await
                .map_err(|e| ValidationError::Read {
                    path: hash::item_path::<L>(hash_value),
                    source: std::io::Error::other(e),
                })??;

            if is_match {
                Ok(None)
            } else {
                Ok(Some(ValidateReason::HashNotMatcher(hash_value.clone(), item.clone())))
            }
        },
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            Ok(Some(ValidateReason::NotFound(hash_value.clone(), item.clone())))
        },
        Err(error) => Err(ValidationError::Read { path, source: error }),
    }
}
