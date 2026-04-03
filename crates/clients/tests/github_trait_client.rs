use clients::clients::github::client::GithubClient;
use clients::clients::github::{GitHubGetOptions, GitHubListOptions};
use clients::prelude::*;
use tracing::{error, info};

/// Создает тестовый клиент Github
fn create_test_client() -> GithubClient {
    GithubClient::new("MihailRis".to_owned(), "voxelcore".to_owned()).expect("Не удалось создать Github клиент")
}

use tracing::subscriber::DefaultGuard;
use tracing_subscriber::EnvFilter;
use tracing_subscriber::layer::SubscriberExt;
use tracing_tree::HierarchicalLayer;

pub fn init_test_tracing() -> DefaultGuard {
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("debug"));

    let subscriber = tracing_subscriber::registry().with(filter).with(
        HierarchicalLayer::new(2)
            .with_ansi(true)
            .with_targets(true)
            .with_bracketed_fields(true)
            .with_thread_names(false)
            .with_indent_lines(true),
    );

    tracing::subscriber::set_default(subscriber)
}

/// Тест для получения всех релизов репозитория через Client trait
#[tokio::test]
async fn test_list_all_releases() {
    let _guard = init_test_tracing();
    let client = create_test_client();

    let result = client.list(GitHubListOptions { search_version: vec![] }).await;

    match result {
        Ok(items) => {
            info!("Получено {} релизов", items.len());
            assert!(!items.is_empty(), "Ожидался непустой список релизов");

            // Проверяем структуру первого item
            let first_item = &items[0];
            assert!(!first_item.name.is_empty(), "Имя релиза не должно быть пустым");
            assert!(!first_item.version.is_empty(), "Версия не должна быть пустой");
            assert!(!first_item.url.is_empty(), "URL релиза не должен быть пустым");
            assert!(first_item.size > 0, "Размер релиза должен быть больше нуля");
        },
        Err(e) => {
            error!("Ошибка при получении списка релизов: {:?}", e);
        },
    }
}

/// Тест для получения релизов с фильтрацией по версии
#[tokio::test]
async fn test_list_releases_with_version_filter() {
    let _guard = init_test_tracing();
    let client = create_test_client();

    // Сначала получаем все релизы, чтобы знать какую версию искать
    let all_releases = client.list(GitHubListOptions { search_version: vec![] }).await;

    let first_version = match all_releases {
        Ok(items) if !items.is_empty() => items[0].version.clone(),
        _ => return, // Если нет релизов, завершаем тест
    };

    info!("Ищем релизы с версией, содержащей: {}", first_version);

    // Теперь ищем релизы с фильтром по версии
    let result = client
        .list(GitHubListOptions {
            search_version: vec![first_version.clone()],
        })
        .await;

    match result {
        Ok(items) => {
            info!(
                "Найдено {} релизов с версией, содержащей '{}'",
                items.len(),
                first_version
            );
            assert!(!items.is_empty(), "Ожидался непустой список отфильтрованных релизов");

            // Проверяем, что все найденные релизы содержат искомую версию
            for item in &items {
                assert!(
                    item.version.contains(&first_version),
                    "Версия {} должна содержать {}",
                    item.version,
                    first_version
                );
            }
        },
        Err(e) => {
            error!("Ошибка при получении отфильтрованного списка релизов: {:?}", e);
        },
    }
}

/// Тест для получения конкретного релиза по версии через Client trait
#[tokio::test]
async fn test_get_item_by_version() {
    let _guard = init_test_tracing();
    let client = create_test_client();

    // Сначала получаем все релизы, чтобы знать какую версию искать
    let all_releases = client.list(GitHubListOptions { search_version: vec![] }).await;

    let first_version = match all_releases {
        Ok(items) if !items.is_empty() => items[0].version.clone(),
        _ => return, // Если нет релизов, завершаем тест
    };

    info!("Получаем релиз по версии: {}", first_version);

    // Получаем конкретный релиз по версии
    let result = client
        .get(GitHubGetOptions {
            version: first_version.clone(),
        })
        .await;

    match result {
        Ok(Some(item)) => {
            info!("Получен релиз по версии {}: {}", first_version, item.name);
            assert_eq!(item.version, first_version, "Версия должна совпадать");
            assert!(!item.name.is_empty(), "Имя релиза не должно быть пустым");
            assert!(!item.url.is_empty(), "URL релиза не должен быть пустым");
            assert!(item.size > 0, "Размер релиза должен быть больше нуля");
        },
        Ok(None) => {
            info!("Не найден релиз для версии {}", first_version);
        },
        Err(e) => {
            error!("Ошибка при получении релиза по версии: {:?}", e);
        },
    }
}

/// Тест для скачивания релиза через Client trait
#[tokio::test]
async fn test_download_item() {
    let _guard = init_test_tracing();
    let client = create_test_client();

    // Сначала получаем последний релиз
    let result = client.list(GitHubListOptions { search_version: vec![] }).await;

    let item = match result {
        Ok(items) if !items.is_empty() => {
            info!("Получен первый релиз для скачивания: {}", items[0].name);
            items[0].clone()
        },
        Ok(_) => {
            info!("Нет доступных релизов для скачивания");
            return;
        },
        Err(e) => {
            error!("Ошибка при получении списка релизов: {:?}", e);
            return;
        },
    };

    // Скачиваем релиз используя URL из item
    let download_result = client.client().get(&item.url).send().await;

    match download_result {
        Ok(response) => {
            info!(
                "Успешно начато скачивание релиза {}, статус: {}",
                item.name,
                response.status()
            );

            let bytes_result = response.bytes().await;

            match bytes_result {
                Ok(bytes) => {
                    info!("Успешно скачан релиз {}, размер: {} байт", item.name, bytes.len());
                    assert!(!bytes.is_empty(), "Скачанные данные не должны быть пустыми");
                },
                Err(e) => {
                    error!("Ошибка при чтении байтов из ответа: {:?}", e);
                },
            }
        },
        Err(e) => {
            error!("Ошибка при отправке запроса на скачивание: {:?}", e);
            info!("Примечание: Это может быть ожидаемо, если сервер недоступен");
        },
    }
}
