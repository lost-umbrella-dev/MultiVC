use clients::hash::Hash;

use crate::{
    error::ValidationError,
    item::LockItem,
    lock::{Lock, ValidateReason},
};

use super::fs;

pub const PARALLELISM: usize = 32;

/// Валидирует одну локальную директорию по хэшу дерева из lock-файла.
pub async fn validate_dir_item<L>(
    hash: &Hash,
    item: &LockItem,
) -> std::result::Result<Option<ValidateReason>, ValidationError>
where
    L: Lock,
{
    let path = fs::item_path::<L>(hash);

    match tokio::fs::metadata(&path).await {
        Ok(metadata) => {
            if !metadata.is_dir() {
                return Ok(Some(ValidateReason::NotFound(hash.clone(), item.clone())));
            }

            if fs::hash_directory(&path, hash)? {
                Ok(None)
            } else {
                Ok(Some(ValidateReason::HashNotMatcher(hash.clone(), item.clone())))
            }
        },
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            Ok(Some(ValidateReason::NotFound(hash.clone(), item.clone())))
        },
        Err(error) => Err(ValidationError::Read { path, source: error }),
    }
}
