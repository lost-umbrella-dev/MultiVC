#[path = "common/mod.rs"]
mod common;

use std::fs;

use composer::error::ComposerError;
use composer::utils::archive;
use tempfile::TempDir;

use common::{create_test_zip, init_test_tracing};

#[test]
fn extract_valid_zip() {
    let _guard = init_test_tracing();

    // 1. Создаём временную директорию
    let tmp = TempDir::new().unwrap();
    let archive_path = tmp.path().join("test.zip");
    let extract_path = tmp.path().join("extracted");

    // 2. Создаём тестовый ZIP-архив с двумя файлами
    create_test_zip(&archive_path, &[("hello.txt", b"world"), ("data.bin", &[1, 2, 3])]);

    // 3. Распаковываем архив
    archive::extract_zip(&archive_path, &extract_path).unwrap();

    // 4. Проверяем что файлы на месте
    assert!(extract_path.join("hello.txt").exists());
    assert!(extract_path.join("data.bin").exists());

    // 5. Проверяем содержимое файлов
    assert_eq!(fs::read_to_string(extract_path.join("hello.txt")).unwrap(), "world");
    assert_eq!(fs::read(extract_path.join("data.bin")).unwrap(), vec![1, 2, 3]);
}

#[test]
fn extract_nested_zip() {
    let _guard = init_test_tracing();

    // 1. Создаём временную директорию
    let tmp = TempDir::new().unwrap();
    let archive_path = tmp.path().join("nested.zip");
    let extract_path = tmp.path().join("extracted");

    // 2. Создаём ZIP-архив с вложенными папками
    create_test_zip(
        &archive_path,
        &[
            ("dir/file_a.txt", b"content a"),
            ("dir/subdir/file_b.txt", b"content b"),
            ("dir/subdir/deep/file_c.txt", b"content c"),
        ],
    );

    // 3. Распаковываем архив
    archive::extract_zip(&archive_path, &extract_path).unwrap();

    // 4. Проверяем что вложенные директории созданы
    assert!(extract_path.join("dir").is_dir());
    assert!(extract_path.join("dir/subdir").is_dir());
    assert!(extract_path.join("dir/subdir/deep").is_dir());

    // 5. Проверяем что файлы существуют и содержат правильные данные
    assert_eq!(fs::read_to_string(extract_path.join("dir/file_a.txt")).unwrap(), "content a");
    assert_eq!(
        fs::read_to_string(extract_path.join("dir/subdir/file_b.txt")).unwrap(),
        "content b"
    );
    assert_eq!(
        fs::read_to_string(extract_path.join("dir/subdir/deep/file_c.txt")).unwrap(),
        "content c"
    );
}

#[test]
fn extract_empty_zip() {
    let _guard = init_test_tracing();

    // 1. Создаём временную директорию
    let tmp = TempDir::new().unwrap();
    let archive_path = tmp.path().join("empty.zip");
    let extract_path = tmp.path().join("extracted");

    // 2. Создаём пустой ZIP-архив (без файлов)
    create_test_zip(&archive_path, &[]);

    // 3. Распаковываем — не должно быть ошибок
    archive::extract_zip(&archive_path, &extract_path).unwrap();

    // 4. Проверяем что целевая директория создана
    assert!(extract_path.exists());
    assert!(extract_path.is_dir());

    // 5. Проверяем что директория пуста
    let entries: Vec<_> = fs::read_dir(&extract_path).unwrap().collect();
    assert!(entries.is_empty(), "директория должна быть пустой");
}

#[test]
fn extract_invalid_file() {
    let _guard = init_test_tracing();

    // 1. Создаём временную директорию
    let tmp = TempDir::new().unwrap();
    let archive_path = tmp.path().join("not_a_zip.bin");
    let extract_path = tmp.path().join("extracted");

    // 2. Записываем невалидные данные (не ZIP-формат)
    fs::write(&archive_path, b"this is definitely not a zip archive").unwrap();

    // 3. Пытаемся распаковать — должна быть ошибка
    let result = archive::extract_zip(&archive_path, &extract_path);

    // 4. Проверяем что результат — ошибка
    assert!(result.is_err(), "невалидный файл должен вызвать ошибку");

    // 5. Проверяем что ошибка имеет вариант Archive с правильным путём
    match result.unwrap_err() {
        ComposerError::Archive {
            path,
            source: _,
        } => {
            assert_eq!(path, archive_path, "путь в ошибке должен совпадать с путём архива");
        },
        other => panic!("ожидали ComposerError::Archive, получили: {other:?}"),
    }
}

#[test]
fn extract_preserves_content() {
    let _guard = init_test_tracing();

    // 1. Подготавливаем тестовые данные разных типов
    let text_content = "Привет, мир! 🌍 Special chars: <>&\"'\n\ttabs and newlines\n";
    let binary_content: Vec<u8> = (0..=255).collect();
    let large_content = "A".repeat(100_000);

    // 2. Создаём временную директорию и ZIP-архив
    let tmp = TempDir::new().unwrap();
    let archive_path = tmp.path().join("content.zip");
    let extract_path = tmp.path().join("extracted");

    create_test_zip(
        &archive_path,
        &[
            ("text.txt", text_content.as_bytes()),
            ("binary.bin", &binary_content),
            ("large.txt", large_content.as_bytes()),
        ],
    );

    // 3. Распаковываем архив
    archive::extract_zip(&archive_path, &extract_path).unwrap();

    // 4. Проверяем что текстовое содержимое совпадает побайтово
    assert_eq!(
        fs::read_to_string(extract_path.join("text.txt")).unwrap(),
        text_content,
        "текстовое содержимое должно совпадать"
    );

    // 5. Проверяем что бинарное содержимое совпадает побайтово
    assert_eq!(
        fs::read(extract_path.join("binary.bin")).unwrap(),
        binary_content,
        "бинарное содержимое должно совпадать"
    );

    // 6. Проверяем что большой файл сохранил свой размер и содержимое
    let extracted_large = fs::read_to_string(extract_path.join("large.txt")).unwrap();
    assert_eq!(
        extracted_large.len(),
        large_content.len(),
        "размер большого файла должен совпадать"
    );
    assert_eq!(extracted_large, large_content, "содержимое большого файла должно совпадать");
}
