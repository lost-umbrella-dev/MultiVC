use std::path::Path;

use crate::error::{ArchiveError, ComposerError};

/// Распаковывает ZIP-архив в указанную директорию.
///
/// Использует `enclosed_name()`, чтобы игнорировать небезопасные пути
/// внутри архива, например содержащие попытки выйти за пределы целевой папки.
pub fn extract_zip(archive_path: &Path, extract_path: &Path) -> Result<(), ComposerError> {
    let do_extract = || -> Result<(), ArchiveError> {
        let file = std::fs::File::open(archive_path)?;
        let mut archive = zip::ZipArchive::new(file)?;

        std::fs::create_dir_all(extract_path)?;

        for index in 0..archive.len() {
            let mut entry = archive.by_index(index)?;

            let Some(name) = entry.enclosed_name() else {
                continue;
            };

            let output_path = extract_path.join(name);

            if entry.is_dir() {
                std::fs::create_dir_all(&output_path)?;
            } else {
                if let Some(parent) = output_path.parent() {
                    std::fs::create_dir_all(parent)?;
                }

                let mut output = std::fs::File::create(&output_path)?;
                std::io::copy(&mut entry, &mut output)?;
            }
        }

        Ok(())
    };

    do_extract().map_err(|source| ComposerError::Archive {
        path: archive_path.to_path_buf(),
        source,
    })
}
