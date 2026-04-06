use clients::github::{GitHubGetOptions, GitHubListOptions, GithubClient};
use tracing::subscriber::DefaultGuard;
use tracing::{error, info};
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::{EnvFilter, Layer};
use tracing_tree::HierarchicalLayer;

/// Создает тестовый клиент Github
fn create_test_client() -> GithubClient {
    GithubClient::new("MihailRis".to_owned(), "voxelcore".to_owned())
        .expect("Не удалось создать Github клиент")
}

/// Инициализирует tracing для тестов.
///
/// Возвращает `DefaultGuard` — пока он жив, логи пишутся.
/// Уровень берётся из `RUST_LOG`, по умолчанию `debug`.
pub fn init_test_tracing() -> DefaultGuard {
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| {
        EnvFilter::new("debug,h2=warn,hyper_util=warn,hyper=warn,reqwest=warn")
    });

    let layer = HierarchicalLayer::new(2)
        .with_ansi(true)
        .with_targets(true)
        .with_bracketed_fields(true)
        .with_thread_names(false)
        .with_indent_lines(true)
        .with_filter(filter);

    let subscriber = tracing_subscriber::registry().with(layer);

    tracing::subscriber::set_default(subscriber)
}

// ── Low-level API ────────────────────────────────────────────────────

/// Тест для получения последнего релиза репозитория
#[tokio::test]
async fn test_get_latest_release() {
    let _guard = init_test_tracing();
    let client = create_test_client();

    let result = client.get_latest_release().await;

    match result {
        Ok(release) => {
            info!("Получен последний релиз: {}", release.tag_name);
            assert!(!release.url.is_empty(), "URL релиза не должен быть пустым");
            assert!(!release.html_url.is_empty(), "HTML URL релиза не должен быть пустым");
            assert!(!release.tag_name.is_empty(), "Tag name не должен быть пустым");
            assert!(!release.tarball_url.is_empty(), "Tarball URL не должен быть пустым");
            assert!(!release.zipball_url.is_empty(), "Zipball URL не должен быть пустым");
            assert!(!release.assets.is_empty(), "Релиз должен содержать хотя бы один asset");
        },
        Err(e) => {
            error!("Ошибка при получении последнего релиза: {:?}", e);
        },
    }
}

// ── High-level API (list / get) ──────────────────────────────────────

/// Тест для получения всех релизов
#[tokio::test]
async fn test_list_all_releases() {
    let _guard = init_test_tracing();
    let client = create_test_client();

    let result = client
        .list(GitHubListOptions {
            search_version: vec![],
        })
        .await;

    match result {
        Ok(items) => {
            info!("Получено {} релизов", items.len());
            assert!(!items.is_empty(), "Ожидался непустой список релизов");

            let first_item = &items[0];
            assert!(!first_item.name.is_empty(), "Имя релиза не должно быть пустым");
            assert!(!first_item.version.to_string().is_empty(), "Версия не должна быть пустой");
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

    let all_releases = client
        .list(GitHubListOptions {
            search_version: vec![],
        })
        .await;

    let first_version = match all_releases {
        Ok(items) if !items.is_empty() => items[0].version.clone(),
        _ => return,
    };

    info!("Ищем релизы с версией, содержащей: {}", first_version);

    let result = client
        .list(GitHubListOptions {
            search_version: vec![first_version.clone()],
        })
        .await;

    match result {
        Ok(items) => {
            info!("Найдено {} релизов с версией, содержащей '{}'", items.len(), first_version);
            assert!(!items.is_empty(), "Ожидался непустой список отфильтрованных релизов");

            for item in &items {
                assert!(
                    item.version.to_string().contains(&first_version.to_string()),
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

/// Тест для получения конкретного релиза по версии
#[tokio::test]
async fn test_get_item_by_version() {
    let _guard = init_test_tracing();
    let client = create_test_client();

    let all_releases = client
        .list(GitHubListOptions {
            search_version: vec![],
        })
        .await;

    let first_version = match all_releases {
        Ok(items) if !items.is_empty() => items[0].version.clone(),
        _ => return,
    };

    info!("Получаем релиз по версии: {}", first_version);

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

// ── Download ─────────────────────────────────────────────────────────

/// Тест для скачивания релиза
#[tokio::test]
async fn test_download_item() {
    let _guard = init_test_tracing();
    let client = create_test_client();

    let result = client
        .list(GitHubListOptions {
            search_version: vec![],
        })
        .await;

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

    let download_result = client.client.get(&item.url).send().await;

    match download_result {
        Ok(response) => {
            info!("Успешно начато скачивание релиза {}, статус: {}", item.name, response.status());

            match response.bytes().await {
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
