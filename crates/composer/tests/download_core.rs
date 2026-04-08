//! Интеграционный тест: полный workflow скачивания ядра.
//!
//! Получает список доступных версий с GitHub, берёт последнюю,
//! прогоняет через `Composer::install_cores` (download → extract → rename → hash → commit),
//! затем проверяет результат: элемент в lock, директория на диске,
//! исполняемый файл переименован в `core.{ext}`, валидация проходит.
//!
//! **Требования:**
//! - Доступ к сети (GitHub API + CDN).
//! - Запуск: `cargo test --package composer --test download_core -- --ignored`
//!   (тест помечен `#[ignore]`, т.к. зависит от сети).

#[path = "common/mod.rs"]
mod common;

use clients::Clients;
use clients::github::{GitHubListOptions, GithubClient};
use composer::paths::AppPaths;
use composer::progress::ProgressBridge;
use composer::{Composer, DownloadRequest};
use tempfile::TempDir;
use tracing::info;

use common::init_test_tracing;

/// Создаёт реальный `GithubClient` для репозитория VoxelCore.
fn real_clients() -> Clients {
    let github = GithubClient::new("MihailRis".to_owned(), "voxelcore".to_owned())
        .expect("не удалось создать GithubClient");
    Clients::new(github)
}

/// Создаёт временную директорию и `AppPaths`, указывающие в неё.
fn test_paths(tmp: &TempDir) -> AppPaths {
    AppPaths {
        root_dir: tmp.path().to_path_buf(),
        cores_dir: tmp.path().join("cores"),
        instances_dir: tmp.path().join("instances"),
    }
}

/// Каноническое имя исполняемого файла ядра для текущей ОС.
fn core_executable_name() -> String {
    let ext = match std::env::consts::OS {
        "windows" => "exe",
        "linux" => "AppImage",
        "macos" => "dmg",
        _ => "bin",
    };
    format!("core.{ext}")
}

// ── Полный pipeline: list → install → verify ─────────────────────────

#[tokio::test]
#[ignore = "требует доступа к сети (GitHub API + CDN)"]
async fn install_latest_core_full_pipeline() {
    let _guard = init_test_tracing();

    // 1. Создаём временную директорию и подкаталоги для lock-файлов
    let tmp = TempDir::new().expect("не удалось создать временную директорию");
    let paths = test_paths(&tmp);
    std::fs::create_dir_all(&paths.cores_dir).unwrap();
    std::fs::create_dir_all(&paths.instances_dir).unwrap();
    info!(dir = %tmp.path().display(), "рабочая директория теста");

    // 2. Получаем список всех доступных версий ядер
    let clients = real_clients();
    info!("получаем список доступных версий ядер...");
    let items = clients
        .core
        .list(GitHubListOptions {
            search_version: vec![],
        })
        .await
        .expect("не удалось получить список релизов с GitHub");

    info!(count = items.len(), "получено версий ядер");

    // 3. Берём последнюю (первую в списке) версию — если для текущей ОС нет ассета, пропускаем тест
    if items.is_empty() {
        info!(os = std::env::consts::OS, "нет подходящего ассета для текущей ОС — тест пропущен");
        return;
    }
    let latest = items.into_iter().next().expect("список релизов пуст");
    info!(
        name = %latest.name,
        version = %latest.version,
        size = latest.size,
        url = %latest.url,
        "выбрана последняя версия для скачивания"
    );

    // 4. Создаём ProgressBridge для отслеживания прогресса скачивания
    let bridge = ProgressBridge::noop();
    let request = DownloadRequest::with_progress(latest.clone(), bridge.sink());

    // 5. Создаём Composer и запускаем install_cores
    let composer = Composer::new(real_clients(), paths.clone());
    info!("запускаем install_cores...");
    let result = composer
        .install_cores(vec![request])
        .await
        .expect("install_cores завершился с фатальной ошибкой");

    // 6. Проверяем что установка прошла без ошибок
    assert!(
        result.is_none(),
        "install_cores вернул ошибки: {:?}",
        result.map(|failures| failures
            .iter()
            .map(|(item, err)| format!("{}: {err}", item.name))
            .collect::<Vec<_>>())
    );
    info!("install_cores завершился успешно");

    // 7. Проверяем что в cores_items появился ровно один элемент
    assert_eq!(
        composer.cores_items().len(),
        1,
        "после установки в cores должен быть ровно 1 элемент"
    );

    // 8. Извлекаем хэш и lock-item установленного ядра
    let entry =
        composer.cores_items().iter().next().expect("cores_items пуст после успешной установки");
    let hash = entry.key().clone();
    let lock_item = entry.value().clone();
    drop(entry);

    info!(
        hash = %hash,
        name = %lock_item.item.name,
        version = %lock_item.item.version,
        "элемент зарегистрирован в lock"
    );

    // 9. Проверяем что метаданные совпадают с исходным Item
    assert_eq!(lock_item.item.name, latest.name, "имя в lock должно совпадать с исходным");
    assert_eq!(lock_item.item.version, latest.version, "версия в lock должна совпадать с исходной");

    // 10. Проверяем что директория ядра существует на диске
    let core_dir = composer::utils::hash::item_path(&paths.cores_dir, &hash);
    assert!(core_dir.exists(), "директория ядра должна существовать: {}", core_dir.display());
    assert!(core_dir.is_dir(), "путь ядра должен быть директорией: {}", core_dir.display());
    info!(path = %core_dir.display(), "директория ядра на диске");

    // 11. Проверяем что исполняемый файл переименован в core.{ext}
    let executable_name = core_executable_name();
    let executable_path = core_dir.join(&executable_name);
    assert!(
        executable_path.exists(),
        "исполняемый файл `{executable_name}` должен существовать в {}",
        core_dir.display()
    );
    assert!(executable_path.is_file(), "`{executable_name}` должен быть файлом, а не директорией");

    // 12. Проверяем что файл не пустой
    let metadata = std::fs::metadata(&executable_path)
        .expect("не удалось получить метаданные исполняемого файла");
    assert!(metadata.len() > 0, "исполняемый файл `{executable_name}` не должен быть пустым");
    info!(
        file = %executable_name,
        size = metadata.len(),
        "исполняемый файл на месте"
    );

    // 13. Проверяем что ProgressBridge зафиксировал прогресс
    let downloaded = bridge.downloaded();
    info!(downloaded_bytes = downloaded, "прогресс скачивания");
    assert!(downloaded > 0, "ProgressBridge должен зафиксировать скачанные байты");

    // 14. Сохраняем lock на диск и загружаем обратно — проверяем persistence
    composer.save_cores().await.expect("save_cores() не должен падать");
    info!("lock ядер сохранён на диск");

    let lock_path = paths.cores_dir.join("lock.toml");
    assert!(lock_path.exists(), "cores/lock.toml должен существовать после save");

    let lock_content = std::fs::read_to_string(&lock_path).unwrap();
    assert!(!lock_content.is_empty(), "cores/lock.toml не должен быть пустым");
    info!(lock_size = lock_content.len(), "lock-файл записан");

    // 15. Валидируем — хэш директории должен совпадать с записанным в lock
    let reasons = composer.validate_cores().await.expect("validate_cores() не должен падать");
    assert!(
        reasons.is_empty(),
        "валидация ядер не должна выявить проблем, но найдено: {} причин",
        reasons.len()
    );
    info!("валидация ядер прошла успешно");
    info!("тест завершён успешно");
}
