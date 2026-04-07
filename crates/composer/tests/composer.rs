#[path = "common/mod.rs"]
mod common;

use clients::Clients;
use clients::github::GithubClient;
use composer::Composer;
use composer::lock::instances::InstancesItem;
use composer::paths::AppPaths;
use tempfile::TempDir;

use common::{fake_hash, init_test_tracing, make_lock_item};

/// Создаёт тестовые `Clients` без реального обращения к сети.
fn test_clients() -> Clients {
    let github = GithubClient::new("test".to_owned(), "test".to_owned()).unwrap();
    Clients::new(github)
}

/// Создаёт временную директорию и `AppPaths`, указывающие в неё.
///
/// Возвращает `(TempDir, AppPaths)` — `TempDir` нужно держать живым
/// на протяжении всего теста, иначе директория будет удалена.
fn test_paths() -> (TempDir, AppPaths) {
    let tmp = TempDir::new().unwrap();
    let paths = AppPaths {
        root_dir: tmp.path().to_path_buf(),
        cores_dir: tmp.path().join("cores"),
        instances_dir: tmp.path().join("instances"),
    };
    (tmp, paths)
}

// ── 1. new — пустое состояние ────────────────────────────────────────

#[test]
fn new_creates_empty_state() {
    let _guard = init_test_tracing();
    let (_tmp, paths) = test_paths();

    // 1. Создаём Composer с пустым состоянием
    let composer = Composer::new(test_clients(), paths);

    // 2. Проверяем что коллекция ядер пуста
    assert!(composer.cores_items().is_empty());

    // 3. Проверяем что коллекция инстансов пуста
    assert!(composer.instances_items().is_empty());
}

// ── 2. save → load round-trip ────────────────────────────────────────

#[tokio::test]
async fn save_and_load_roundtrip() {
    let _guard = init_test_tracing();

    // 1. Создаём временную директорию и подкаталоги для lock-файлов
    let (tmp, paths) = test_paths();
    std::fs::create_dir_all(&paths.cores_dir).unwrap();
    std::fs::create_dir_all(&paths.instances_dir).unwrap();

    // 2. Создаём Composer и добавляем элементы в cores
    let composer = Composer::new(test_clients(), paths.clone());
    let hash = fake_hash("core_hash_1");
    let item = make_lock_item("test-core", "1.0.0");
    composer.cores_items().insert(hash.clone(), item.clone());

    // 3. Сохраняем всё на диск
    composer.save().await.expect("save() не должен падать");

    // 4. Загружаем данные с диска в новый Composer
    let loaded = Composer::load(test_clients(), paths).await.expect("load() не должен падать");

    // 5. Проверяем что ядра загрузились корректно
    assert_eq!(loaded.cores_items().len(), 1, "должен быть ровно 1 элемент в cores");
    assert!(
        loaded.cores_items().contains_key(&hash),
        "загруженный lock должен содержать тот же хэш"
    );

    // 6. Проверяем поля загруженного элемента
    let loaded_item = loaded.cores_items().get(&hash).expect("элемент должен существовать");
    assert_eq!(loaded_item.item.name, "test-core");
    assert_eq!(loaded_item.item.version.to_string(), "1.0.0");

    // keep tmp alive until end of test
    drop(tmp);
}

// ── 3. save_cores — сохраняет только cores/lock.toml ─────────────────

#[tokio::test]
async fn save_cores_only() {
    let _guard = init_test_tracing();

    // 1. Создаём временную директорию и подкаталоги
    let (tmp, paths) = test_paths();
    std::fs::create_dir_all(&paths.cores_dir).unwrap();
    std::fs::create_dir_all(&paths.instances_dir).unwrap();

    // 2. Создаём Composer и добавляем элемент в cores
    let composer = Composer::new(test_clients(), paths.clone());
    let hash = fake_hash("only_core");
    let item = make_lock_item("core-only", "2.0.0");
    composer.cores_items().insert(hash.clone(), item);

    // 3. Сохраняем только cores
    composer.save_cores().await.expect("save_cores() не должен падать");

    // 4. Проверяем что файл cores/lock.toml создан
    let cores_lock_path = paths.cores_dir.join("lock.toml");
    assert!(cores_lock_path.exists(), "cores/lock.toml должен быть создан после save_cores()");

    // 5. Проверяем что файл не пустой (содержит сериализованные данные)
    let content = std::fs::read_to_string(&cores_lock_path).unwrap();
    assert!(!content.is_empty(), "cores/lock.toml не должен быть пустым");

    drop(tmp);
}

// ── 4. save_instances — сохраняет только instances/lock.toml ─────────

#[tokio::test]
async fn save_instances_only() {
    let _guard = init_test_tracing();

    // 1. Создаём временную директорию и подкаталоги
    let (tmp, paths) = test_paths();
    std::fs::create_dir_all(&paths.cores_dir).unwrap();
    std::fs::create_dir_all(&paths.instances_dir).unwrap();

    // 2. Создаём Composer и добавляем элемент в instances
    let composer = Composer::new(test_clients(), paths.clone());
    let item = InstancesItem {
        icon: String::from("test-icon"),
        banner: String::from("test-banner"),
        last_launch: None,
        created_at: None,
    };
    composer.instances_items().insert("only_instance".to_owned(), item);

    // 3. Сохраняем только instances
    composer.save_instances().await.expect("save_instances() не должен падать");

    // 4. Проверяем что файл instances/lock.toml создан
    let instances_lock_path = paths.instances_dir.join("lock.toml");
    assert!(
        instances_lock_path.exists(),
        "instances/lock.toml должен быть создан после save_instances()"
    );

    // 5. Проверяем что файл не пустой (содержит сериализованные данные)
    let content = std::fs::read_to_string(&instances_lock_path).unwrap();
    assert!(!content.is_empty(), "instances/lock.toml не должен быть пустым");

    drop(tmp);
}

// ── 5. validate — пустое состояние не порождает ошибок ────────────────

#[tokio::test]
async fn validate_empty_state() {
    let _guard = init_test_tracing();

    // 1. Создаём временную директорию и подкаталоги
    let (tmp, paths) = test_paths();
    std::fs::create_dir_all(&paths.cores_dir).unwrap();
    std::fs::create_dir_all(&paths.instances_dir).unwrap();

    // 2. Создаём Composer с пустым состоянием
    let composer = Composer::new(test_clients(), paths);

    // 3. Валидируем ядра — для пустого состояния список причин должен быть пуст
    let cores_reasons = composer
        .validate_cores()
        .await
        .expect("validate_cores() не должен падать для пустого состояния");
    assert!(cores_reasons.is_empty(), "пустой cores не должен порождать причин невалидности");

    // 4. Валидируем инстансы — аналогично пустой результат
    let instances_reasons = composer
        .validate_instances()
        .await
        .expect("validate_instances() не должен падать для пустого состояния");
    assert!(
        instances_reasons.is_empty(),
        "пустой instances не должен порождать причин невалидности"
    );

    drop(tmp);
}
