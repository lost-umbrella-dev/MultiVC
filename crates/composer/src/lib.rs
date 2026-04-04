use clients::clients::Clients;

// пока что нет провайдеров для контент паков

use crate::error::Result;
use crate::item::LockMap;
// use crate::lock::content::ContentsLock;
use crate::lock::core::CoresLock;
use crate::lock::instances::InstancesLock;
use crate::lock::{Lock, ValidateReason};

mod downloads;

pub use downloads::DownloadRequest;
pub mod error;
pub mod item;
pub mod lock;
pub mod message;
pub mod progress;
pub mod utils;

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
            instances: InstancesLock { items: LockMap::new() },
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

    /// Прямой доступ к элементам инстансов.
    pub fn instances_items(&self) -> &LockMap {
        self.instances.items()
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
    pub async fn validate_instances(&self) -> Result<Vec<ValidateReason>> {
        self.instances.validate_dir().await
    }
}
