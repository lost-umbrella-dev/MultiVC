use clients::{
    hash::Hash,
    item::Item,
    prelude::{ClientDownload, ClientGeneral},
};
use futures_util::{StreamExt, stream};
use serde::{Serialize, de::DeserializeOwned};
use tracing::{Instrument, Span};

use crate::{
    error::{ComposerError, Result, ValidationError, ValidationErrors},
    item::{LockItem, LockMap},
    utils::{download, validate},
};

pub mod content;
pub mod core;
pub mod instance;
pub mod instances;

pub enum ValidateReason {
    HashNotMatcher(Hash, LockItem),
    NotFound(Hash, LockItem),
}

#[allow(async_fn_in_trait)]
pub trait Lock
where
    Self: Sized + DeserializeOwned + Serialize,
{
    /// Возвращает scouped-span для логирования
    fn span() -> Span;

    /// Путь к lock-файлу (с включением folder)
    fn file_name() -> &'static std::path::Path;

    /// Путь к папке с lock-файлом и файлами lock-файла
    fn folder_name() -> &'static std::path::Path;

    /// Возвращает список элементов в lock-файле
    fn items(&self) -> &LockMap;

    /// Возвращает мутабельный список элементов в lock-файле
    fn items_mut(&mut self) -> &mut LockMap;

    /// Загружает lock-файл из диска
    async fn load() -> Result<Self> {
        let span = tracing::debug_span!(
            parent: &Self::span(),
            "lock.load",
            file = %Self::file_name().display(),
        );

        async {
            tracing::debug!("loading lock file");
            let bytes = tokio::fs::read(Self::file_name()).await?;
            let lock: Self = toml::from_slice(&bytes)?;
            tracing::debug!(items = lock.items().len(), "lock file loaded");
            Ok(lock)
        }
        .instrument(span)
        .await
    }

    /// Сохраняет lock-файл на диск
    async fn save(&self) -> Result<()> {
        let span = tracing::debug_span!(
            parent: &Self::span(),
            "lock.save",
            file = %Self::file_name().display(),
            items = self.items().len(),
        );

        async {
            tracing::debug!("saving lock file");
            let bytes = toml::to_string_pretty(self)?;
            tokio::fs::write(Self::file_name(), bytes).await?;
            tracing::debug!("lock file saved");
            Ok(())
        }
        .instrument(span)
        .await
    }

    /// Принимает вектор [DownloadRequest], при скачивании идёт сохранение на диск через writer
    /// и параллельно для каждого создаётся digest для последующего сохранения в [LockMap] и валидации.
    ///
    /// Каждый [DownloadRequest] содержит [Item] и опциональный per-item [ProgressSink],
    /// что позволяет отслеживать прогресс каждого скачивания независимо.
    ///
    /// workflow для архивов:
    /// 1. Запрашивает файл во временной дирректории
    /// 2. Скачивает файл в эту дирректорию
    /// 3. Далее распаковываем архив (при скачивании хэш уже проверен)
    /// 4. Создаём хэш распакованной папки
    /// 5. Переносим папку в репозиторий `Lock` файла с именем хэша
    /// 6. Сохраняем в [LockMap]
    ///
    /// Возвращает:
    /// - `Ok(None)`, если все items успешно обработаны;
    /// - `Ok(Some(failures))`, если часть items завершилась ошибкой (с причиной);
    /// - `Err(...)`, если произошла фатальная ошибка batch-уровня.
    async fn download<C>(
        &mut self,
        client: &C,
        requests: Vec<download::DownloadRequest>,
    ) -> Result<Option<Vec<(Item, ComposerError)>>>
    where
        C: ClientDownload + ClientGeneral + Sync,
    {
        let total = requests.len();
        let span = tracing::info_span!(
            parent: &Self::span(),
            "lock.download",
            total,
            parallelism = download::PARALLELISM,
        );

        async {
            tracing::info!(total, "starting batch download");

            tokio::fs::create_dir_all(Self::folder_name()).await?;

            let results = stream::iter(
                requests
                    .into_iter()
                    .map(|request| async move { download::download_item::<Self, C>(client, request).await }),
            )
            .buffer_unordered(download::PARALLELISM)
            .collect::<Vec<_>>()
            .await;

            let mut successful = Vec::new();
            let mut failed = Vec::new();

            for result in results {
                match result {
                    Ok((hash, lock_item)) => successful.push((hash, lock_item)),
                    Err((item, error)) => failed.push((item, error)),
                }
            }

            for (hash, lock_item) in successful.iter() {
                tracing::debug!(
                    name = %lock_item.item.name,
                    version = %lock_item.item.version,
                    hash = %hash,
                    "item downloaded successfully",
                );
            }

            let successful_count = successful.len();
            let failed_count = failed.len();

            for (hash, lock_item) in successful {
                self.items_mut().insert(hash, lock_item);
            }

            if failed.is_empty() {
                tracing::info!(successful = successful_count, "batch download complete — all succeeded");
                Ok(None)
            } else {
                for (item, error) in &failed {
                    tracing::warn!(
                        name = %item.name,
                        version = %item.version,
                        error = %error,
                        "item download failed",
                    );
                }
                tracing::warn!(
                    successful = successful_count,
                    failed = failed_count,
                    "batch download complete — some items failed",
                );
                Ok(Some(failed))
            }
        }
        .instrument(span)
        .await
    }

    /// Проверка папку на наличие данных из lock-файла
    ///
    /// Возвращает список элементов, которые не найдены в папке извлекая их из lock
    ///
    /// Использует хэш из lock-файла для поиска в папке
    async fn validate_dir(&mut self) -> Result<Vec<ValidateReason>> {
        let span = tracing::info_span!(
            parent: &Self::span(),
            "lock.validate_dir",
            items = self.items().len(),
            parallelism = validate::PARALLELISM,
        );

        async {
            let items: Vec<_> = self
                .items()
                .iter()
                .map(|entry| (entry.key().clone(), entry.value().clone()))
                .collect();

            let total = items.len();
            tracing::info!(total, "starting directory validation");

            let results = stream::iter(items.into_iter().map(|(hash, item)| async move {
                match validate::validate_dir_item::<Self>(&hash, &item).await {
                    Ok(None) => Ok::<_, ValidationError>(Ok((hash, item))),
                    Ok(Some(reason)) => Ok(Err(reason)),
                    Err(error) => Err(error),
                }
            }))
            .buffer_unordered(validate::PARALLELISM)
            .collect::<Vec<_>>()
            .await;

            let mut reasons = Vec::new();
            let valid_items = LockMap::new();
            let mut fatal_errors = Vec::new();

            for result in results {
                match result {
                    Ok(Ok((hash, item))) => {
                        valid_items.insert(hash, item);
                    },
                    Ok(Err(reason)) => reasons.push(reason),
                    Err(error) => fatal_errors.push(error),
                }
            }

            if !fatal_errors.is_empty() {
                let count = fatal_errors.len();
                tracing::error!(fatal_errors = count, "validation encountered fatal errors",);
                return Err(ComposerError::Validation(ValidationErrors(fatal_errors)));
            }

            let valid = valid_items.len();
            let invalid = reasons.len();
            tracing::info!(valid, invalid, total, "directory validation complete",);

            *self.items_mut() = valid_items;

            Ok(reasons)
        }
        .instrument(span)
        .await
    }
}
