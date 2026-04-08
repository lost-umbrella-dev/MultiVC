use std::path::Path;

use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use tracing::{Instrument, Span};

use crate::error::Result;

/// Concurrent map: instance name → metadata.
pub type InstancesMap = DashMap<String, InstancesItem>;

/// Метаданные инстанса для UI (хранятся в lock-файле).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstancesItem {
    /// Иконка (base64)
    pub icon: String,
    /// Баннер (base64)
    pub banner: String,
    /// Время последнего запуска инстанса.
    #[serde(default)]
    pub last_launch: Option<chrono::DateTime<chrono::Utc>>,
    /// Время создания инстанса.
    #[serde(default)]
    pub created_at: Option<chrono::DateTime<chrono::Utc>>,
}

/// Причина невалидности инстанса.
pub enum InstanceValidateReason {
    /// Папка инстанса не найдена на диске.
    NotFound(String, InstancesItem),
    /// Конфигурация `instance.toml` отсутствует в папке.
    ConfigMissing(String, InstancesItem),
}

/// Lock-файл инстансов.
///
/// Хранит UI-метаданные (иконка, баннер) для каждого инстанса.
/// Ключ — уникальное имя инстанса (оно же имя папки на диске).
///
/// **Не** реализует трейт `Lock`, потому что валидация инстансов
/// проверяет наличие папки по имени, а не по хэшу содержимого.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct InstancesLock {
    #[serde(flatten)]
    pub items: InstancesMap,
}

impl InstancesLock {
    fn span() -> Span {
        tracing::info_span!("lock", r#type = "instances")
    }

    /// Прямой доступ к элементам.
    pub fn items(&self) -> &InstancesMap {
        &self.items
    }

    /// Загружает lock-файл с диска.
    ///
    /// Если файл не найден — возвращает `Self::default()` и сохраняет на диск.
    pub async fn load(lock_file: &Path) -> Result<Self> {
        let span = tracing::debug_span!(
            parent: &Self::span(),
            "lock.load",
            file = %lock_file.display(),
        );

        let lock_file = lock_file.to_path_buf();

        async move {
            tracing::debug!("loading instances lock file");
            match tokio::fs::read(&lock_file).await {
                Ok(bytes) => {
                    let lock: Self = toml::from_slice(&bytes)?;
                    tracing::debug!(items = lock.items.len(), "instances lock loaded");
                    Ok(lock)
                },
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                    tracing::debug!("instances lock not found, creating default");
                    let lock = Self::default();
                    lock.save(&lock_file).await?;
                    Ok(lock)
                },
                Err(e) => Err(e.into()),
            }
        }
        .instrument(span)
        .await
    }

    /// Сохраняет lock-файл на диск.
    pub async fn save(
        &self,
        lock_file: &Path,
    ) -> Result<()> {
        let span = tracing::debug_span!(
            parent: &Self::span(),
            "lock.save",
            file = %lock_file.display(),
            items = self.items.len(),
        );

        let lock_file = lock_file.to_path_buf();

        async move {
            tracing::debug!("saving instances lock file");

            if let Some(parent) = lock_file.parent() {
                tokio::fs::create_dir_all(parent).await?;
            }

            let bytes = toml::to_string_pretty(self)?;
            tokio::fs::write(&lock_file, bytes).await?;
            tracing::debug!("instances lock saved");
            Ok(())
        }
        .instrument(span)
        .await
    }

    /// Удаляет инстанс из lock-файла и его директорию с диска.
    ///
    /// Возвращает удалённый элемент, или `None` если не найден.
    /// Lock-файл **не** сохраняется автоматически.
    pub async fn remove(
        &self,
        dir: &Path,
        name: &str,
    ) -> Result<Option<InstancesItem>> {
        let span = tracing::debug_span!(
            parent: &Self::span(),
            "lock.remove",
            name = name,
        );

        let dir = dir.to_path_buf();
        let name = name.to_owned();

        async move {
            let removed = self.items.remove(&name).map(|(_, item)| item);

            if removed.is_some() {
                tracing::debug!("instance removed from lock");

                let path = dir.join(&name);
                if tokio::fs::try_exists(&path).await? {
                    tokio::fs::remove_dir_all(&path).await?;
                    tracing::debug!(path = %path.display(), "instance directory removed");
                }
            } else {
                tracing::debug!("instance not found in lock");
            }

            Ok(removed)
        }
        .instrument(span)
        .await
    }

    /// Проверяет наличие папок инстансов на диске.
    ///
    /// Для каждого инстанса в lock проверяет, существует ли папка.
    /// Возвращает причины невалидности, не мутирует lock.
    pub async fn validate_dir(
        &self,
        dir: &Path,
    ) -> Result<Vec<InstanceValidateReason>> {
        let span = tracing::info_span!(
            parent: &Self::span(),
            "lock.validate_dir",
            items = self.items.len(),
        );

        let dir = dir.to_path_buf();

        async move {
            let entries: Vec<_> = self
                .items
                .iter()
                .map(|entry| (entry.key().clone(), entry.value().clone()))
                .collect();

            let total = entries.len();
            tracing::info!(total, "starting instances directory validation");

            let mut reasons = Vec::new();
            let mut valid_count = 0usize;

            for (name, meta) in entries {
                let path = dir.join(&name);
                match tokio::fs::metadata(&path).await {
                    Ok(m) if m.is_dir() => {
                        let config_path = path.join("instance.toml");
                        if tokio::fs::try_exists(&config_path).await.unwrap_or(false) {
                            tracing::debug!(name = %name, "instance directory valid");
                            valid_count += 1;
                        } else {
                            tracing::warn!(name = %name, "instance.toml missing");
                            reasons.push(InstanceValidateReason::ConfigMissing(name, meta));
                        }
                    },
                    _ => {
                        tracing::warn!(name = %name, "instance directory not found");
                        reasons.push(InstanceValidateReason::NotFound(name, meta));
                    },
                }
            }

            let invalid_count = reasons.len();
            tracing::info!(
                valid = valid_count,
                invalid = invalid_count,
                total,
                "instances validation complete"
            );

            Ok(reasons)
        }
        .instrument(span)
        .await
    }
}
