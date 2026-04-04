use std::path::Path;

/// Распаковывает ZIP-архив в указанную директорию.
///
/// Использует `enclosed_name()`, чтобы игнорировать небезопасные пути
/// внутри архива, например содержащие попытки выйти за пределы целевой папки.
pub fn extract_zip(archive_path: &Path, extract_path: &Path) -> std::io::Result<()> {
    let file = std::fs::File::open(archive_path)?;
    let mut archive = zip::ZipArchive::new(file).map_err(std::io::Error::other)?;

    std::fs::create_dir_all(extract_path)?;

    for index in 0..archive.len() {
        let mut entry = archive.by_index(index).map_err(std::io::Error::other)?;

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
}
