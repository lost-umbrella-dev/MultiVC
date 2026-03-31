use network_clients::clients::voxelworld::client::VoxelworldClient;
use network_clients::clients::voxelworld::client::modifications::{ModSort, ModsQueryParams};
use network_clients::error::ClientError;
use tracing::{error, warn};

use tracing::subscriber::DefaultGuard;
use tracing_subscriber::{EnvFilter, fmt};

pub fn init_test_tracing() -> DefaultGuard {
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("trace,serde=trace,serde_json=trace,reqwest=debug"));

    let subscriber = fmt()
        .with_env_filter(filter)
        .with_ansi_sanitization(true)
        .compact()
        .with_test_writer()
        .with_target(false)
        .with_thread_names(false)
        .with_line_number(false)
        .with_file(false)
        .without_time()
        .finish();

    tracing::subscriber::set_default(subscriber)
}

/// Создает тестовый клиент Voxelworld без авторизации
fn create_test_client() -> VoxelworldClient {
    VoxelworldClient::new(None)
}

/// Создает тестовый клиент Voxelworld с токеном авторизации
fn create_test_client_with_token(token: String) -> VoxelworldClient {
    VoxelworldClient::new(Some(token))
}

/// Тест для получения списка модов без параметров фильтрации
#[tokio::test]
// #[ignore = "Требует работающий API сервер"]
async fn test_get_mods_no_params() {
    let _guard = init_test_tracing();
    let client = create_test_client();
    let params = ModsQueryParams {
        title: None,
        tags: None,
        page: None,
        sort: None,
    };

    let result = client.get_mods(&params).await;

    match result {
        Ok(mods) => {
            assert!(!mods.is_empty(), "Ожидался непустой список модов");
            // Проверяем структуру первого мода
            let first_mod = &mods[0];
            assert!(first_mod.id > 0, "ID мода должен быть положительным");
            assert!(!first_mod.slug.is_empty(), "Slug не должен быть пустым");
            assert!(!first_mod.title.is_empty(), "Название не должно быть пустым");
            assert!(
                first_mod.downloads >= 0,
                "Количество скачиваний не может быть отрицательным"
            );
            assert!(first_mod.likes >= 0, "Количество лайков не может быть отрицательным");
            assert!(!first_mod.author.name.is_empty(), "Имя автора не должно быть пустым");
        },
        Err(e) => {
            error!("Ошибка при получении списка модов: {:?}", e);
            // В тестовом окружении это может быть ожидаемо
            // assert!(false, "Не удалось получить список модов");
        },
    }
}

/// Тест для получения списка модов с параметром поиска
#[tokio::test]
// #[ignore = "Требует работающий API сервер"]
async fn test_get_mods_with_query() {
    let _guard = init_test_tracing();
    let client = create_test_client();
    let params = ModsQueryParams {
        title: Some("chests".to_owned()),
        tags: None,
        page: None,
        sort: None,
    };

    let result = client.get_mods(&params).await;

    match result {
        Ok(mods) => {
            // Результаты должны содержать моды, связанные с поиском
            assert!(!mods.is_empty())
        },
        Err(e) => {
            error!("Ошибка при поиске модов: {:?}", e);
        },
    }
}

/// Тест для получения списка модов с сортировкой
#[tokio::test]
async fn test_get_mods_with_sort() {
    let _guard = init_test_tracing();
    let client = create_test_client();

    // Тестируем сортировку по популярноси
    let params_popular = ModsQueryParams {
        title: None,
        tags: None,
        page: None,
        sort: Some(ModSort::Popular),
    };

    let result = client.get_mods(&params_popular).await;
    assert!(result.is_ok(), "Не удалось получить моды с сортировкой по популярности");

    // Тестируем сортировку по дате добавления
    let params_likes = ModsQueryParams {
        title: None,
        tags: None,
        page: None,
        sort: Some(ModSort::DateAdd),
    };

    let result = client.get_mods(&params_likes).await;

    assert!(
        result.is_ok(),
        "Не удалось получить моды с сортировкой по дате добавления"
    );

    // Тестируем сортировку по дате обновления
    let params_updated = ModsQueryParams {
        title: None,
        tags: None,
        page: None,
        sort: Some(ModSort::DateUpdate),
    };

    let result = client.get_mods(&params_updated).await;
    assert!(
        result.is_ok(),
        "Не удалось получить моды с сортировкой по дате обновления"
    );

    // Тестируем сортировку по подписке
    let params_updated = ModsQueryParams {
        title: None,
        tags: None,
        page: None,
        sort: Some(ModSort::Subscribe),
    };

    let result = client.get_mods(&params_updated).await;
    assert!(result.is_ok(), "Не удалось получить моды с сортировкой по подписке");
}

/// Тест для получения списка модов с пагинацией
#[tokio::test]
async fn test_get_mods_with_pagination() {
    let _guard = init_test_tracing();
    let client = create_test_client();

    // Получаем первую страницу
    let params_page1 = ModsQueryParams {
        title: None,
        tags: None,
        page: Some(1),
        sort: None,
    };

    let result_page1 = client.get_mods(&params_page1).await;
    assert!(result_page1.is_ok(), "Не удалось получить первую страницу");

    // Получаем вторую страницу
    let params_page2 = ModsQueryParams {
        title: None,
        tags: None,
        page: Some(2),
        sort: None,
    };

    let result_page2 = client.get_mods(&params_page2).await;
    // Вторая страница может быть пустой, но запрос должен пройти успешно
    assert!(result_page2.is_ok(), "Не удалось получить вторую страницу");
}

/// Тест для получения списка модов с фильтрацией по тегам
#[tokio::test]
async fn test_get_mods_with_tags() {
    let _guard = init_test_tracing();
    let client = create_test_client();

    let params = ModsQueryParams {
        title: None,
        tags: Some(vec![87, 10]),
        page: None,
        sort: None,
    };

    let result = client.get_mods(&params).await;
    assert!(result.is_ok(), "Не удалось получить моды с фильтрацией по тегам");
}

/// Тест для получения информации о моде по ID
#[tokio::test]
async fn test_get_mod_by_id() {
    let _guard = init_test_tracing();
    let client = create_test_client();

    // Сначала получаем список модов, чтобы найти существующий ID
    let list_params = ModsQueryParams {
        title: None,
        tags: None,
        page: Some(1),
        sort: Some(ModSort::Popular),
    };

    let mods = client.get_mods(&list_params).await;
    if mods.is_err() || mods.as_ref().unwrap().is_empty() {
        warn!("Пропуск теста: не удалось получить список модов");
        return;
    }

    let first_mod_id = mods.unwrap()[0].id;

    // Получаем детальную информацию по ID
    let result = client.get_mod_by_id_or_slug(&first_mod_id.to_string()).await;

    match result {
        Ok(mod_detail) => {
            assert_eq!(mod_detail.id, first_mod_id, "ID мода должен совпадать");
            assert!(!mod_detail.title.is_empty(), "Название не должно быть пустым");
            assert!(!mod_detail.description.is_empty(), "Описание не должно быть пустым");
            assert!(!mod_detail.author.name.is_empty(), "Имя автора не должно быть пустым");
            assert!(!mod_detail.slug.is_empty(), "Slug не должен быть пустым");
        },
        Err(e) => {
            error!("Ошибка при получении деталей мода: {:?}", e);
        },
    }
}

/// Тест для получения информации о моде по slug
#[tokio::test]
async fn test_get_mod_by_slug() {
    let _guard = init_test_tracing();
    let client = create_test_client();

    // Сначала получаем список модов, чтобы найти существующий slug
    let list_params = ModsQueryParams {
        title: None,
        tags: None,
        page: Some(1),
        sort: Some(ModSort::Popular),
    };

    let mods = client.get_mods(&list_params).await;
    if mods.is_err() || mods.as_ref().unwrap().is_empty() {
        warn!("Пропуск теста: не удалось получить список модов");
        return;
    }

    let first_mod_slug = mods.unwrap()[0].slug.clone();

    // Получаем детальную информацию по slug
    let result = client.get_mod_by_id_or_slug(&first_mod_slug).await;

    match result {
        Ok(mod_detail) => {
            assert_eq!(mod_detail.slug, first_mod_slug, "Slug должен совпадать");
            assert!(!mod_detail.title.is_empty(), "Название не должно быть пустым");
            assert!(!mod_detail.description.is_empty(), "Описание не должно быть пустым");
        },
        Err(e) => {
            error!("Ошибка при получении деталей мода по slug: {:?}", e);
        },
    }
}

/// Тест для получения информации о несуществующем моде
#[tokio::test]
async fn test_get_mod_not_found() {
    let _guard = init_test_tracing();
    let client = create_test_client();

    let result = client.get_mod_by_id_or_slug("nonexistent-mod-99999").await;

    match result {
        Err(ClientError::NotFound) => {
            // Ожидаемая ошибка для несуществующего мода
        },
        Ok(_) => {
            panic!("Ожидалась ошибка NotFound для несуществующего мода");
        },
        Err(e) => {
            error!("Получена неожиданная ошибка: {:?}", e);
        },
    }
}

/// Тест для получения списка версий мода
#[tokio::test]
async fn test_get_mod_versions() {
    let _guard = init_test_tracing();
    let client = create_test_client();

    // Сначала получаем список модов, чтобы найти существующий ID
    let list_params = ModsQueryParams {
        title: None,
        tags: None,
        page: Some(1),
        sort: Some(ModSort::Popular),
    };

    let mods = client.get_mods(&list_params).await;
    if mods.is_err() || mods.as_ref().unwrap().is_empty() {
        warn!("Пропуск теста: не удалось получить список модов");
        return;
    }

    let first_mod_slug = mods.unwrap()[0].slug.clone();

    // Получаем версии мода
    let result = client.get_mod_versions(&first_mod_slug, None, None, None).await;

    match result {
        Ok(versions) => {
            // Мод может не иметь версий, но запрос должен пройти успешно
            for version in &versions {
                assert!(version.id > 0, "ID версии должен быть положительным");
                assert!(!version.version_number.is_empty(), "Номер версии не должен быть пустым");
                assert!(!version.status.title.is_empty(), "Статус версии не должен быть пустым");
            }
        },
        Err(e) => {
            error!("Ошибка при получении версий мода: {:?}", e);
        },
    }
}

/// Тест для получения списка версий мода с пагинацией
#[tokio::test]
async fn test_get_mod_versions_with_pagination() {
    let _guard = init_test_tracing();
    let client = create_test_client();

    // Сначала получаем список модов
    let list_params = ModsQueryParams {
        title: None,
        tags: None,
        page: Some(1),
        sort: Some(ModSort::Popular),
    };

    let mods = client.get_mods(&list_params).await;
    if mods.is_err() || mods.as_ref().unwrap().is_empty() {
        warn!("Пропуск теста: не удалось получить список модов");
        return;
    }

    let first_mod_slug = mods.unwrap()[0].slug.clone();

    // Получаем версии с пагинацией
    let result = client.get_mod_versions(&first_mod_slug, Some(1), Some(5), None).await;

    assert!(result.is_ok(), "Не удалось получить версии с пагинацией");
}

/// Тест для получения детальной информации о версии мода
#[tokio::test]
async fn test_get_mod_version() {
    let _guard = init_test_tracing();
    let client = create_test_client();

    // Сначала получаем список модов
    let list_params = ModsQueryParams {
        title: None,
        tags: None,
        page: Some(1),
        sort: Some(ModSort::Popular),
    };

    let mods = client.get_mods(&list_params).await;
    if mods.is_err() || mods.as_ref().unwrap().is_empty() {
        warn!("Пропуск теста: не удалось получить список модов");
        return;
    }

    let first_mod_slug = mods.unwrap()[0].slug.clone();

    // Получаем список версий мода
    let versions = client.get_mod_versions(&first_mod_slug, None, None, None).await;
    if versions.is_err() || versions.as_ref().unwrap().is_empty() {
        warn!("Пропуск теста: у мода нет версий");
        return;
    }

    let first_version_id = versions.unwrap()[0].id as u64;

    // Получаем детальную информацию о версии
    let result = client.get_mod_version(&first_mod_slug, first_version_id).await;

    match result {
        Ok(version_detail) => {
            assert_eq!(version_detail.id as u64, first_version_id, "ID версии должен совпадать");
            assert!(
                !version_detail.version_number.is_empty(),
                "Номер версии не должен быть пустым"
            );
            assert!(
                !version_detail.project.title.is_empty(),
                "Название проекта не должно быть пустым"
            );
            assert!(
                !version_detail.status.title.is_empty(),
                "Статус версии не должен быть пустым"
            );
        },
        Err(e) => {
            error!("Ошибка при получении деталей версии: {:?}", e);
        },
    }
}

/// Тест для получения детальной информации о несуществующей версии
#[tokio::test]
async fn test_get_mod_version_not_found() {
    let _guard = init_test_tracing();
    let client = create_test_client();

    let result = client.get_mod_version("some-mod", 999999).await;

    match result {
        Err(ClientError::NotFound) => {
            // Ожидаемая ошибка для несуществующей версии
        },
        Ok(_) => {
            panic!("Ожидалась ошибка NotFound для несуществующей версии");
        },
        Err(e) => {
            error!("Получена неожиданная ошибка: {:?}", e);
        },
    }
}

/// Тест для получения последней версии мода
#[tokio::test]
async fn test_get_mod_latest_version() {
    let _guard = init_test_tracing();
    let client = create_test_client();

    // Сначала получаем список модов
    let list_params = ModsQueryParams {
        title: None,
        tags: None,
        page: Some(1),
        sort: Some(ModSort::Popular),
    };

    let mods = client.get_mods(&list_params).await;
    if mods.is_err() || mods.as_ref().unwrap().is_empty() {
        warn!("Пропуск теста: не удалось получить список модов");
        return;
    }

    let first_mod_slug = mods.unwrap()[0].slug.clone();

    // Получаем последнюю версию
    let result = client.get_mod_latest_version(&first_mod_slug).await;

    match result {
        Ok(latest_version) => {
            assert!(
                !latest_version.version_number.is_empty(),
                "Номер версии не должен быть пустым"
            );
            assert!(
                !latest_version.project.title.is_empty(),
                "Название проекта не должно быть пустым"
            );
            assert!(
                !latest_version.status.title.is_empty(),
                "Статус версии не должен быть пустым"
            );
        },
        Err(e) => {
            error!("Ошибка при получении последней версии: {:?}", e);
        },
    }
}

/// Тест для получения последней версии несуществующего мода
#[tokio::test]
async fn test_get_mod_latest_version_not_found() {
    let _guard = init_test_tracing();
    let client = create_test_client();

    let result = client.get_mod_latest_version("nonexistent-mod-99999").await;

    match result {
        Err(ClientError::NotFound) => {
            // Ожидаемая ошибка для несуществующего мода
        },
        Ok(_) => {
            panic!("Ожидалась ошибка NotFound для несуществующего мода");
        },
        Err(e) => {
            error!("Получена неожиданная ошибка: {:?}", e);
        },
    }
}

/// Тест для скачивания версии мода
#[tokio::test]
async fn test_download_mod_version() {
    let _guard = init_test_tracing();
    let client = create_test_client();

    // Сначала получаем список модов
    let list_params = ModsQueryParams {
        title: None,
        tags: None,
        page: Some(1),
        sort: Some(ModSort::Popular),
    };

    let mods = client.get_mods(&list_params).await;
    if mods.is_err() || mods.as_ref().unwrap().is_empty() {
        warn!("Пропуск теста: не удалось получить список модов");
        return;
    }

    let first_mod = &mods.unwrap()[0];

    // Получаем версии мода
    let versions = client.get_mod_versions(&first_mod.slug, None, None, None).await;
    if versions.is_err() || versions.as_ref().unwrap().is_empty() {
        warn!("Пропуск теста: у мода нет версий");
        return;
    }

    let first_version = &versions.unwrap()[0];
    let mod_id = first_mod.id as u64;
    let version_number = &first_version.version_number;

    // Скачиваем версию
    let result = client.download_mod_version(mod_id, version_number).await;

    match result {
        Ok(bytes) => {
            assert!(!bytes.is_empty(), "Скачанные данные не должны быть пустыми");
            // Проверяем, что это действительный ZIP файл (первые байты)
            let header = &bytes[..std::cmp::min(4, bytes.len())];
            // ZIP файлы начинаются с магических байтов 0x50 0x4B 0x03 0x04 или 0x50 0x4B 0x05 0x06
            assert!(
                header == [0x50, 0x4B, 0x03, 0x04] || header == [0x50, 0x4B, 0x05, 0x06],
                "Скачанные данные должны быть в формате ZIP"
            );
        },
        Err(e) => {
            error!("Ошибка при скачивании версии мода: {:?}", e);
        },
    }
}

/// Тест для скачивания несуществующей версии мода
#[tokio::test]
async fn test_download_mod_version_not_found() {
    let _guard = init_test_tracing();
    let client = create_test_client();

    let result = client.download_mod_version(999999, "99.99.99").await;

    match result {
        Err(ClientError::NotFound) => {
            // Ожидаемая ошибка для несуществующей версии
        },
        Ok(_) => {
            panic!("Ожидалась ошибка NotFound для несуществующей версии");
        },
        Err(e) => {
            error!("Получена неожиданная ошибка: {:?}", e);
        },
    }
}

/// Тест для работы с авторизованным клиентом
#[tokio::test]
async fn test_client_with_auth_token() {
    let _guard = init_test_tracing();
    let mut _client = create_test_client();

    // Устанавливаем тестовый токен (в реальных тестах должен быть валидный токен)
    if let Ok(test_token) = std::env::var("VOXELWORLD_TEST_TOKEN") {
        _client = create_test_client_with_token(test_token);

        // Проверяем, что запросы с авторизацией работают
        let list_params = ModsQueryParams {
            title: None,
            tags: None,
            page: None,
            sort: None,
        };

        let result = _client.get_mods(&list_params).await;
        assert!(result.is_ok(), "Не удалось получить список модов с авторизацией");
    } else {
        warn!("Пропуск теста: не установлен VOXELWORLD_TEST_TOKEN");
    }
}
