use clients::hash::Hash;
use futures_util::{StreamExt, stream};
use serde::{Serialize, de::DeserializeOwned};
use tracing::{Instrument, Span};

use crate::{
    error::{ComposerError, Result, ValidationError, ValidationErrors},
    item::{LockItem, LockMap},
    utils::validate,
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
    Self: Sized + DeserializeOwned + Serialize + Default,
{
    /// Возвращает scouped-span для логирования
    fn span() -> Span;

    /// Путь к lock-файлу (с включением folder)
    fn file_name() -> &'static std::path::Path;

    /// Путь к папке с lock-файлом и файлами lock-файла
    fn folder_name() -> &'static std::path::Path;

    /// Возвращает список элементов в lock-файле
    fn items(&self) -> &LockMap;

    /// Загружает lock-файл из диска.
    ///
    /// Если файл не найден — возвращает `Self::default()` (первый запуск) и сохраняет на диске.
    async fn load() -> Result<Self> {
        let span = tracing::debug_span!(
            parent: &Self::span(),
            "lock.load",
            file = %Self::file_name().display(),
        );

        async {
            tracing::debug!("loading lock file");
            match tokio::fs::read(Self::file_name()).await {
                Ok(bytes) => {
                    let lock: Self = toml::from_slice(&bytes)?;
                    tracing::debug!(items = lock.items().len(), "lock file loaded");
                    Ok(lock)
                },
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                    tracing::debug!("lock file not found, creating default");
                    let lock = Self::default();
                    lock.save().await?;
                    Ok(lock)
                },
                Err(e) => Err(e.into()),
            }
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

            // Создаём родительские директории, если их ещё нет
            if let Some(parent) = Self::file_name().parent() {
                tokio::fs::create_dir_all(parent).await?;
            }

            let bytes = toml::to_string_pretty(self)?;
            tokio::fs::write(Self::file_name(), bytes).await?;
            tracing::debug!("lock file saved");
            Ok(())
        }
        .instrument(span)
        .await
    }

    /// Удаляет элемент из lock-файла и его директорию с диска.
    ///
    /// Возвращает удалённый элемент, или `None` если элемент не найден.
    /// Lock-файл **не** сохраняется — вызывающий код решает когда вызвать `save()`.
    async fn remove(
        &self,
        hash: &Hash,
    ) -> Result<Option<LockItem>> {
        let span = tracing::debug_span!(
            parent: &Self::span(),
            "lock.remove",
            hash = %hash,
        );

        async {
            let removed = self.items().remove(hash).map(|(_, item)| item);

            if let Some(ref item) = removed {
                tracing::debug!(
                    name = %item.item.name,
                    version = %item.item.version,
                    "item removed from lock",
                );

                let path = crate::utils::hash::item_path::<Self>(hash);
                if tokio::fs::try_exists(&path).await? {
                    tokio::fs::remove_dir_all(&path).await?;
                    tracing::debug!(path = %path.display(), "directory removed");
                }
            } else {
                tracing::debug!("item not found in lock");
            }

            Ok(removed)
        }
        .instrument(span)
        .await
    }

    /// Проверка папку на наличие данных из lock-файла
    ///
    /// Возвращает список элементов, которые не найдены в папке извлекая их из lock
    ///
    /// Использует хэш из lock-файла для поиска в папке
    async fn validate_dir(&self) -> Result<Vec<ValidateReason>> {
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

            // Replace items atomically: clear old entries and insert validated ones.
            // DashMap supports interior mutability, so &self is sufficient.
            self.items().clear();
            for entry in valid_items.into_iter() {
                self.items().insert(entry.0, entry.1);
            }

            Ok(reasons)
        }
        .instrument(span)
        .await
    }
}
