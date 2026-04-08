//! Интеграционные тесты для трейта `Lock` и его реализаций.

#[path = "common/mod.rs"]
mod common;

use clients::item::CoreOrigin;
use composer::error::ComposerError;
use composer::lock::Lock;
use composer::lock::core::CoresLock;
use composer::utils::hash::item_path;
use tempfile::TempDir;

use common::{fake_hash, init_test_tracing, make_lock_item, make_lock_map};

// ── 1. save → load round-trip ────────────────────────────────────────

#[tokio::test]
async fn save_then_load_roundtrip() {
    let _guard = init_test_tracing();

    // 1. Создаём временную директорию и путь к cores/
    let tmp = TempDir::new().unwrap();
    let cores_dir = tmp.path().join("cores");
    std::fs::create_dir_all(&cores_dir).unwrap();

    // 2. Заполняем CoresLock тремя элементами
    let (_, pairs) = make_lock_map(3);
    let lock = CoresLock::default();
    for (hash, item) in &pairs {
        lock.items().insert(hash.clone(), item.clone());
    }

    // 3. Сохраняем на диск
    let lock_file = cores_dir.join("lock.toml");
    lock.save(&lock_file).await.expect("save() не должен завершиться ошибкой");

    // 4. Проверяем что файл появился на диске
    assert!(lock_file.exists(), "lock-файл должен существовать после save()");

    // 5. Загружаем обратно
    let loaded = CoresLock::load(&lock_file).await.expect("load() не должен завершиться ошибкой");

    // 6. Проверяем что количество элементов совпадает
    assert_eq!(
        loaded.items().len(),
        pairs.len(),
        "количество элементов после round-trip должно совпадать"
    );

    // 7. Проверяем каждый элемент по ключу
    for (hash, original_item) in &pairs {
        let loaded_item = loaded
            .items()
            .get(hash)
            .unwrap_or_else(|| panic!("элемент с хэшем {hash} должен присутствовать после load()"));
        assert_eq!(loaded_item.item.name, original_item.item.name, "имя элемента должно совпадать");
        assert_eq!(
            loaded_item.item.version, original_item.item.version,
            "версия элемента должна совпадать"
        );
        assert_eq!(loaded_item.item.url, original_item.item.url, "URL элемента должен совпадать");
    }
}

// ── 2. load при отсутствии файла создаёт default ─────────────────────

#[tokio::test]
async fn load_nonexistent_creates_default() {
    let _guard = init_test_tracing();

    // 1. Создаём пустую временную директорию с подпапкой cores/ без lock.toml
    let tmp = TempDir::new().unwrap();
    let cores_dir = tmp.path().join("cores");
    std::fs::create_dir_all(&cores_dir).unwrap();
    let lock_file = cores_dir.join("lock.toml");

    // 2. Убеждаемся что файла нет
    assert!(!lock_file.exists(), "lock-файл не должен существовать до вызова load()");

    // 3. Вызываем load() — должен создать пустой lock и записать файл
    let loaded = CoresLock::load(&lock_file)
        .await
        .expect("load() для несуществующего файла не должен завершиться ошибкой");

    // 4. Проверяем что lock пустой (default)
    assert_eq!(loaded.items().len(), 0, "загруженный lock должен быть пустым (default)");

    // 5. Проверяем что файл был создан на диске
    assert!(lock_file.exists(), "lock-файл должен быть создан на диске после load() с default");

    // 6. Повторный load() должен успешно прочитать пустой файл
    let reloaded =
        CoresLock::load(&lock_file).await.expect("повторный load() не должен завершиться ошибкой");
    assert_eq!(reloaded.items().len(), 0, "повторный load() должен вернуть пустой lock");
}

// ── 3. load повреждённого файла → ошибка десериализации ──────────────

#[tokio::test]
async fn load_corrupted_file() {
    let _guard = init_test_tracing();

    // 1. Создаём временную директорию и записываем невалидный TOML
    let tmp = TempDir::new().unwrap();
    let cores_dir = tmp.path().join("cores");
    std::fs::create_dir_all(&cores_dir).unwrap();
    let lock_file = cores_dir.join("lock.toml");
    std::fs::write(&lock_file, b"{{{{invalid toml garbage!@#$%").unwrap();

    // 2. Вызываем load() — должен вернуть ошибку десериализации
    let result = CoresLock::load(&lock_file).await;

    // 3. Проверяем что результат — ошибка
    assert!(result.is_err(), "load() повреждённого файла должен вернуть ошибку");

    // 4. Проверяем что ошибка — Serialise (toml::de::Error)
    let err = result.unwrap_err();
    assert!(
        matches!(err, ComposerError::Serialise(_)),
        "ошибка должна быть ComposerError::Serialise, получили: {err:?}"
    );
}

// ── 4. remove существующего элемента ─────────────────────────────────

#[tokio::test]
async fn remove_existing_item() {
    let _guard = init_test_tracing();

    // 1. Создаём временную директорию
    let tmp = TempDir::new().unwrap();
    let cores_dir = tmp.path().join("cores");
    std::fs::create_dir_all(&cores_dir).unwrap();

    // 2. Создаём CoresLock с одним элементом
    let hash = fake_hash("abcdef1234567890");
    let lock_item = make_lock_item("test-core", "2.0.0");
    let lock = CoresLock::default();
    lock.items().insert(hash.clone(), lock_item.clone());

    // 3. Создаём директорию элемента на диске (как если бы он был скачан)
    let dir_path = item_path(&cores_dir, &hash);
    std::fs::create_dir_all(&dir_path).unwrap();
    std::fs::write(dir_path.join("dummy.bin"), b"some data").unwrap();
    assert!(dir_path.exists(), "директория элемента должна существовать до remove()");

    // 4. Вызываем remove() — должен вернуть Some(item)
    let removed =
        lock.remove(&cores_dir, &hash).await.expect("remove() не должен завершиться ошибкой");
    assert!(removed.is_some(), "remove() должен вернуть Some для существующего элемента");

    // 5. Проверяем что возвращённый элемент совпадает с оригиналом
    let removed_item = removed.unwrap();
    assert_eq!(
        removed_item.item.name, lock_item.item.name,
        "имя удалённого элемента должно совпадать"
    );
    assert_eq!(
        removed_item.item.version, lock_item.item.version,
        "версия удалённого элемента должна совпадать"
    );

    // 6. Проверяем что элемент удалён из DashMap
    assert_eq!(lock.items().len(), 0, "после remove() в lock не должно быть элементов");

    // 7. Проверяем что директория элемента удалена с диска
    assert!(!dir_path.exists(), "директория элемента должна быть удалена после remove()");
}

// ── 5. remove несуществующего элемента → None ────────────────────────

#[tokio::test]
async fn remove_nonexistent_item() {
    let _guard = init_test_tracing();

    // 1. Создаём временную директорию
    let tmp = TempDir::new().unwrap();
    let cores_dir = tmp.path().join("cores");
    std::fs::create_dir_all(&cores_dir).unwrap();

    // 2. Создаём пустой CoresLock
    let lock = CoresLock::default();

    // 3. Пытаемся удалить несуществующий элемент
    let hash = fake_hash("nonexistent_hash_value");
    let removed = lock
        .remove(&cores_dir, &hash)
        .await
        .expect("remove() не должен завершиться ошибкой даже для несуществующего элемента");

    // 4. Проверяем что результат — None
    assert!(removed.is_none(), "remove() должен вернуть None для несуществующего элемента");

    // 5. Проверяем что lock по-прежнему пуст
    assert_eq!(
        lock.items().len(),
        0,
        "lock должен оставаться пустым после remove() несуществующего элемента"
    );
}

// ── 6. DashMap с #[serde(flatten)] корректно (де)сериализуется ───────

#[tokio::test]
async fn dashmap_serde_roundtrip() {
    // Этот тест работает только с сериализацией в памяти
    let _guard = init_test_tracing();

    // 1. Создаём CoresLock с несколькими элементами
    let (_, pairs) = make_lock_map(4);
    let lock = CoresLock::default();
    for (hash, item) in &pairs {
        lock.items().insert(hash.clone(), item.clone());
    }

    // 2. Сериализуем в TOML-строку
    let toml_str = toml::to_string_pretty(&lock)
        .expect("сериализация CoresLock в TOML не должна завершиться ошибкой");

    // 3. Проверяем что TOML-строка не пустая
    assert!(!toml_str.is_empty(), "сериализованный TOML не должен быть пустым");

    // 4. Десериализуем обратно в CoresLock
    let deserialized: CoresLock = toml::from_str(&toml_str)
        .expect("десериализация CoresLock из TOML не должна завершиться ошибкой");

    // 5. Проверяем что количество элементов совпадает
    assert_eq!(
        deserialized.items().len(),
        pairs.len(),
        "количество элементов после TOML round-trip должно совпадать"
    );

    // 6. Проверяем каждый элемент по ключу
    for (hash, original_item) in &pairs {
        let loaded_item = deserialized.items().get(hash).unwrap_or_else(|| {
            panic!("элемент с хэшем {hash} должен присутствовать после десериализации")
        });
        assert_eq!(
            loaded_item.item.name, original_item.item.name,
            "имя элемента должно совпадать после TOML round-trip"
        );
        assert_eq!(
            loaded_item.item.version, original_item.item.version,
            "версия элемента должна совпадать после TOML round-trip"
        );
        assert_eq!(
            loaded_item.item.url, original_item.item.url,
            "URL элемента должен совпадать после TOML round-trip"
        );
        assert_eq!(
            loaded_item.item.size, original_item.item.size,
            "размер элемента должен совпадать после TOML round-trip"
        );
    }

    // 7. Проверяем что TOML содержит ключи в формате "sha256:..."
    for (hash, _) in &pairs {
        let key = hash.to_string();
        assert!(
            toml_str.contains(&format!("[{key}]")) || toml_str.contains(&format!("\"{key}\"")),
            "TOML должен содержать ключ {key}"
        );
    }
}

// ── 7. CoreOrigin defaults to Release when missing from TOML ────────

#[tokio::test]
async fn origin_defaults_to_release_when_missing() {
    let _guard = init_test_tracing();

    // 1. Create a lock with one item and serialize to TOML
    let lock = CoresLock::default();
    let hash = fake_hash("origin_test_hash");
    let item = make_lock_item("origin-core", "1.0.0");
    lock.items().insert(hash.clone(), item);
    let toml_str = toml::to_string_pretty(&lock).unwrap();

    // 2. Remove the `origin` line to simulate an old lock file
    let toml_without_origin: String =
        toml_str.lines().filter(|l| !l.starts_with("origin")).collect::<Vec<_>>().join("\n");

    // 3. Deserialize — origin should default to Release
    let loaded: CoresLock = toml::from_str(&toml_without_origin)
        .expect("deserialization without origin field should succeed");
    let loaded_item = loaded.items().get(&hash).expect("item must be present");
    assert_eq!(
        loaded_item.origin,
        CoreOrigin::Release,
        "missing origin field must default to Release"
    );
}

// ── 8. CoreOrigin::Build roundtrips through TOML ────────────────────

#[tokio::test]
async fn origin_build_roundtrip() {
    let _guard = init_test_tracing();

    // 1. Create a lock item with origin: Build
    let lock = CoresLock::default();
    let hash = fake_hash("build_origin_hash");
    let mut item = make_lock_item("built-core", "2.0.0");
    item.origin = CoreOrigin::Build;
    lock.items().insert(hash.clone(), item);

    // 2. Serialize to TOML
    let toml_str = toml::to_string_pretty(&lock).unwrap();

    // 3. Verify "Build" appears in the serialized output
    assert!(toml_str.contains("Build"), "serialized TOML must contain 'Build' for origin field");

    // 4. Deserialize and check roundtrip
    let loaded: CoresLock =
        toml::from_str(&toml_str).expect("deserialization with Build origin should succeed");
    let loaded_item = loaded.items().get(&hash).expect("item must be present");
    assert_eq!(loaded_item.origin, CoreOrigin::Build, "origin: Build must survive TOML roundtrip");
}
