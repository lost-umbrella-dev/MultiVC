#[path = "common/mod.rs"]
mod common;

use std::path::Path;

use clients::hash::Hash;
use composer::utils::hash::item_path;

use common::init_test_tracing;

#[test]
fn item_path_sha256_cores() {
    let _guard = init_test_tracing();

    // 1. Создаём SHA256 хэш
    let hash = Hash::SHA256("abc".to_owned());

    // 2. Вычисляем путь с базовой директорией "cores"
    let path = item_path(Path::new("cores"), &hash);

    // 3. Проверяем что путь равен "cores/sha256-abc"
    assert_eq!(path, Path::new("cores").join("sha256-abc"));
}

#[test]
fn item_path_sha512_cores() {
    let _guard = init_test_tracing();

    // 1. Создаём SHA512 хэш
    let hash = Hash::SHA512("def".to_owned());

    // 2. Вычисляем путь с базовой директорией "cores"
    let path = item_path(Path::new("cores"), &hash);

    // 3. Проверяем что путь равен "cores/sha512-def"
    assert_eq!(path, Path::new("cores").join("sha512-def"));
}

#[test]
fn item_path_contents() {
    let _guard = init_test_tracing();

    // 1. Создаём SHA256 хэш
    let hash = Hash::SHA256("123".to_owned());

    // 2. Вычисляем путь с базовой директорией "contents"
    let path = item_path(Path::new("contents"), &hash);

    // 3. Проверяем что путь равен "contents/sha256-123"
    assert_eq!(path, Path::new("contents").join("sha256-123"));
}
