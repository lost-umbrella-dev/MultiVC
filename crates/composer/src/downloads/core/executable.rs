#[cfg(target_os = "windows")]
use crate::error::ComposerError;
use crate::error::Result;
use std::path::Path;
// ── Константы ────────────────────────────────────────────────────────

/// Каноническое имя исполняемого файла ядра: `core.{ext}`.
#[cfg(target_os = "windows")]
pub const CANONICAL_NAME: &str = "core.exe";

#[cfg(target_os = "linux")]
pub const CANONICAL_NAME: &str = "core.AppImage";

#[cfg(target_os = "macos")]
pub const CANONICAL_NAME: &str = "core.dmg";

#[cfg(not(any(target_os = "windows", target_os = "linux", target_os = "macos")))]
pub const CANONICAL_NAME: &str = "core.bin";

/// Имя исполняемого файла ядра, которое мы ищем в архиве.
///
/// В zip-архиве VoxelCore лежит несколько `.exe` файлов.
/// Нам нужен именно `VoxelCore.exe`, остальные — вспомогательные.
#[cfg(target_os = "windows")]
const VOXELCORE_EXE_NAME: &str = "VoxelCore.exe";

// ── Public API ───────────────────────────────────────────────────────

/// Расширение исполняемого файла ядра для текущей ОС.
#[allow(dead_code)]
pub fn ext() -> &'static str {
    #[cfg(target_os = "windows")]
    {
        "exe"
    }

    #[cfg(target_os = "linux")]
    {
        "AppImage"
    }

    #[cfg(target_os = "macos")]
    {
        "dmg"
    }

    #[cfg(not(any(target_os = "windows", target_os = "linux", target_os = "macos")))]
    {
        "bin"
    }
}

#[allow(dead_code)]
/// Каноническое имя исполняемого файла ядра.
pub fn name() -> &'static str {
    CANONICAL_NAME
}

#[allow(dead_code)]
/// Возвращает `true` если на текущей ОС ядро скачивается как архив,
/// который нужно распаковывать.
///
/// - **Windows**: `true` — качается `.zip`, внутри `VoxelCore.exe` + ресурсы.
/// - **Linux / macOS**: `false` — качается один файл (`.AppImage` / `.dmg`).
pub const fn needs_extraction() -> bool {
    cfg!(target_os = "windows")
}

/// Переименовывает исполняемый файл ядра в каноническое имя [`CANONICAL_NAME`].
///
/// Поведение зависит от платформы:
///
/// - **Windows** — рекурсивно ищет `VoxelCore.exe` в `dir` (распакованный архив),
///   перемещает в корень `dir` как `core.exe`. Остальные `.exe` не трогает.
///
/// - **Linux / macOS** — `source` это скачанный файл (`.AppImage` / `.dmg`),
///   `dir` — целевая директория. Переименовывает `source` → `dir/core.{ext}`.
///
/// # Arguments
///
/// - `dir` — директория с содержимым (extract dir на Windows, content dir на Linux/macOS).
/// - `source` — путь к скачанному файлу. **Игнорируется на Windows** (поиск рекурсивный).
///   На Linux/macOS это путь к файлу, который нужно переименовать.
pub async fn rename(dir: &Path, #[cfg_attr(target_os = "windows", allow(unused))] source: &Path) -> Result<()> {
    #[cfg(target_os = "windows")]
    {
        rename_archive(dir).await
    }

    #[cfg(not(target_os = "windows"))]
    {
        rename_direct(source, dir).await
    }
}

// ── Windows implementation ───────────────────────────────────────────

/// Рекурсивно ищет `VoxelCore.exe` в распакованной директории
/// и перемещает в корень как `core.exe`.
#[cfg(target_os = "windows")]
async fn rename_archive(extract_dir: &Path) -> Result<()> {
    let source = find_voxelcore_exe(extract_dir)
        .await?
        .ok_or_else(|| ComposerError::CoreExecutableNotFound {
            path: extract_dir.to_path_buf(),
        })?;

    let dest = extract_dir.join(CANONICAL_NAME);

    if source != dest {
        tracing::debug!(
            from = %source.display(),
            to = %dest.display(),
            "moving VoxelCore.exe → core.exe",
        );

        // copy + remove: надёжнее чем rename на Windows
        // (залоченный файл, антивирус, cross-volume, etc.)
        tokio::fs::copy(&source, &dest).await?;
        tokio::fs::remove_file(&source).await?;

        tracing::debug!(source = %source.display(), "original removed");
    }

    Ok(())
}

/// Рекурсивно ищет `VoxelCore.exe` (регистронезависимо) в директории и поддиректориях.
#[cfg(target_os = "windows")]
async fn find_voxelcore_exe(dir: &Path) -> Result<Option<std::path::PathBuf>> {
    let mut stack = vec![dir.to_path_buf()];

    while let Some(current) = stack.pop() {
        let mut entries = tokio::fs::read_dir(&current).await?;

        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();
            let file_type = entry.file_type().await?;

            if file_type.is_file() {
                if let Some(file_name) = path.file_name().and_then(|n| n.to_str())
                    && file_name.eq_ignore_ascii_case(VOXELCORE_EXE_NAME)
                {
                    return Ok(Some(path));
                }
            } else if file_type.is_dir() {
                stack.push(path);
            }
        }
    }

    Ok(None)
}

// ── Linux / macOS implementation ─────────────────────────────────────

/// Переименовывает скачанный файл в `core.{ext}`.
///
/// На этих платформах ядро — один файл (`.AppImage` или `.dmg`),
/// который не нужно распаковывать.
#[cfg(not(target_os = "windows"))]
async fn rename_direct(downloaded_file: &Path, target_dir: &Path) -> Result<()> {
    let dest = target_dir.join(CANONICAL_NAME);

    if downloaded_file != dest {
        tracing::debug!(
            from = %downloaded_file.display(),
            to = %dest.display(),
            "moving downloaded file → {}",
            CANONICAL_NAME,
        );

        tokio::fs::copy(downloaded_file, &dest).await?;
        tokio::fs::remove_file(downloaded_file).await?;

        tracing::debug!(source = %downloaded_file.display(), "original removed");
    }

    Ok(())
}

// ── Tests ────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn ext_returns_platform_value() {
        let result = ext();

        #[cfg(target_os = "windows")]
        assert_eq!(result, "exe");

        #[cfg(target_os = "linux")]
        assert_eq!(result, "AppImage");

        #[cfg(target_os = "macos")]
        assert_eq!(result, "dmg");

        assert!(!result.is_empty());
    }

    #[test]
    fn name_returns_canonical() {
        let result = name();
        let expected = format!("core.{}", ext());
        assert_eq!(result, expected);
    }

    #[test]
    fn needs_extraction_platform() {
        #[cfg(target_os = "windows")]
        assert!(needs_extraction());

        #[cfg(not(target_os = "windows"))]
        assert!(!needs_extraction());
    }

    // ── Windows: rename() через archive path ─────────────────────

    /// VoxelCore.exe на верхнем уровне → переименовывается в core.exe.
    /// Другие .exe остаются нетронутыми.
    #[cfg(target_os = "windows")]
    #[tokio::test]
    async fn rename_finds_voxelcore_ignores_others() {
        let tmp = TempDir::new().unwrap();
        let dir = tmp.path();

        fs::write(dir.join("VoxelCore.exe"), b"voxelcore-binary").unwrap();
        fs::write(dir.join("vcruntime140.exe"), b"runtime-binary").unwrap();

        // source игнорируется на Windows
        rename(dir, Path::new("")).await.unwrap();

        assert!(dir.join("core.exe").exists());
        assert_eq!(fs::read(dir.join("core.exe")).unwrap(), b"voxelcore-binary");
        assert!(!dir.join("VoxelCore.exe").exists());
        assert!(dir.join("vcruntime140.exe").exists());
        assert_eq!(fs::read(dir.join("vcruntime140.exe")).unwrap(), b"runtime-binary");
    }

    /// VoxelCore.exe во вложенной директории — поднимается в корень.
    #[cfg(target_os = "windows")]
    #[tokio::test]
    async fn rename_finds_nested() {
        let tmp = TempDir::new().unwrap();
        let dir = tmp.path();

        let nested = dir.join("voxelcore-0.31.0_win64");
        fs::create_dir(&nested).unwrap();
        fs::write(nested.join("VoxelCore.exe"), b"nested-binary").unwrap();

        rename(dir, Path::new("")).await.unwrap();

        assert!(dir.join("core.exe").exists());
        assert_eq!(fs::read(dir.join("core.exe")).unwrap(), b"nested-binary");
        assert!(!nested.join("VoxelCore.exe").exists());
    }

    /// Регистронезависимый поиск: `voxelcore.EXE` тоже находится.
    #[cfg(target_os = "windows")]
    #[tokio::test]
    async fn rename_case_insensitive() {
        let tmp = TempDir::new().unwrap();
        let dir = tmp.path();

        fs::write(dir.join("voxelcore.EXE"), b"case-test").unwrap();

        rename(dir, Path::new("")).await.unwrap();

        assert!(dir.join("core.exe").exists());
    }

    /// Нет VoxelCore.exe → ошибка CoreExecutableNotFound.
    #[cfg(target_os = "windows")]
    #[tokio::test]
    async fn rename_not_found() {
        let tmp = TempDir::new().unwrap();
        let dir = tmp.path();

        fs::write(dir.join("readme.txt"), b"not an exe").unwrap();
        fs::write(dir.join("other.exe"), b"wrong exe").unwrap();

        let result = rename(dir, Path::new("")).await;
        assert!(result.is_err());

        match result.unwrap_err() {
            ComposerError::CoreExecutableNotFound { .. } => {},
            other => panic!("ожидали CoreExecutableNotFound, получили: {other:?}"),
        }

        // other.exe не тронут
        assert!(dir.join("other.exe").exists());
    }

    /// Директория с именем VoxelCore.exe — игнорируется (ищем только файлы).
    #[cfg(target_os = "windows")]
    #[tokio::test]
    async fn rename_ignores_directories() {
        let tmp = TempDir::new().unwrap();
        let dir = tmp.path();

        fs::create_dir(dir.join("VoxelCore.exe")).unwrap();

        let result = rename(dir, Path::new("")).await;
        assert!(result.is_err());
    }

    // ── Linux / macOS: rename() через direct path ────────────────

    /// Скачанный файл перемещается и переименовывается.
    #[cfg(not(target_os = "windows"))]
    #[tokio::test]
    async fn rename_moves_direct_file() {
        let tmp = TempDir::new().unwrap();
        let dir = tmp.path();

        let downloaded = dir.join("voxelcore-0.31.0.AppImage");
        fs::write(&downloaded, b"appimage-data").unwrap();

        rename(dir, &downloaded).await.unwrap();

        let dest = dir.join(name());
        assert!(dest.exists());
        assert_eq!(fs::read(&dest).unwrap(), b"appimage-data");
        assert!(!downloaded.exists());
    }

    /// Файл уже с правильным именем — ничего не делаем.
    #[cfg(not(target_os = "windows"))]
    #[tokio::test]
    async fn rename_already_correct() {
        let tmp = TempDir::new().unwrap();
        let dir = tmp.path();

        let target = dir.join(name());
        fs::write(&target, b"already-correct").unwrap();

        rename(dir, &target).await.unwrap();

        assert!(target.exists());
        assert_eq!(fs::read(&target).unwrap(), b"already-correct");
    }
}
