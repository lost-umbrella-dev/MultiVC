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
    let file_name = match hash {
        Hash::SHA256(value) => format!("sha256-{value}"),
        Hash::SHA512(value) => format!("sha512-{value}"),
    };

    L::folder_name().join(file_name)
}
