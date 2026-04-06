#[path = "common/mod.rs"]
mod common;

use std::path::Path;

use clients::hash::Hash;
use composer::utils::fs;
use tempfile::TempDir;

use common::{fake_hash, init_test_tracing, write_test_file};

#[test]
fn hash_empty_directory() {
    let _guard = init_test_tracing();

    // 1. Создаём пустую временную директорию
    let tmp = TempDir::new().unwrap();

    // 2. Вычисляем хэш пустой директории
    let hash = fs::compute_directory_hash(tmp.path()).unwrap();

    // 3. Проверяем что результат — SHA256 и вычисление не паникует
    assert!(matches!(hash, Hash::SHA256(_)), "хэш пустой директории должен быть SHA256");
}

#[test]
fn hash_single_file() {
    let _guard = init_test_tracing();

    // 1. Создаём директорию с одним файлом
    let tmp = TempDir::new().unwrap();
    write_test_file(tmp.path(), "hello.txt", b"Hello, world!");

    // 2. Вычисляем хэш
    let hash = fs::compute_directory_hash(tmp.path()).unwrap();

    // 3. Проверяем что результат — SHA256 с непустым hex-дайджестом
    match &hash {
        Hash::SHA256(hex_str) => {
            assert!(!hex_str.is_empty(), "hex-дайджест не должен быть пустым");
            assert!(
                hex_str.len() == 64,
                "SHA256 hex-дайджест должен быть длиной 64 символа, получили {}",
                hex_str.len()
            );
        },
        _ => panic!("ожидался Hash::SHA256"),
    }

    // 4. Повторно вычисляем — результат детерминистичный
    let hash2 = fs::compute_directory_hash(tmp.path()).unwrap();
    assert_eq!(hash, hash2, "повторное вычисление должно дать тот же хэш");
}

#[test]
fn hash_nested_dirs() {
    let _guard = init_test_tracing();

    // 1. Создаём дерево с вложенными директориями и файлами
    let tmp = TempDir::new().unwrap();
    write_test_file(tmp.path(), "root.txt", b"root content");
    write_test_file(tmp.path(), "sub/nested.txt", b"nested content");
    write_test_file(tmp.path(), "sub/deep/leaf.txt", b"leaf content");

    // 2. Вычисляем хэш дерева
    let hash = fs::compute_directory_hash(tmp.path()).unwrap();

    // 3. Проверяем что хэш вычислен корректно (включает маркеры dir: и file:)
    assert!(matches!(hash, Hash::SHA256(_)), "хэш вложенных директорий должен быть SHA256");

    // 4. Сравниваем с хэшем плоской директории — должны различаться
    let flat_tmp = TempDir::new().unwrap();
    write_test_file(flat_tmp.path(), "root.txt", b"root content");
    let flat_hash = fs::compute_directory_hash(flat_tmp.path()).unwrap();

    assert_ne!(
        hash, flat_hash,
        "хэш дерева с вложенными директориями должен отличаться от плоской структуры"
    );
}

#[test]
fn hash_deterministic() {
    let _guard = init_test_tracing();

    // 1. Создаём две идентичные директории с одинаковым содержимым
    let tmp_a = TempDir::new().unwrap();
    write_test_file(tmp_a.path(), "alpha.txt", b"content alpha");
    write_test_file(tmp_a.path(), "beta.txt", b"content beta");
    write_test_file(tmp_a.path(), "sub/gamma.txt", b"content gamma");

    let tmp_b = TempDir::new().unwrap();
    write_test_file(tmp_b.path(), "alpha.txt", b"content alpha");
    write_test_file(tmp_b.path(), "beta.txt", b"content beta");
    write_test_file(tmp_b.path(), "sub/gamma.txt", b"content gamma");

    // 2. Вычисляем хэши обеих директорий
    let hash_a = fs::compute_directory_hash(tmp_a.path()).unwrap();
    let hash_b = fs::compute_directory_hash(tmp_b.path()).unwrap();

    // 3. Проверяем что хэши совпадают — детерминизм
    assert_eq!(hash_a, hash_b, "два идентичных дерева файлов должны давать одинаковый хэш");
}

#[test]
fn hash_content_change_detected() {
    let _guard = init_test_tracing();

    // 1. Создаём директорию и вычисляем начальный хэш
    let tmp = TempDir::new().unwrap();
    write_test_file(tmp.path(), "data.txt", b"original content");
    let hash_before = fs::compute_directory_hash(tmp.path()).unwrap();

    // 2. Изменяем содержимое файла
    write_test_file(tmp.path(), "data.txt", b"modified content");

    // 3. Вычисляем хэш после изменения
    let hash_after = fs::compute_directory_hash(tmp.path()).unwrap();

    // 4. Проверяем что хэш изменился
    assert_ne!(
        hash_before, hash_after,
        "изменение содержимого файла должно приводить к другому хэшу"
    );
}

#[test]
fn hash_filename_change_detected() {
    let _guard = init_test_tracing();

    // 1. Создаём директорию с файлом и вычисляем хэш
    let tmp_a = TempDir::new().unwrap();
    write_test_file(tmp_a.path(), "original_name.txt", b"same content");
    let hash_a = fs::compute_directory_hash(tmp_a.path()).unwrap();

    // 2. Создаём директорию с тем же содержимым, но другим именем файла
    let tmp_b = TempDir::new().unwrap();
    write_test_file(tmp_b.path(), "renamed_file.txt", b"same content");
    let hash_b = fs::compute_directory_hash(tmp_b.path()).unwrap();

    // 3. Проверяем что хэши различаются — имя файла влияет на хэш
    assert_ne!(hash_a, hash_b, "переименование файла должно приводить к другому хэшу");
}

#[test]
fn hash_add_file_detected() {
    let _guard = init_test_tracing();

    // 1. Создаём директорию с одним файлом и вычисляем хэш
    let tmp = TempDir::new().unwrap();
    write_test_file(tmp.path(), "first.txt", b"first");
    let hash_before = fs::compute_directory_hash(tmp.path()).unwrap();

    // 2. Добавляем второй файл
    write_test_file(tmp.path(), "second.txt", b"second");

    // 3. Вычисляем хэш после добавления
    let hash_after = fs::compute_directory_hash(tmp.path()).unwrap();

    // 4. Проверяем что хэш изменился
    assert_ne!(hash_before, hash_after, "добавление нового файла должно приводить к другому хэшу");
}

#[test]
fn hash_directory_verify_match() {
    let _guard = init_test_tracing();

    // 1. Создаём директорию с файлами
    let tmp = TempDir::new().unwrap();
    write_test_file(tmp.path(), "config.toml", b"[settings]\nkey = \"value\"");
    write_test_file(tmp.path(), "src/main.rs", b"fn main() {}");

    // 2. Вычисляем эталонный хэш
    let expected_hash = fs::compute_directory_hash(tmp.path()).unwrap();

    // 3. Проверяем что hash_directory возвращает true при совпадении
    let result = fs::hash_directory(tmp.path(), &expected_hash);
    assert!(result.is_ok(), "hash_directory не должен возвращать ошибку: {:?}", result.err());
    assert!(result.unwrap(), "hash_directory должен вернуть true для корректного хэша");
}

#[test]
fn hash_directory_verify_mismatch() {
    let _guard = init_test_tracing();

    // 1. Создаём директорию с файлом
    let tmp = TempDir::new().unwrap();
    write_test_file(tmp.path(), "readme.md", b"# Hello");

    // 2. Создаём заведомо неправильный хэш
    let wrong_hash = fake_hash("0000000000000000000000000000000000000000000000000000000000000000");

    // 3. Проверяем что hash_directory возвращает false при несовпадении
    let result = fs::hash_directory(tmp.path(), &wrong_hash);
    assert!(
        result.is_ok(),
        "hash_directory не должен возвращать ошибку при несовпадении: {:?}",
        result.err()
    );
    assert!(!result.unwrap(), "hash_directory должен вернуть false для неверного хэша");
}

#[test]
fn normalize_relative_path_uses_forward_slash() {
    let _guard = init_test_tracing();

    // 1. Проверяем нормализацию простого пути
    let simple = Path::new("foo").join("bar").join("baz.txt");
    let normalized = fs::normalize_relative_path(&simple);
    assert_eq!(normalized, "foo/bar/baz.txt", "путь должен использовать прямые слеши");

    // 2. Проверяем что результат не содержит обратных слешей
    assert!(!normalized.contains('\\'), "нормализованный путь не должен содержать обратных слешей");

    // 3. Проверяем однокомпонентный путь
    let single = Path::new("file.txt");
    let normalized_single = fs::normalize_relative_path(single);
    assert_eq!(
        normalized_single, "file.txt",
        "однокомпонентный путь должен остаться без изменений"
    );

    // 4. Проверяем глубоко вложенный путь
    let deep = Path::new("a").join("b").join("c").join("d").join("e.rs");
    let normalized_deep = fs::normalize_relative_path(&deep);
    assert_eq!(
        normalized_deep, "a/b/c/d/e.rs",
        "глубоко вложенный путь должен использовать прямые слеши"
    );
}
