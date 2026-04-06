//! UI state — cached data from the background worker, organized per-tab.

use std::collections::{HashMap, HashSet};

use clients::hash::Hash;
use clients::item::Item;
use composer::item::LockItem;
use composer::lock::ValidateReason;
use composer::lock::instance::Instance;
use composer::lock::instances::{InstanceValidateReason, InstancesItem};

use crate::download_tracker::DownloadTracker;

// ── Root state ───────────────────────────────────────────────────────

/// Корневое UI-состояние, обновляемое из [`Event`]-ов.
#[derive(Default)]
pub struct UiState {
    /// Данные вкладки «Ядра».
    pub cores: CoresTabState,
    /// Данные вкладки «Инстансы».
    pub instances: InstancesTabState,
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
