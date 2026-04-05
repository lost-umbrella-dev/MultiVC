use std::path::{Component, Path};

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

/// Извлекает только папку `res/` из GitHub source zipball.
///
/// GitHub zipball содержит корневую папку `{repo}-{sha}/`, поэтому
/// `res/` лежит на втором уровне: `{repo}-{sha}/res/...`.
///
/// Результат извлекается в `extract_path/res/...` (prefix обрезается).
///
/// Возвращает `Ok(true)` если `res/` была найдена и извлечена,
/// `Ok(false)` если `res/` отсутствует в архиве.
pub fn extract_res_from_zip(archive_path: &Path, extract_path: &Path) -> Result<bool, ComposerError> {
    let do_extract = || -> Result<bool, ArchiveError> {
        let file = std::fs::File::open(archive_path)?;
        let mut archive = zip::ZipArchive::new(file)?;

        let mut found = false;

        for index in 0..archive.len() {
            let mut entry = archive.by_index(index)?;

            let Some(name) = entry.enclosed_name() else {
                continue;
            };

            // Collect path components: first is the root folder (e.g. "RepoName-sha"),
            // second should be "res" for entries we care about.
            let components: Vec<Component<'_>> = name.components().collect();

            if components.len() < 2 {
                continue;
            }

            // Check that the second component is "res"
            let Component::Normal(second) = &components[1] else {
                continue;
            };
            if *second != "res" {
                continue;
            }

            // Strip the first component (root folder), keep everything from "res/" onward
            let relative: std::path::PathBuf = components[1..].iter().collect();
            let output_path = extract_path.join(&relative);

            if entry.is_dir() {
                std::fs::create_dir_all(&output_path)?;
            } else {
                if let Some(parent) = output_path.parent() {
                    std::fs::create_dir_all(parent)?;
                }

                let mut output = std::fs::File::create(&output_path)?;
                std::io::copy(&mut entry, &mut output)?;
            }

            found = true;
        }

        Ok(found)
    };

    do_extract().map_err(|source| ComposerError::Archive {
        path: archive_path.to_path_buf(),
        source,
    })
}
