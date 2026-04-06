use std::collections::HashMap;
use std::path::Path;

use chrono::{DateTime, Utc};
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

// ── View structs ─────────────────────────────────────────────────────

/// Информация о ядре с зависимостями (для отображения в UI).
#[derive(Debug, Clone)]
pub struct CoreInfo {
    pub hash: Hash,
    pub name: String,
    pub version: String,
    pub size: u64,
    pub timestamp: DateTime<Utc>,
    /// Имена инстансов, использующих это ядро.
    pub dependents: Vec<String>,
}

/// Подробная информация об инстансе (для отображения в UI).
#[derive(Debug, Clone)]
pub struct InstanceDetail {
    /// Имя инстанса (ключ в lock).
    pub name: String,
    /// Конфигурация из `instance.toml`.
    pub config: Instance,
    /// UI-метаданные из lock (иконка, баннер).
    pub meta: InstancesItem,
    /// Человекочитаемая версия ядра (например `"v0.31.1"`), или `None`
    /// если ядро не найдено в lock.
    pub core_version_display: Option<String>,
}

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

// ── Query helpers ────────────────────────────────────────────────────

impl Composer {
    /// Резолвит хэш ядра в человекочитаемую версию (например `"v0.31.1"`).
    ///
    /// Возвращает `None` если ядро с таким хэшем не найдено в lock.
    pub fn resolve_core_version(&self, hash: &Hash) -> Option<String> {
        self.cores.items().get(hash).map(|entry| entry.item.version.to_string())
    }

    /// Возвращает список всех ядер с информацией о зависимых инстансах.
    ///
    /// Один вызов вместо `cores_items()` + N × `instances_using_core()`.
    pub async fn cores_with_dependents(&self) -> Result<Vec<CoreInfo>> {
        // Собираем все инстансы и их core_version за один проход
        let mut core_to_instances: std::collections::HashMap<Hash, Vec<String>> = std::collections::HashMap::new();

        for entry in self.instances.items().iter() {
            let name = entry.key().clone();
            match self.get_instance(&name).await {
                Ok(instance) => {
                    core_to_instances.entry(instance.core_version).or_default().push(name);
                },
                Err(e) => {
                    tracing::warn!(instance = %name, error = %e, "failed to read instance config, skipping");
                },
            }
        }

        let mut result = Vec::new();
        for entry in self.cores.items().iter() {
            let hash = entry.key().clone();
            let lock_item = entry.value();
            let dependents = core_to_instances.remove(&hash).unwrap_or_default();
            result.push(CoreInfo {
                hash,
                name: lock_item.item.name.clone(),
                version: lock_item.item.version.to_string(),
                size: lock_item.item.size,
                timestamp: lock_item.timestamp,
                dependents,
            });
        }

        Ok(result)
    }

    /// Возвращает список всех инстансов с полной информацией.
    ///
    /// Один вызов вместо `instances_items()` + N × `get_instance()` + N × `resolve_core_version()`.
    pub async fn instances_with_details(&self) -> Result<Vec<InstanceDetail>> {
        let mut result = Vec::new();

        for entry in self.instances.items().iter() {
            let name = entry.key().clone();
            let meta = entry.value().clone();

            match self.get_instance(&name).await {
                Ok(config) => {
                    let core_version_display = self.resolve_core_version(&config.core_version);
                    result.push(InstanceDetail {
                        name,
                        config,
                        meta,
                        core_version_display,
                    });
                },
                Err(e) => {
                    tracing::warn!(instance = %name, error = %e, "failed to read instance config, skipping");
                },
            }
        }

        Ok(result)
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

    /// Строит полную карту зависимостей: хэш ядра → список имён инстансов.
    pub async fn core_dependents_map(&self) -> HashMap<Hash, Vec<String>> {
        let mut map: HashMap<Hash, Vec<String>> = HashMap::new();
        for entry in self.instances.items().iter() {
            let name = entry.key().clone();
            match self.get_instance(&name).await {
                Ok(instance) => {
                    map.entry(instance.core_version).or_default().push(name);
                },
                Err(e) => {
                    tracing::warn!(instance = %name, error = %e, "failed to read instance config, skipping");
                },
            }
        }
        map
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
                .join(instance.core_version.to_path_buf())
                .join("res")
                .to_string_lossy()
                .into_owned(),
        ])
    }

    /// Собирает [`tokio::process::Command`] для запуска инстанса,
    /// **не** запуская процесс.
    ///
    /// Позволяет вызывающему коду настроить `Stdio` (например `piped()`
    /// для перехвата логов) перед вызовом `.spawn()`.
    ///
    /// # Errors
    ///
    /// - [`ComposerError::InstanceNotFound`] — инстанс не зарегистрирован.
    /// - [`ComposerError::LaunchExeNotFound`] — исполняемый файл ядра не найден на диске.
    pub async fn launch_instance_cmd(&self, name: &str) -> Result<tokio::process::Command> {
        if !self.instances.items().contains_key(name) {
            return Err(ComposerError::InstanceNotFound { name: name.to_owned() });
        }

        let instance = self.get_instance(name).await?;
        let base = std::env::current_dir()?;

        // Абсолютный путь к исполняемому файлу ядра
        let core_dir = base.join(utils::hash::item_path::<CoresLock>(&instance.core_version));
        let exe_path = core_dir.join(downloads::core::executable::CANONICAL_NAME);

        if !tokio::fs::try_exists(&exe_path).await? {
            return Err(ComposerError::LaunchExeNotFound {
                name: name.to_owned(),
                exe_path,
            });
        }

        let args = self.build_launch_args(name).await?;

        // Рабочая директория — папка инстанса (абсолютный путь)
        let instance_dir = base.join(InstancesLock::FOLDER).join(name);

        let mut cmd = tokio::process::Command::new(&exe_path);
        cmd.args(&args);
        cmd.current_dir(&instance_dir);

        #[cfg(windows)]
        {
            const CREATE_NO_WINDOW: u32 = 0x0800_0000;
            cmd.creation_flags(CREATE_NO_WINDOW);
        }

        tracing::debug!(
            exe = %exe_path.display(),
            args = ?args,
            cwd = %instance_dir.display(),
            "prepared launch command for instance '{name}'",
        );

        Ok(cmd)
    }

    /// Запускает инстанс как дочерний процесс.
    ///
    /// Находит исполняемый файл ядра, собирает аргументы `--dir` / `--res`
    /// и спавнит процесс. Stdout и stderr наследуются от родительского процесса.
    ///
    /// Возвращает [`tokio::process::Child`] — вызывающий код решает как работать:
    /// - **CLI**: `child.wait().await` — ждёт завершения.
    /// - **GUI/TUI**: `child.id()` для мониторинга, `child.kill()` для остановки.
    ///
    /// Для перехвата stdout/stderr используйте
    /// [`launch_instance_cmd`](Self::launch_instance_cmd),
    /// настройте `Stdio::piped()` и вызовите `.spawn()` вручную.
    pub async fn launch_instance(&self, name: &str) -> Result<tokio::process::Child> {
        let mut cmd = self.launch_instance_cmd(name).await?;
        let child = cmd.spawn()?;
        tracing::info!(instance = %name, pid = ?child.id(), "instance process spawned");
        Ok(child)
    }
}
