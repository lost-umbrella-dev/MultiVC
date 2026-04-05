#[path = "common/mod.rs"]
mod common;

use std::path::Path;

use clients::hash::Hash;
use composer::lock::content::ContentsLock;
use composer::lock::core::CoresLock;
use composer::lock::instances::InstancesLock;
use composer::utils::hash::item_path;

use common::init_test_tracing;

#[test]
fn item_path_sha256_cores() {
    let _guard = init_test_tracing();

    // 1. Создаём SHA256 хэш
    let hash = Hash::SHA256("abc".to_owned());

    // 2. Вычисляем путь для CoresLock
    let path = item_path::<CoresLock>(&hash);

    // 3. Проверяем что путь заканчивается на "cores/sha256-abc"
    assert_eq!(path, Path::new("cores").join("sha256-abc"));
}

#[test]
fn item_path_sha512_cores() {
    let _guard = init_test_tracing();

    // 1. Создаём SHA512 хэш
    let hash = Hash::SHA512("def".to_owned());

    // 2. Вычисляем путь для CoresLock
    let path = item_path::<CoresLock>(&hash);

    // 3. Проверяем что путь заканчивается на "cores/sha512-def"
    assert_eq!(path, Path::new("cores").join("sha512-def"));
}

#[test]
fn item_path_instances() {
    let _guard = init_test_tracing();

    // 1. Создаём SHA256 хэш
    let hash = Hash::SHA256("xyz".to_owned());

    // 2. Вычисляем путь для InstancesLock
    let path = item_path::<InstancesLock>(&hash);

    // 3. Проверяем что путь заканчивается на "instances/sha256-xyz"
    assert_eq!(path, Path::new("instances").join("sha256-xyz"));
}

#[test]
fn item_path_contents() {
    let _guard = init_test_tracing();

    // 1. Создаём SHA256 хэш
    let hash = Hash::SHA256("123".to_owned());

    // 2. Вычисляем путь для ContentsLock
    let path = item_path::<ContentsLock>(&hash);

    // 3. Проверяем что путь заканчивается на "contents/sha256-123"
    assert_eq!(path, Path::new("contents").join("sha256-123"));
}
