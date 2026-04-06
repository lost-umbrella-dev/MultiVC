//! UI state — cached data from the background worker, organized per-tab.

use std::collections::{HashMap, HashSet};

use clients::hash::Hash;
use clients::item::Item;
use composer::item::LockItem;
use composer::lock::ValidateReason;
use composer::lock::instance::Instance;
use composer::lock::instances::{InstanceValidateReason, InstancesItem};

use crate::download_tracker::DownloadTracker;

// ── Sort state ───────────────────────────────────────────────────────

/// Sort direction for table columns.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SortDir {
    #[default]
    None, // default/insertion order
    Ascending,
    Descending,
}

impl SortDir {
    /// Cycles: None → Ascending → Descending → None
    pub fn cycle(self) -> Self {
        match self {
            Self::None => Self::Ascending,
            Self::Ascending => Self::Descending,
            Self::Descending => Self::None,
        }
    }
}

/// Which column is active for sorting in the cores tab.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CoresSortColumn {
    #[default]
    None,
    Name,
    Version,
}

/// Which column is active for sorting in the instances tab.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum InstancesSortColumn {
    #[default]
    LastLaunch, // default sort
    Name,
}

// ── Settings state ───────────────────────────────────────────────────

/// Settings state — persisted settings and modal state.
pub struct SettingsState {
    pub lock: crate::settings::SettingsLock,
    /// Whether the settings modal is currently open.
    pub open: bool,
}

impl Default for SettingsState {
    fn default() -> Self {
        Self {
            lock: crate::settings::SettingsLock::load(),
            open: false,
        }
    }
}

// ── Root state ───────────────────────────────────────────────────────

/// Корневое UI-состояние, обновляемое из `Event`-ов.
#[derive(Default)]
pub struct UiState {
    /// Данные вкладки «Ядра».
    pub cores: CoresTabState,
    /// Данные вкладки «Инстансы».
    pub instances: InstancesTabState,
    /// Settings state.
    pub settings: SettingsState,
}

// ── Cores tab ────────────────────────────────────────────────────────

/// Состояние вкладки «Ядра».
#[derive(Default)]
pub struct CoresTabState {
    /// Установленные ядра (кэш из lock).
    pub installed: Vec<(Hash, LockItem)>,
    /// Доступные на GitHub версии.
    pub available: Vec<Item>,
    /// Результат последней валидации.
    pub validation: Vec<ValidateReason>,
    /// Per-item загрузки.
    pub downloads: DownloadTracker,
    /// Items, выбранные для скачивания.
    /// Накапливаются при кликах на чекбоксы, отправляются по кнопке "Download".
    pub pending_installs: Vec<Item>,
    /// Идёт ли глобальная операция (валидация, fetch),
    /// блокирующая всю вкладку.
    pub busy: bool,
    /// Уже запрашивали fetch при первом входе на вкладку.
    pub fetched_once: bool,
    /// Подтверждение удаления ядра: (hash, display_name).
    pub confirm_remove: Option<(Hash, String)>,
    /// Маппинг: хэш ядра → список имён инстансов, использующих это ядро.
    /// Заполняется при загрузке конфигов инстансов.
    pub core_dependents: HashMap<Hash, Vec<String>>,
    /// Sort column for installed cores.
    pub installed_sort_col: CoresSortColumn,
    /// Sort direction for installed cores.
    pub installed_sort_dir: SortDir,
    /// Sort column for available cores.
    pub available_sort_col: CoresSortColumn,
    /// Sort direction for available cores.
    pub available_sort_dir: SortDir,
}

// ── Instances tab ────────────────────────────────────────────────────

/// Состояние вкладки «Инстансы».
#[derive(Default)]
pub struct InstancesTabState {
    /// Установленные инстансы (кэш из lock).
    pub installed: Vec<(String, InstancesItem)>,
    /// Результат последней валидации.
    pub validation: Vec<InstanceValidateReason>,
    /// Форма создания нового инстанса (модальное окно, если открыта).
    pub create_form: Option<InstanceForm>,
    /// Форма редактирования (имя + форма).
    #[allow(dead_code)]
    pub edit_form: Option<(String, InstanceForm)>,
    /// Просматриваемый инстанс (результат GetInstance).
    pub viewing: Option<Instance>,
    /// Имена инстансов, над которыми сейчас идёт операция.
    pub busy_instances: HashSet<String>,
    /// Подтверждение удаления инстанса (имя для отображения).
    pub confirm_remove: Option<String>,
    /// Идёт ли глобальная операция (валидация, сохранение).
    pub busy: bool,
    /// Запущенные инстансы: имя → PID.
    pub running_instances: HashMap<String, u32>,
    /// Имя инстанса, чей лог сейчас открыт в модалке.
    pub log_viewer: Option<String>,
    /// Sort column for instances.
    pub sort_col: InstancesSortColumn,
    /// Sort direction for instances.
    pub sort_dir: SortDir,
}

impl InstancesTabState {
    fn default() -> Self {
        Self {
            installed: Vec::new(),
            validation: Vec::new(),
            create_form: None,
            edit_form: None,
            viewing: None,
            busy_instances: HashSet::new(),
            confirm_remove: None,
            busy: false,
            running_instances: HashMap::new(),
            log_viewer: None,
            sort_col: InstancesSortColumn::default(),
            sort_dir: SortDir::Descending,
        }
    }
}

/// Поля формы создания / редактирования инстанса.
#[derive(Default, Clone)]
pub struct InstanceForm {
    pub name: String,
    pub description: String,
    /// Индекс выбранного ядра в `CoresTabState.installed`.
    pub selected_core_idx: Option<usize>,
    pub icon: String,
    pub banner: String,
}
