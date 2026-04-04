use std::path::Path;

use crate::error::{ComposerError, Result};

/// Расширение исполняемого файла ядра для текущей ОС.
pub fn ext() -> &'static str {
    match std::env::consts::OS {
        "windows" => "exe",
        "linux" => "AppImage",
        "macos" => "dmg",
        _ => "bin",
    }
}

/// Каноническое имя исполняемого файла ядра: `core.{ext}`.
pub fn name() -> String {
    format!("core.{}", ext())
}

/// Находит исполняемый файл ядра в распакованной директории и переименовывает в `core.{ext}`.
///
/// Поиск по расширению, специфичному для текущей ОС.
/// Если найден файл — переименовывает. Иначе — ошибка [`ComposerError::CoreExecutableNotFound`].
pub async fn rename(extract_dir: &Path) -> Result<()> {
    let extension = ext();
    let target_name = name();

    let mut entries = tokio::fs::read_dir(extract_dir).await?;
    let mut found: Option<std::path::PathBuf> = None;

    while let Some(entry) = entries.next_entry().await? {
        let path = entry.path();
        if path.is_file() {
            let matches =
                matches!(path.extension().and_then(|e| e.to_str()), Some(e) if e.eq_ignore_ascii_case(extension));
            if matches {
                found = Some(path);
                break;
            }
        }
    }

    let source = found.ok_or_else(|| ComposerError::CoreExecutableNotFound {
        path: extract_dir.to_path_buf(),
    })?;

    let dest = extract_dir.join(&target_name);

    if source != dest {
        tracing::debug!(
            from = %source.display(),
            to = %dest.display(),
            "renaming core executable",
        );
        tokio::fs::rename(&source, &dest).await?;
    }

    Ok(())
}
