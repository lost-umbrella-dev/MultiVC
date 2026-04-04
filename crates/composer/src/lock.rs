use clients::{
    hash::Hash,
    item::Item,
    prelude::{ClientDownload, ClientGeneral},
};
use futures_util::{StreamExt, stream};
use serde::{Serialize, de::DeserializeOwned};
use tracing::Span;

use crate::{
    error::{Result, ValidationError, ValidationErrors},
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
        let bytes = tokio::fs::read(Self::file_name()).await?;
        Ok(toml::from_slice(&bytes)?)
    }

    /// Сохраняет lock-файл на диск
    async fn save(&self) -> Result<()> {
        let bytes = toml::to_string_pretty(self)?;
        tokio::fs::write(Self::file_name(), bytes).await?;
        Ok(())
    }

    /// Принимает слайс из [Item], при скачивании идёт сохранение на диск через writer и параллельно
    /// для каждого создаётся digest для последующего сохранения в [LockMap] и валидации
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
    /// - `Ok(Some(items))`, если часть items завершилась ошибкой;
    /// - `Err(...)`, если произошла фатальная ошибка batch-уровня.
    async fn download<C>(&mut self, client: &C, items: &[Item]) -> Result<Option<Vec<Item>>>
    where
        C: ClientDownload + ClientGeneral + Sync,
    {
        tokio::fs::create_dir_all(Self::folder_name()).await?;

        let results = stream::iter(
            items
                .iter()
                .map(|item| async move { download::download_item::<Self, C>(client, item.clone()).await }),
        )
        .buffer_unordered(download::PARALLELISM)
        .collect::<Vec<_>>()
        .await;

        let mut successful = Vec::new();
        let mut failed = Vec::new();

        for result in results {
            match result {
                Ok((hash, lock_item)) => successful.push((hash, lock_item)),
                Err(item) => failed.push(item),
            }
        }

        for (hash, lock_item) in successful {
            self.items_mut().insert(hash, lock_item);
        }

        if failed.is_empty() { Ok(None) } else { Ok(Some(failed)) }
    }

    /// Проверка папку на наличие данных из lock-файла
    ///
    /// Возвращает список элементов, которые не найдены в папке извлекая их из lock
    ///
    /// Использует хэш из lock-файла для поиска в папке
    async fn validate_dir(&mut self) -> Result<Vec<ValidateReason>> {
        let items: Vec<_> = self
            .items()
            .iter()
            .map(|entry| (entry.key().clone(), entry.value().clone()))
            .collect();

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
            return Err(ValidationErrors(fatal_errors).into());
        }

        *self.items_mut() = valid_items;

        Ok(reasons)
    }
}
