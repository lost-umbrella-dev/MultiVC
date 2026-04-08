use std::path::{Path, PathBuf};

use clients::hash::Hash;

/// Возвращает путь к элементу в хранилище по его хэшу.
///
/// Формат имени директории: `{algorithm}-{hex_digest}`.
pub fn item_path(
    base_dir: &Path,
    hash: &Hash,
) -> PathBuf {
    base_dir.join(hash.to_path_buf())
}
