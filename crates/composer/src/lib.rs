use std::path::Path;

use clients::Clients;
use clients::hash::Hash;

// пока что нет провайдеров для контент паков

use crate::error::{ComposerError, Result};
use crate::item::{LockItem, LockMap};
// use crate::lock::content::ContentsLock;
use crate::lock::core::CoresLock;
use crate::lock::instance::Instance;
use crate::lock::instances::{InstanceValidateReason, InstancesItem, InstancesLock, InstancesMap};
use crate::lock::{Lock, ValidateReason};

mod downloads;

pub use downloads::DownloadRequest;
pub mod error;
pub mod item;
pub mod lock;
pub mod message;
pub mod progress;
pub mod utils;
pub mod worker;

/// Имя файла конфигурации инстанса внутри папки инстанса.
const INSTANCE_CONFIG_NAME: &str = "instance.toml";

/// Фасад над in-memory состоянием приложения.
///
/// Скрывает внутренние lock-структуры и клиенты, предоставляя
/// высокоуровневый API для GUI, TUI и CLI.
///
/// Download-функции (`install_cores`, `install_contents`) вынесены в модуль `downloads`.
pub struct Composer {
    pub(crate) clients: Clients,
    pub(crate) instances: InstancesLock,
    pub(crate) cores: CoresLock,
    // pub(crate) contents: ContentsLock,
}

// ── Construction ─────────────────────────────────────────────────────

impl Composer {
    /// Создаёт пустой State (свежая установка, тесты).
    pub fn new(clients: Clients) -> Self {
        Self {
            clients,
            instances: InstancesLock::default(),
            cores: CoresLock { items: LockMap::new() },
            // contents: ContentsLock { items: LockMap::new() },
        }
    }

    /// Загружает все lock-файлы с диска.
    pub async fn load(clients: Clients) -> Result<Self> {
        let instances = InstancesLock::load().await?;
        let cores = CoresLock::load().await?;
        // let contents = ContentsLock::load().await?;

        Ok(Self {
            clients,
            instances,
            cores,
            // contents,
        })
    }
}

// ── Persistence ──────────────────────────────────────────────────────

impl Composer {
    /// Сохраняет все lock-файлы на диск.
    pub async fn save(&self) -> Result<()> {
        self.instances.save().await?;
        self.cores.save().await?;
        // self.contents.save().await?;
        Ok(())
    }

    /// Сохраняет только lock ядер.
    pub async fn save_cores(&self) -> Result<()> {
        self.cores.save().await
    }

    // /// Сохраняет только lock контент-паков.
    // pub async fn save_contents(&self) -> Result<()> {
    //     self.contents.save().await
    // }

    /// Сохраняет только lock инстансов.
    pub async fn save_instances(&self) -> Result<()> {
        self.instances.save().await
    }
}

// ── Items access ─────────────────────────────────────────────────────

impl Composer {
    /// Прямой доступ к элементам ядер.
    pub fn cores_items(&self) -> &LockMap {
        self.cores.items()
    }

    /// Прямой доступ к элементам инстансов (имя → UI-метаданные).
    pub fn instances_items(&self) -> &InstancesMap {
        self.instances.items()
    }
}

// ── Instances CRUD ───────────────────────────────────────────────────

impl Composer {
    /// Создаёт новый инстанс.
    ///
    /// 1. Проверяет уникальность имени в lock.
    /// 2. Создаёт папку `instances/{name}/`.
    /// 3. Сериализует [`Instance`] в `instances/{name}/instance.toml`.
    /// 4. Регистрирует [`InstancesItem`] в lock.
    /// 5. Сохраняет lock на диск.
    ///
    /// TODO: после добавления клиентов зависимостей реализовать доставку зависимостей в instance
    pub async fn create_instance(&self, name: String, config: Instance, meta: InstancesItem) -> Result<()> {
        // Уникальность имени
        if self.instances.items().contains_key(&name) {
            return Err(ComposerError::InstanceAlreadyExists { name });
        }

        let instance_dir = Path::new(InstancesLock::FOLDER).join(&name);
        tokio::fs::create_dir_all(&instance_dir).await?;

        // Записываем instance.toml
        let config_path = instance_dir.join(INSTANCE_CONFIG_NAME);
        let toml_bytes = toml::to_string_pretty(&config)?;
        tokio::fs::write(&config_path, toml_bytes).await?;

        // Регистрируем в lock
        self.instances.items().insert(name, meta);
        self.instances.save().await?;

        Ok(())
    }

    /// Читает конфигурацию инстанса (`instance.toml`) по имени.
    ///
    /// Возвращает [`ComposerError::InstanceNotFound`] если инстанс не зарегистрирован в lock.
    pub async fn get_instance(&self, name: &str) -> Result<Instance> {
        if !self.instances.items().contains_key(name) {
            return Err(ComposerError::InstanceNotFound { name: name.to_owned() });
        }

        let config_path = Path::new(InstancesLock::FOLDER).join(name).join(INSTANCE_CONFIG_NAME);

        let bytes = tokio::fs::read(&config_path).await?;
        let instance: Instance = toml::from_slice(&bytes)?;
        Ok(instance)
    }

    /// Обновляет конфигурацию и метаданные инстанса.
    ///
    /// Перезаписывает `instance.toml` и обновляет запись в lock.
    /// Lock-файл сохраняется автоматически.
    ///
    /// Возвращает [`ComposerError::InstanceNotFound`] если инстанс не зарегистрирован.
    pub async fn edit_instance(&self, name: &str, config: Instance, meta: InstancesItem) -> Result<()> {
        if !self.instances.items().contains_key(name) {
            return Err(ComposerError::InstanceNotFound { name: name.to_owned() });
        }

        // Перезаписываем instance.toml
        let config_path = Path::new(InstancesLock::FOLDER).join(name).join(INSTANCE_CONFIG_NAME);

        let toml_bytes = toml::to_string_pretty(&config)?;
        tokio::fs::write(&config_path, toml_bytes).await?;

        // Обновляем метаданные в lock
        self.instances.items().insert(name.to_owned(), meta);
        self.instances.save().await?;

        Ok(())
    }

    /// Удаляет инстанс по имени: убирает из lock и удаляет директорию с диска.
    ///
    /// Lock-файл **не** сохраняется автоматически — вызывающий код
    /// должен вызвать [`save_instances()`](Self::save_instances) после.
    ///
    /// Возвращает удалённый элемент, или `None` если элемент не найден.
    pub async fn remove_instance(&self, name: &str) -> Result<Option<InstancesItem>> {
        self.instances.remove(name).await
    }
}

// ── Remove (Cores) ───────────────────────────────────────────────────

impl Composer {
    /// Возвращает имена инстансов, чей `core_version` совпадает с `hash`.
    pub async fn instances_using_core(&self, hash: &Hash) -> Result<Vec<String>> {
        let mut dependents = Vec::new();
        for entry in self.instances.items().iter() {
            let name = entry.key().clone();
            match self.get_instance(&name).await {
                Ok(instance) if &instance.core_version == hash => {
                    dependents.push(name);
                },
                Ok(_) => {},
                // Если instance.toml не удалось прочитать — пропускаем,
                // не блокируем удаление из-за битого инстанса.
                Err(e) => {
                    tracing::warn!(instance = %name, error = %e, "failed to read instance config, skipping");
                },
            }
        }
        Ok(dependents)
    }

    /// Удаляет ядро по хэшу: убирает из lock и удаляет директорию с диска.
    ///
    /// Если хотя бы один инстанс ссылается на это ядро (`core_version`),
    /// возвращает [`ComposerError::CoreInUse`] со списком имён инстансов.
    ///
    /// Lock-файл **не** сохраняется автоматически — вызывающий код
    /// должен вызвать [`save_cores()`](Self::save_cores) после.
    ///
    /// Возвращает удалённый элемент, или `None` если элемент не найден.
    pub async fn remove_core(&self, hash: &Hash) -> Result<Option<LockItem>> {
        let dependents = self.instances_using_core(hash).await?;
        if !dependents.is_empty() {
            return Err(ComposerError::CoreInUse {
                hash: hash.clone(),
                dependents,
            });
        }
        self.cores.remove(hash).await
    }
}

// ── Validation ───────────────────────────────────────────────────────

impl Composer {
    /// Проверяет директорию ядер на соответствие lock-файлу.
    pub async fn validate_cores(&self) -> Result<Vec<ValidateReason>> {
        self.cores.validate_dir().await
    }

    // /// Проверяет директорию контент-паков на соответствие lock-файлу.
    // pub async fn validate_contents(&self) -> Result<Vec<ValidateReason>> {
    //     self.contents.validate_dir().await
    // }

    /// Проверяет директорию инстансов на соответствие lock-файлу.
    pub async fn validate_instances(&self) -> Result<Vec<InstanceValidateReason>> {
        self.instances.validate_dir().await
    }
}

// ── Launch helpers ───────────────────────────────────────────────────

impl Composer {
    /// Собирает аргументы запуска для инстанса.
    ///
    /// Возвращает вектор аргументов:
    /// - `--dir <абсолютный путь к папке инстанса>`
    /// - `--res <абсолютный путь к папке res ядра>`
    ///
    /// Возвращает [`ComposerError::InstanceNotFound`] если инстанс не зарегистрирован.
    pub async fn build_launch_args(&self, name: &str) -> Result<Vec<String>> {
        if !self.instances.items().contains_key(name) {
            return Err(ComposerError::InstanceNotFound { name: name.to_owned() });
        }

        let instance = self.get_instance(name).await?;

        let base = std::env::current_dir()?;
        let instance_dir = base.join(InstancesLock::FOLDER).join(name);

        Ok(vec![
            "--dir".to_owned(),
            instance_dir.to_string_lossy().into_owned(),
            "--res".to_owned(),
            base.join("cores")
                .join(instance.core_version.as_ref())
                .to_string_lossy()
                .into_owned(),
        ])
    }
}
