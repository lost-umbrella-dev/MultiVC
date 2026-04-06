//! Интеграционные тесты для трейта `Lock` и его реализаций.
//!
//! **Важно:** тесты изменяют текущую рабочую директорию процесса,
//! поэтому их необходимо запускать с `--test-threads=1` или использовать
//! мьютекс `CWD_LOCK` для сериализации.

#[path = "common/mod.rs"]
mod common;

use tokio::sync::Mutex;

use composer::error::ComposerError;
use composer::item::LockMap;
use composer::lock::Lock;
use composer::lock::core::CoresLock;
use composer::utils::hash::item_path;
use tempfile::TempDir;

use common::{fake_hash, init_test_tracing, make_lock_item, make_lock_map};

/// Мьютекс для сериализации тестов, изменяющих текущую рабочую директорию.
///
/// `std::env::set_current_dir` влияет на весь процесс, поэтому тесты,
/// которые его вызывают, не могут выполняться параллельно.
static CWD_LOCK: Mutex<()> = Mutex::const_new(());

// ── 1. save → load round-trip ────────────────────────────────────────

#[tokio::test]
async fn save_then_load_roundtrip() {
    // Захватываем мьютекс, чтобы ни один другой тест не менял cwd
    let _cwd = CWD_LOCK.lock().await;
    let _guard = init_test_tracing();

    // 1. Создаём временную директорию и переключаем cwd
    let tmp = TempDir::new().unwrap();
    let original_dir = std::env::current_dir().unwrap();
    std::env::set_current_dir(tmp.path()).unwrap();

    // 2. Создаём директорию `cores/` — Lock::save() пишет в `cores/lock.toml`
    std::fs::create_dir_all("cores").unwrap();

    // 3. Заполняем CoresLock тремя элементами
    let (map, pairs) = make_lock_map(3);
    let lock = CoresLock {
        items: map,
    };

    // 4. Сохраняем на диск
    lock.save().await.expect("save() не должен завершиться ошибкой");

    // 5. Проверяем что файл появился на диске
    assert!(
        std::path::Path::new("cores/lock.toml").exists(),
        "lock-файл должен существовать после save()"
    );

    // 6. Загружаем обратно
    let loaded = CoresLock::load().await.expect("load() не должен завершиться ошибкой");

    // 7. Проверяем что количество элементов совпадает
    assert_eq!(
        loaded.items().len(),
        pairs.len(),
        "количество элементов после round-trip должно совпадать"
    );

    // 8. Проверяем каждый элемент по ключу
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

    // Восстанавливаем cwd
    std::env::set_current_dir(&original_dir).unwrap();
}

// ── 2. load при отсутствии файла создаёт default ─────────────────────

#[tokio::test]
async fn load_nonexistent_creates_default() {
    // Захватываем мьютекс
    let _cwd = CWD_LOCK.lock().await;
    let _guard = init_test_tracing();

    // 1. Создаём пустую временную директорию и переключаем cwd
    let tmp = TempDir::new().unwrap();
    let original_dir = std::env::current_dir().unwrap();
    std::env::set_current_dir(tmp.path()).unwrap();

    // 2. Создаём директорию `cores/` — без lock.toml внутри
    std::fs::create_dir_all("cores").unwrap();

    // 3. Убеждаемся что файла нет
    assert!(
        !std::path::Path::new("cores/lock.toml").exists(),
        "lock-файл не должен существовать до вызова load()"
    );

    // 4. Вызываем load() — должен создать пустой lock и записать файл
    let loaded = CoresLock::load()
        .await
        .expect("load() для несуществующего файла не должен завершиться ошибкой");

    // 5. Проверяем что lock пустой (default)
    assert_eq!(loaded.items().len(), 0, "загруженный lock должен быть пустым (default)");

    // 6. Проверяем что файл был создан на диске
    assert!(
        std::path::Path::new("cores/lock.toml").exists(),
        "lock-файл должен быть создан на диске после load() с default"
    );

    // 7. Повторный load() должен успешно прочитать пустой файл
    let reloaded = CoresLock::load().await.expect("повторный load() не должен завершиться ошибкой");
    assert_eq!(reloaded.items().len(), 0, "повторный load() должен вернуть пустой lock");

    // Восстанавливаем cwd
    std::env::set_current_dir(&original_dir).unwrap();
}

// ── 3. load повреждённого файла → ошибка десериализации ──────────────

#[tokio::test]
async fn load_corrupted_file() {
    // Захватываем мьютекс
    let _cwd = CWD_LOCK.lock().await;
    let _guard = init_test_tracing();

    // 1. Создаём временную директорию и переключаем cwd
    let tmp = TempDir::new().unwrap();
    let original_dir = std::env::current_dir().unwrap();
    std::env::set_current_dir(tmp.path()).unwrap();

    // 2. Создаём директорию `cores/` и записываем невалидный TOML
    std::fs::create_dir_all("cores").unwrap();
    std::fs::write("cores/lock.toml", b"{{{{invalid toml garbage!@#$%").unwrap();

    // 3. Вызываем load() — должен вернуть ошибку десериализации
    let result = CoresLock::load().await;

    // 4. Проверяем что результат — ошибка
    assert!(result.is_err(), "load() повреждённого файла должен вернуть ошибку");

    // 5. Проверяем что ошибка — Serialise (toml::de::Error)
    let err = result.unwrap_err();
    assert!(
        matches!(err, ComposerError::Serialise(_)),
        "ошибка должна быть ComposerError::Serialise, получили: {err:?}"
    );

    // Восстанавливаем cwd
    std::env::set_current_dir(&original_dir).unwrap();
}

// ── 4. remove существующего элемента ─────────────────────────────────

#[tokio::test]
async fn remove_existing_item() {
    // Захватываем мьютекс
    let _cwd = CWD_LOCK.lock().await;
    let _guard = init_test_tracing();

    // 1. Создаём временную директорию и переключаем cwd
    let tmp = TempDir::new().unwrap();
    let original_dir = std::env::current_dir().unwrap();
    std::env::set_current_dir(tmp.path()).unwrap();

    // 2. Создаём директорию `cores/`
    std::fs::create_dir_all("cores").unwrap();

    // 3. Создаём CoresLock с одним элементом
    let hash = fake_hash("abcdef1234567890");
    let lock_item = make_lock_item("test-core", "2.0.0");
    let lock = CoresLock {
        items: LockMap::new(),
    };
    lock.items().insert(hash.clone(), lock_item.clone());

    // 4. Создаём директорию элемента на диске (как если бы он был скачан)
    let dir_path = item_path::<CoresLock>(&hash);
    std::fs::create_dir_all(&dir_path).unwrap();
    std::fs::write(dir_path.join("dummy.bin"), b"some data").unwrap();
    assert!(dir_path.exists(), "директория элемента должна существовать до remove()");

    // 5. Вызываем remove() — должен вернуть Some(item)
    let removed = lock.remove(&hash).await.expect("remove() не должен завершиться ошибкой");
    assert!(removed.is_some(), "remove() должен вернуть Some для существующего элемента");

    // 6. Проверяем что возвращённый элемент совпадает с оригиналом
    let removed_item = removed.unwrap();
    assert_eq!(
        removed_item.item.name, lock_item.item.name,
        "имя удалённого элемента должно совпадать"
    );
    assert_eq!(
        removed_item.item.version, lock_item.item.version,
        "версия удалённого элемента должна совпадать"
    );

    // 7. Проверяем что элемент удалён из DashMap
    assert_eq!(lock.items().len(), 0, "после remove() в lock не должно быть элементов");

    // 8. Проверяем что директория элемента удалена с диска
    assert!(!dir_path.exists(), "директория элемента должна быть удалена после remove()");

    // Восстанавливаем cwd
    std::env::set_current_dir(&original_dir).unwrap();
}

// ── 5. remove несуществующего элемента → None ────────────────────────

#[tokio::test]
async fn remove_nonexistent_item() {
    // Захватываем мьютекс
    let _cwd = CWD_LOCK.lock().await;
    let _guard = init_test_tracing();

    // 1. Создаём временную директорию и переключаем cwd
    let tmp = TempDir::new().unwrap();
    let original_dir = std::env::current_dir().unwrap();
    std::env::set_current_dir(tmp.path()).unwrap();

    // 2. Создаём директорию `cores/`
    std::fs::create_dir_all("cores").unwrap();

    // 3. Создаём пустой CoresLock
    let lock = CoresLock {
        items: LockMap::new(),
    };

    // 4. Пытаемся удалить несуществующий элемент
    let hash = fake_hash("nonexistent_hash_value");
    let removed = lock
        .remove(&hash)
        .await
        .expect("remove() не должен завершиться ошибкой даже для несуществующего элемента");

    // 5. Проверяем что результат — None
    assert!(removed.is_none(), "remove() должен вернуть None для несуществующего элемента");

    // 6. Проверяем что lock по-прежнему пуст
    assert_eq!(
        lock.items().len(),
        0,
        "lock должен оставаться пустым после remove() несуществующего элемента"
    );

    // Восстанавливаем cwd
    std::env::set_current_dir(&original_dir).unwrap();
}

// ── 6. DashMap с #[serde(flatten)] корректно (де)сериализуется ───────

#[tokio::test]
async fn dashmap_serde_roundtrip() {
    // Этот тест НЕ меняет cwd — работает только с сериализацией в памяти
    let _guard = init_test_tracing();

    // 1. Создаём CoresLock с несколькими элементами
    let (map, pairs) = make_lock_map(4);
    let lock = CoresLock {
        items: map,
    };

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
