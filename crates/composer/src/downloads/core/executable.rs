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

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    // 1. ext() возвращает платформозависимое расширение
    #[test]
    fn ext_returns_platform_value() {
        let result = ext();

        // Шаг 1: проверяем, что результат соответствует текущей ОС
        #[cfg(target_os = "windows")]
        assert_eq!(result, "exe");

        #[cfg(target_os = "linux")]
        assert_eq!(result, "AppImage");

        #[cfg(target_os = "macos")]
        assert_eq!(result, "dmg");

        // Шаг 2: в любом случае строка не пустая
        assert!(!result.is_empty(), "расширение не должно быть пустым");
    }

    // 2. name() возвращает «core.{ext}»
    #[test]
    fn name_format() {
        // Шаг 1: получаем имя
        let result = name();

        // Шаг 2: проверяем формат
        let expected = format!("core.{}", ext());
        assert_eq!(result, expected);
    }

    // 3. rename находит файл по расширению и переименовывает в core.{ext}
    #[tokio::test]
    async fn rename_finds_and_renames() {
        // Шаг 1: создаём временную директорию
        let tmp = TempDir::new().expect("не удалось создать временную директорию");
        let dir = tmp.path();

        // Шаг 2: создаём файл с правильным расширением, но другим именем
        let extension = ext();
        let source_name = format!("server-v1.2.3.{extension}");
        let source_path = dir.join(&source_name);
        fs::write(&source_path, b"fake-binary").expect("не удалось записать файл");

        // Шаг 3: вызываем rename
        rename(dir).await.expect("rename завершился с ошибкой");

        // Шаг 4: исходный файл должен исчезнуть, а core.{ext} — появиться
        assert!(!source_path.exists(), "исходный файл должен быть удалён");
        let dest_path = dir.join(name());
        assert!(dest_path.exists(), "целевой файл core.{{ext}} должен существовать");

        // Шаг 5: содержимое должно сохраниться
        let content = fs::read(&dest_path).expect("не удалось прочитать целевой файл");
        assert_eq!(content, b"fake-binary");
    }

    // 4. Файл уже называется core.{ext} — ничего не делаем, ошибки нет
    #[tokio::test]
    async fn rename_already_named_core() {
        // Шаг 1: создаём временную директорию
        let tmp = TempDir::new().expect("не удалось создать временную директорию");
        let dir = tmp.path();

        // Шаг 2: создаём файл, который уже называется core.{ext}
        let target = dir.join(name());
        fs::write(&target, b"already-correct").expect("не удалось записать файл");

        // Шаг 3: вызываем rename — должно пройти без ошибки
        rename(dir)
            .await
            .expect("rename не должен падать, если файл уже назван правильно");

        // Шаг 4: файл на месте, содержимое не изменилось
        assert!(target.exists(), "файл должен остаться на месте");
        let content = fs::read(&target).expect("не удалось прочитать файл");
        assert_eq!(content, b"already-correct");
    }

    // 5. Нет файла с нужным расширением → CoreExecutableNotFound
    #[tokio::test]
    async fn rename_no_executable_found() {
        // Шаг 1: создаём пустую временную директорию
        let tmp = TempDir::new().expect("не удалось создать временную директорию");
        let dir = tmp.path();

        // Шаг 2: добавляем файл с неправильным расширением
        fs::write(dir.join("readme.txt"), b"not an executable").expect("не удалось записать файл");

        // Шаг 3: вызываем rename — ожидаем ошибку
        let result = rename(dir).await;
        assert!(result.is_err(), "должна быть ошибка, если нет исполняемого файла");

        // Шаг 4: проверяем, что ошибка именно CoreExecutableNotFound
        let err = result.unwrap_err();
        match err {
            ComposerError::CoreExecutableNotFound { path } => {
                assert_eq!(path, dir.to_path_buf(), "путь в ошибке должен совпадать с директорией");
            },
            other => panic!("ожидали CoreExecutableNotFound, получили: {other:?}"),
        }
    }

    // 6. Регистронезависимый поиск: «SERVER.EXE» → находит и переименовывает (только Windows)
    #[cfg(target_os = "windows")]
    #[tokio::test]
    async fn rename_case_insensitive() {
        // Шаг 1: создаём временную директорию
        let tmp = TempDir::new().expect("не удалось создать временную директорию");
        let dir = tmp.path();

        // Шаг 2: создаём файл с расширением в верхнем регистре
        let upper = format!("SERVER.{}", ext().to_ascii_uppercase());
        let source_path = dir.join(&upper);
        fs::write(&source_path, b"upper-case-ext").expect("не удалось записать файл");

        // Шаг 3: вызываем rename
        rename(dir)
            .await
            .expect("rename должен найти файл с расширением в верхнем регистре");

        // Шаг 4: core.{ext} должен появиться
        let dest_path = dir.join(name());
        assert!(dest_path.exists(), "целевой файл core.{{ext}} должен существовать");

        // Шаг 5: содержимое сохранилось
        let content = fs::read(&dest_path).expect("не удалось прочитать целевой файл");
        assert_eq!(content, b"upper-case-ext");
    }

    // 7. Директория с подходящим расширением в имени — не считается файлом
    #[tokio::test]
    async fn rename_ignores_directories() {
        // Шаг 1: создаём временную директорию
        let tmp = TempDir::new().expect("не удалось создать временную директорию");
        let dir = tmp.path();

        // Шаг 2: создаём вложенную директорию, имя которой заканчивается на нужное расширение
        let decoy_name = format!("something.{}", ext());
        let decoy_dir = dir.join(&decoy_name);
        fs::create_dir(&decoy_dir).expect("не удалось создать директорию-обманку");

        // Шаг 3: вызываем rename — директория не должна считаться исполняемым файлом
        let result = rename(dir).await;
        assert!(result.is_err(), "директория не должна считаться исполняемым файлом");

        // Шаг 4: проверяем тип ошибки
        let err = result.unwrap_err();
        match err {
            ComposerError::CoreExecutableNotFound { .. } => {
                // Ожидаемое поведение: директория проигнорирована, файл не найден
            },
            other => panic!("ожидали CoreExecutableNotFound, получили: {other:?}"),
        }

        // Шаг 5: директория-обманка осталась нетронутой
        assert!(decoy_dir.exists(), "директория-обманка не должна быть затронута");
        assert!(decoy_dir.is_dir(), "обманка должна оставаться директорией");
    }
}
