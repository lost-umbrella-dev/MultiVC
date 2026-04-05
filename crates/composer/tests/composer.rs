#[path = "common/mod.rs"]
mod common;

use tokio::sync::Mutex;

use clients::Clients;
use clients::github::GithubClient;
use composer::Composer;
use composer::lock::instances::InstancesItem;
use tempfile::TempDir;

use common::{fake_hash, init_test_tracing, make_lock_item};

/// Глобальный мьютекс для сериализации тестов, меняющих cwd.
///
/// Lock-файлы используют жёстко заданные относительные пути (`cores/lock.toml`,
/// `instances/lock.toml`), поэтому перед записью/чтением мы переключаем cwd
/// во временную директорию. Чтобы параллельные тесты не конфликтовали — берём мьютекс.
static CWD_LOCK: Mutex<()> = Mutex::const_new(());

/// Создаёт тестовые `Clients` без реального обращения к сети.
///
/// `GithubClient::new` только собирает `reqwest::Client` — HTTP-запросы
/// не выполняются до явного вызова.
fn test_clients() -> Clients {
    let github = GithubClient::new("test".to_owned(), "test".to_owned()).unwrap();
    Clients::new(github)
}

// ── 1. new — пустое состояние ────────────────────────────────────────

#[test]
fn new_creates_empty_state() {
    let _guard = init_test_tracing();

    // 1. Создаём Composer с пустым состоянием
    let composer = Composer::new(test_clients());

    // 2. Проверяем что коллекция ядер пуста
    assert!(composer.cores_items().is_empty());

    // 3. Проверяем что коллекция инстансов пуста
    assert!(composer.instances_items().is_empty());
}

// ── 2. save → load round-trip ────────────────────────────────────────

#[tokio::test]
async fn save_and_load_roundtrip() {
    let _guard = init_test_tracing();
    let _cwd = CWD_LOCK.lock().await;

    // 1. Запоминаем исходную рабочую директорию
    let original_dir = std::env::current_dir().expect("не удалось получить текущую директорию");

    // 2. Создаём временную директорию и подкаталоги для lock-файлов
    let tmp = TempDir::new().expect("не удалось создать временную директорию");
    std::fs::create_dir_all(tmp.path().join("cores")).unwrap();
    std::fs::create_dir_all(tmp.path().join("instances")).unwrap();

    // 3. Переключаем cwd во временную директорию
    std::env::set_current_dir(tmp.path()).expect("не удалось сменить cwd");

    // 4. Создаём Composer и добавляем элементы в cores
    let composer = Composer::new(test_clients());
    let hash = fake_hash("core_hash_1");
    let item = make_lock_item("test-core", "1.0.0");
    composer.cores_items().insert(hash.clone(), item.clone());

    // 5. Сохраняем всё на диск
    composer.save().await.expect("save() не должен падать");

    // 6. Загружаем данные с диска в новый Composer
    let loaded = Composer::load(test_clients()).await.expect("load() не должен падать");

    // 7. Проверяем что ядра загрузились корректно
    assert_eq!(loaded.cores_items().len(), 1, "должен быть ровно 1 элемент в cores");
    assert!(
        loaded.cores_items().contains_key(&hash),
        "загруженный lock должен содержать тот же хэш"
    );

    // 8. Проверяем поля загруженного элемента
    let loaded_item = loaded.cores_items().get(&hash).expect("элемент должен существовать");
    assert_eq!(loaded_item.item.name, "test-core");
    assert_eq!(loaded_item.item.version, "1.0.0");

    // 9. Восстанавливаем исходную рабочую директорию
    std::env::set_current_dir(&original_dir).expect("не удалось восстановить cwd");
}

// ── 3. save_cores — сохраняет только cores/lock.toml ─────────────────

#[tokio::test]
async fn save_cores_only() {
    let _guard = init_test_tracing();
    let _cwd = CWD_LOCK.lock().await;

    // 1. Запоминаем исходную рабочую директорию
    let original_dir = std::env::current_dir().expect("не удалось получить текущую директорию");

    // 2. Создаём временную директорию и подкаталоги
    let tmp = TempDir::new().expect("не удалось создать временную директорию");
    std::fs::create_dir_all(tmp.path().join("cores")).unwrap();
    std::fs::create_dir_all(tmp.path().join("instances")).unwrap();

    // 3. Переключаем cwd во временную директорию
    std::env::set_current_dir(tmp.path()).expect("не удалось сменить cwd");

    // 4. Создаём Composer и добавляем элемент в cores
    let composer = Composer::new(test_clients());
    let hash = fake_hash("only_core");
    let item = make_lock_item("core-only", "2.0.0");
    composer.cores_items().insert(hash.clone(), item);

    // 5. Сохраняем только cores
    composer.save_cores().await.expect("save_cores() не должен падать");

    // 6. Проверяем что файл cores/lock.toml создан
    let cores_lock_path = tmp.path().join("cores/lock.toml");
    assert!(
        cores_lock_path.exists(),
        "cores/lock.toml должен быть создан после save_cores()"
    );

    // 7. Проверяем что файл не пустой (содержит сериализованные данные)
    let content = std::fs::read_to_string(&cores_lock_path).unwrap();
    assert!(!content.is_empty(), "cores/lock.toml не должен быть пустым");

    // 8. Восстанавливаем исходную рабочую директорию
    std::env::set_current_dir(&original_dir).expect("не удалось восстановить cwd");
}

// ── 4. save_instances — сохраняет только instances/lock.toml ─────────

#[tokio::test]
async fn save_instances_only() {
    let _guard = init_test_tracing();
    let _cwd = CWD_LOCK.lock().await;

    // 1. Запоминаем исходную рабочую директорию
    let original_dir = std::env::current_dir().expect("не удалось получить текущую директорию");

    // 2. Создаём временную директорию и подкаталоги
    let tmp = TempDir::new().expect("не удалось создать временную директорию");
    std::fs::create_dir_all(tmp.path().join("cores")).unwrap();
    std::fs::create_dir_all(tmp.path().join("instances")).unwrap();

    // 3. Переключаем cwd во временную директорию
    std::env::set_current_dir(tmp.path()).expect("не удалось сменить cwd");

    // 4. Создаём Composer и добавляем элемент в instances
    let composer = Composer::new(test_clients());
    let item = InstancesItem {
        icon: String::from("test-icon"),
        banner: String::from("test-banner"),
    };
    composer.instances_items().insert("only_instance".to_owned(), item);

    // 5. Сохраняем только instances
    composer
        .save_instances()
        .await
        .expect("save_instances() не должен падать");

    // 6. Проверяем что файл instances/lock.toml создан
    let instances_lock_path = tmp.path().join("instances/lock.toml");
    assert!(
        instances_lock_path.exists(),
        "instances/lock.toml должен быть создан после save_instances()"
    );

    // 7. Проверяем что файл не пустой (содержит сериализованные данные)
    let content = std::fs::read_to_string(&instances_lock_path).unwrap();
    assert!(!content.is_empty(), "instances/lock.toml не должен быть пустым");

    // 8. Восстанавливаем исходную рабочую директорию
    std::env::set_current_dir(&original_dir).expect("не удалось восстановить cwd");
}

// ── 5. validate — пустое состояние не порождает ошибок ────────────────

#[tokio::test]
async fn validate_empty_state() {
    let _guard = init_test_tracing();
    let _cwd = CWD_LOCK.lock().await;

    // 1. Запоминаем исходную рабочую директорию
    let original_dir = std::env::current_dir().expect("не удалось получить текущую директорию");

    // 2. Создаём временную директорию и подкаталоги
    let tmp = TempDir::new().expect("не удалось создать временную директорию");
    std::fs::create_dir_all(tmp.path().join("cores")).unwrap();
    std::fs::create_dir_all(tmp.path().join("instances")).unwrap();

    // 3. Переключаем cwd во временную директорию
    std::env::set_current_dir(tmp.path()).expect("не удалось сменить cwd");

    // 4. Создаём Composer с пустым состоянием
    let composer = Composer::new(test_clients());

    // 5. Валидируем ядра — для пустого состояния список причин должен быть пуст
    let cores_reasons = composer
        .validate_cores()
        .await
        .expect("validate_cores() не должен падать для пустого состояния");
    assert!(
        cores_reasons.is_empty(),
        "пустой cores не должен порождать причин невалидности"
    );

    // 6. Валидируем инстансы — аналогично пустой результат
    let instances_reasons = composer
        .validate_instances()
        .await
        .expect("validate_instances() не должен падать для пустого состояния");
    assert!(
        instances_reasons.is_empty(),
        "пустой instances не должен порождать причин невалидности"
    );

    // 7. Восстанавливаем исходную рабочую директорию
    std::env::set_current_dir(&original_dir).expect("не удалось восстановить cwd");
}
