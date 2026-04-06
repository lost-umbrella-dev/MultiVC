//! Протокол сообщений для канала UI ↔ Background.
//!
//! [`Command`] — отправляется из UI-потока в background (Composer).
//! [`Event`] — отправляется из background обратно в UI.
//!
//! ```text
//! UI Thread                        Background Thread
//! ─────────                        ─────────────────
//! tx.send(Command) ──────────────► rx.recv() → Composer::handle()
//!                                        │
//! rx.recv() ◄──────────────────── tx.send(Event)
//! ```

use clients::github::GitHubListOptions;
use clients::hash::Hash;
use clients::item::Item;
use clients::version::Version;

use crate::DownloadRequest;
use crate::error::ComposerError;
use crate::item::LockItem;
use crate::lock::ValidateReason;
use crate::lock::instance::Instance;
use crate::lock::instances::{InstanceValidateReason, InstancesItem};

/// Снимок элементов lock-файла cores для передачи в UI.
pub type ItemsSnapshot = Vec<(Hash, LockItem)>;

/// Снимок элементов lock-файла инстансов для передачи в UI.
pub type InstancesSnapshot = Vec<(String, InstancesItem)>;

// ── Commands (UI → Background) ──────────────────────────────────────

/// Команда от UI к background-потоку.
pub enum Command {
    /// Сохранить все lock-файлы на диск.
    Save,

    /// Сохранить только lock ядер.
    SaveCores,

    /// Сохранить только lock инстансов.
    SaveInstances,

    /// Установить ядра (скачать, распаковать, зарегистрировать).
    InstallCores { requests: Vec<DownloadRequest> },

    /// Проверить директорию ядер на соответствие lock-файлу.
    ValidateCores,

    /// Проверить директорию инстансов на соответствие lock-файлу.
    ValidateInstances,

    /// Удалить ядро по хэшу.
    RemoveCore { hash: Hash },

    /// Создать новый инстанс.
    CreateInstance {
        name: String,
        config: Instance,
        meta: InstancesItem,
    },

    /// Получить конфигурацию инстанса (instance.toml) по имени.
    GetInstance { name: String },

    /// Обновить инстанс (конфигурацию + метаданные).
    EditInstance {
        name: String,
        config: Instance,
        meta: InstancesItem,
    },

    /// Удалить инстанс по имени.
    RemoveInstance { name: String },

    /// Запросить текущий список установленных ядер из lock.
    GetCoresItems,

    /// Запросить текущий список установленных инстансов из lock.
    GetInstancesItems,

    /// Получить список доступных версий ядер.
    ///
    /// `search_version` — фильтр по версиям (пустой = все).
    FetchCoresList { search_version: GitHubListOptions },

    /// Получить конкретную версию ядра.
    FetchCore { version: Version },

    /// Запустить инстанс как дочерний процесс.
    LaunchInstance { name: String },

    /// Остановить запущенный инстанс (kill процесс).
    StopInstance { name: String },

    /// Завершить background-поток.
    Shutdown,
}

// ── Events (Background → UI) ────────────────────────────────────────

/// Событие от background-потока к UI.
///
/// Ошибки передаются как [`ComposerError`] — UI сам решает как их отображать
/// (`Display` для текста, `Debug` для логов, pattern match для иконок и т.д.).
pub enum Event {
    /// Сохранение завершено.
    Saved(Result<(), ComposerError>),

    /// Сохранение ядер завершено.
    CoresSaved(Result<(), ComposerError>),

    /// Сохранение инстансов завершено.
    InstancesSaved(Result<(), ComposerError>),

    /// Установка ядер завершена.
    CoresInstalled(CoresInstalledResult),

    /// Валидация ядер завершена.
    CoresValidated(Result<Vec<ValidateReason>, ComposerError>),

    /// Валидация инстансов завершена.
    InstancesValidated(Result<Vec<InstanceValidateReason>, ComposerError>),

    /// Ядро удалено.
    CoreRemoved {
        hash: Hash,
        /// Удалённый элемент, или `None` если не найден.
        item: Option<LockItem>,
    },

    /// Инстанс создан.
    InstanceCreated(Result<String, ComposerError>),

    /// Информация об инстансе.
    InstanceInfo(Result<Instance, ComposerError>),

    /// Инстанс обновлён.
    InstanceEdited(Result<String, ComposerError>),

    /// Инстанс удалён.
    InstanceRemoved {
        name: String,
        /// Удалённый элемент, или `None` если не найден.
        item: Option<InstancesItem>,
    },

    /// Список доступных версий ядер получен.
    CoresFetched(Result<Vec<Item>, ComposerError>),

    /// Конкретная версия ядра найдена (или не найдена).
    CoreFetched(Result<Option<Item>, ComposerError>),

    /// Текущий список установленных ядер.
    CoresItems(ItemsSnapshot),

    /// Текущий список установленных инстансов.
    InstancesItems(InstancesSnapshot),

    /// Инстанс запущен.
    InstanceLaunched {
        name: String,
        result: Result<u32, ComposerError>,
    },

    /// Инстанс (дочерний процесс) завершился.
    InstanceStopped {
        name: String,
        /// Exit code, or `None` if killed / unknown.
        status: Option<i32>,
    },

    /// Произошла фатальная ошибка.
    Error(ComposerError),

    /// Background-поток завершён.
    ShutdownComplete,
}

/// Результат установки ядер.
pub struct CoresInstalledResult {
    /// Количество успешно установленных элементов.
    pub successful: usize,
    /// Элементы, которые не удалось установить.
    pub failed: Vec<(Item, ComposerError)>,
}
