use std::path::PathBuf;

use clients::hash::Hash;

use crate::lock::Lock;

/// Возвращает путь к элементу в хранилище lock-а по его хэшу.
///
/// Формат имени директории: `{algorithm}-{hex_digest}`.
pub fn item_path<L>(hash: &Hash) -> PathBuf
where
    L: Lock,
{
    L::folder_name().join(hash.to_path_buf())
}
