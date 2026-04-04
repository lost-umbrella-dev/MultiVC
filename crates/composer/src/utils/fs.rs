use std::{
    io::Read,
    path::{Path, PathBuf},
};

use clients::hash::Hash;
use digest::DynDigest;
use sha2::{Digest as ShaDigest, Sha256};

use crate::{error::ValidationError, lock::Lock};

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

/// Вычисляет хеш директории и сравнивает с заданным
pub fn hash_directory(path: &Path, hash: &Hash) -> std::result::Result<bool, ValidationError> {
    let mut hasher = hash.hasher();
    hash_directory_into(path, path, hasher.as_mut())?;
    Ok(hash.verify_digest(&hasher.finalize()))
}

pub fn hash_directory_into(
    root: &Path,
    current: &Path,
    hasher: &mut dyn DynDigest,
) -> std::result::Result<(), ValidationError> {
    let mut entries = std::fs::read_dir(current)
        .map_err(|source| ValidationError::Read {
            path: current.to_path_buf(),
            source,
        })?
        .collect::<std::result::Result<Vec<_>, _>>()
        .map_err(|source| ValidationError::Read {
            path: current.to_path_buf(),
            source,
        })?;

    entries.sort_by(|a, b| a.path().cmp(&b.path()));

    for entry in entries {
        let path = entry.path();
        let relative = path.strip_prefix(root).map_err(|source| ValidationError::Read {
            path: path.clone(),
            source: std::io::Error::other(source),
        })?;

        let file_type = entry.file_type().map_err(|source| ValidationError::Read {
            path: path.clone(),
            source,
        })?;

        if file_type.is_dir() {
            hasher.update(b"dir:");
            hasher.update(normalize_relative_path(relative).as_bytes());
            hasher.update(&[0]);

            hash_directory_into(root, &path, hasher)?;
        } else if file_type.is_file() {
            hasher.update(b"file:");
            hasher.update(normalize_relative_path(relative).as_bytes());
            hasher.update(&[0]);

            let mut file = std::fs::File::open(&path).map_err(|source| ValidationError::Read {
                path: path.clone(),
                source,
            })?;

            let mut buffer = [0u8; 8192];
            loop {
                let read = file.read(&mut buffer).map_err(|source| ValidationError::Read {
                    path: path.clone(),
                    source,
                })?;
                if read == 0 {
                    break;
                }
                hasher.update(&buffer[..read]);
            }
        }
    }

    Ok(())
}

pub fn normalize_relative_path(path: &Path) -> String {
    path.components()
        .map(|component| component.as_os_str().to_string_lossy())
        .collect::<Vec<_>>()
        .join("/")
}

pub fn compute_directory_hash(path: &Path) -> std::io::Result<Hash> {
    let mut hasher = Sha256::new();
    hash_directory_into(path, path, &mut hasher).map_err(std::io::Error::other)?;
    Ok(Hash::SHA256(hex::encode(hasher.finalize())))
}
