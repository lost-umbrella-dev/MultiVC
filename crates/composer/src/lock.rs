use std::path::Path;

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

    /// Возвращает список элементов в lock-файле
    fn items(&self) -> &LockMap;

    /// Загружает lock-файл из диска.
    ///
    /// Если файл не найден — возвращает `Self::default()` (первый запуск) и сохраняет на диске.
    async fn load(lock_file: &Path) -> Result<Self>;

    /// Сохраняет lock-файл на диск
    async fn save(
        &self,
        lock_file: &Path,
    ) -> Result<()> {
        let span = tracing::debug_span!(
            parent: &Self::span(),
            "lock.save",
            file = %lock_file.display(),
            items = self.items().len(),
        );

        let lock_file = lock_file.to_path_buf();

        async move {
            tracing::debug!("saving lock file");

            // Создаём родительские директории, если их ещё нет
            if let Some(parent) = lock_file.parent() {
                tokio::fs::create_dir_all(parent).await?;
            }

            let bytes = toml::to_string_pretty(self)?;
            tokio::fs::write(&lock_file, bytes).await?;
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
        dir: &Path,
        hash: &Hash,
    ) -> Result<Option<LockItem>> {
        let span = tracing::debug_span!(
            parent: &Self::span(),
            "lock.remove",
            hash = %hash,
        );

        let dir = dir.to_path_buf();

        async move {
            let removed = self.items().remove(hash).map(|(_, item)| item);

            if let Some(ref item) = removed {
                tracing::debug!(
                    name = %item.item.name,
                    version = %item.item.version,
                    "item removed from lock",
                );

                let path = crate::utils::hash::item_path(&dir, hash);
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
    async fn validate_dir(
        &self,
        dir: &Path,
    ) -> Result<Vec<ValidateReason>> {
        let span = tracing::info_span!(
            parent: &Self::span(),
            "lock.validate_dir",
            items = self.items().len(),
            parallelism = validate::PARALLELISM,
        );

        let folder = dir.to_path_buf();

        async move {
            let items: Vec<_> = self
                .items()
                .iter()
                .map(|entry| (entry.key().clone(), entry.value().clone()))
                .collect();

            let total = items.len();
            tracing::info!(total, "starting directory validation");

            let results = stream::iter(items.into_iter().map(|(hash, item)| {
                let folder = folder.clone();
                async move {
                    match validate::validate_dir_item(&folder, &hash, &item).await {
                        Ok(None) => Ok::<_, ValidationError>(Ok((hash, item))),
                        Ok(Some(reason)) => Ok(Err(reason)),
                        Err(error) => Err(error),
                    }
                }
            }))
            .buffer_unordered(validate::PARALLELISM)
            .collect::<Vec<_>>()
            .await;

            let mut reasons = Vec::new();
            let mut valid_count = 0usize;
            let mut fatal_errors = Vec::new();

            for result in results {
                match result {
                    Ok(Ok(_)) => {
                        valid_count += 1;
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

            let invalid = reasons.len();
            tracing::info!(valid = valid_count, invalid, total, "directory validation complete",);

            Ok(reasons)
        }
        .instrument(span)
        .await
    }
}
