//! Language/i18n system for MultiVC GUI.
//!
//! Provides translation support with `Lang` enum and `t()` function
//! for rendering localized strings. All translations are statically compiled.

use serde::{Deserialize, Serialize};

/// Available languages for the MultiVC GUI.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum Lang {
    /// English (default)
    #[default]
    En,
    /// Russian
    Ru,
}

/// Translates a key to a localized string for the given language.
///
/// Returns `&'static str` with no runtime allocation for known keys.
/// Unknown keys are leaked to produce a `&'static str` (safe — only
/// happens if a developer forgot to add a key, not at user's input).
pub fn t(key: &str, lang: Lang) -> &'static str {
    match (key, lang) {
        // ── Tab names ───────────────────────────────────────────
        ("tab.cores", Lang::En) => "Cores",
        ("tab.cores", Lang::Ru) => "Ядра",
        ("tab.instances", Lang::En) => "Instances",
        ("tab.instances", Lang::Ru) => "Инстансы",

        // ── Toolbar / actions ───────────────────────────────────
        ("action.validate", Lang::En) => "Validate",
        ("action.validate", Lang::Ru) => "Проверить",
        ("action.refresh", Lang::En) => "Fetch from GitHub",
        ("action.refresh", Lang::Ru) => "Загрузить с GitHub",
        ("action.new_instance", Lang::En) => "New instance",
        ("action.new_instance", Lang::Ru) => "Новый инстанс",
        ("action.create", Lang::En) => "Create",
        ("action.create", Lang::Ru) => "Создать",
        ("action.cancel", Lang::En) => "Cancel",
        ("action.cancel", Lang::Ru) => "Отмена",
        ("action.download", Lang::En) => "Download",
        ("action.download", Lang::Ru) => "Скачать",

        // ── Settings modal ──────────────────────────────────────
        ("settings.title", Lang::En) => "Settings",
        ("settings.title", Lang::Ru) => "Настройки",
        ("settings.language", Lang::En) => "Language",
        ("settings.language", Lang::Ru) => "Язык",
        ("settings.build_info", Lang::En) => "Build Info",
        ("settings.build_info", Lang::Ru) => "Информация о сборке",
        ("settings.version", Lang::En) => "Version",
        ("settings.version", Lang::Ru) => "Версия",
        ("settings.cores_count", Lang::En) => "Cores installed",
        ("settings.cores_count", Lang::Ru) => "Ядер установлено",
        ("settings.instances_count", Lang::En) => "Instances installed",
        ("settings.instances_count", Lang::Ru) => "Инстансов установлено",
        ("settings.open_cores", Lang::En) => "Open Cores Folder",
        ("settings.open_cores", Lang::Ru) => "Открыть папку ядер",
        ("settings.open_instances", Lang::En) => "Open Instances Folder",
        ("settings.open_instances", Lang::Ru) => "Открыть папку инстансов",
        ("settings.open_root", Lang::En) => "Open Launcher Folder",
        ("settings.open_root", Lang::Ru) => "Открыть папку лаунчера",

        // ── Column headers ──────────────────────────────────────
        ("col.name", Lang::En) => "Name",
        ("col.name", Lang::Ru) => "Имя",
        ("col.version", Lang::En) => "Version",
        ("col.version", Lang::Ru) => "Версия",
        ("col.hash", Lang::En) => "Hash",
        ("col.hash", Lang::Ru) => "Хэш",
        ("col.size", Lang::En) => "Size",
        ("col.size", Lang::Ru) => "Размер",
        ("col.actions", Lang::En) => "Actions",
        ("col.actions", Lang::Ru) => "Действия",
        ("col.last_launch", Lang::En) => "Last Launch",
        ("col.last_launch", Lang::Ru) => "Последний запуск",

        // ── Section labels ──────────────────────────────────────
        ("section.installed", Lang::En) => "Installed",
        ("section.installed", Lang::Ru) => "Установлено",
        ("section.available", Lang::En) => "Available versions",
        ("section.available", Lang::Ru) => "Доступные версии",
        ("section.validation", Lang::En) => "Validation results",
        ("section.validation", Lang::Ru) => "Результаты валидации",

        // ── Tooltips ────────────────────────────────────────────
        ("tip.validate_cores", Lang::En) => "Validate cores",
        ("tip.validate_cores", Lang::Ru) => "Проверить ядра",
        ("tip.validate_instances", Lang::En) => "Validate instances",
        ("tip.validate_instances", Lang::Ru) => "Проверить инстансы",
        ("tip.fetch_github", Lang::En) => "Fetch from GitHub",
        ("tip.fetch_github", Lang::Ru) => "Загрузить с GitHub",
        ("tip.download_selected", Lang::En) => "Download all selected cores",
        ("tip.download_selected", Lang::Ru) => "Скачать все выбранные ядра",
        ("tip.create_instance", Lang::En) => "Create instance",
        ("tip.create_instance", Lang::Ru) => "Создать инстанс",
        ("tip.delete", Lang::En) => "Delete",
        ("tip.delete", Lang::Ru) => "Удалить",
        ("tip.open_folder", Lang::En) => "Open folder",
        ("tip.open_folder", Lang::Ru) => "Открыть папку",
        ("tip.view_log", Lang::En) => "View log",
        ("tip.view_log", Lang::Ru) => "Просмотр лога",
        ("tip.launch", Lang::En) => "Launch",
        ("tip.launch", Lang::Ru) => "Запуск",
        ("tip.stop", Lang::En) => "Stop",
        ("tip.stop", Lang::Ru) => "Остановить",
        ("tip.stop_first", Lang::En) => "Stop instance first",
        ("tip.stop_first", Lang::Ru) => "Сначала остановите инстанс",
        ("tip.settings", Lang::En) => "Settings",
        ("tip.settings", Lang::Ru) => "Настройки",
        ("tip.installed", Lang::En) => "Installed",
        ("tip.installed", Lang::Ru) => "Установлено",
        ("tip.open_editor", Lang::En) => "Open in external editor",
        ("tip.open_editor", Lang::Ru) => "Открыть во внешнем редакторе",

        // ── Instance create form ────────────────────────────────
        ("form.new_instance", Lang::En) => "New instance",
        ("form.new_instance", Lang::Ru) => "Новый инстанс",
        ("form.name", Lang::En) => "Name:",
        ("form.name", Lang::Ru) => "Имя:",
        ("form.description", Lang::En) => "Description:",
        ("form.description", Lang::Ru) => "Описание:",
        ("form.core", Lang::En) => "Core:",
        ("form.core", Lang::Ru) => "Ядро:",
        ("form.select_core", Lang::En) => "(select core)",
        ("form.select_core", Lang::Ru) => "(выберите ядро)",

        // ── Confirm dialogs ─────────────────────────────────────
        ("confirm.delete_core", Lang::En) => "Delete core?",
        ("confirm.delete_core", Lang::Ru) => "Удалить ядро?",
        ("confirm.delete_instance", Lang::En) => "Delete instance?",
        ("confirm.delete_instance", Lang::Ru) => "Удалить инстанс?",

        // ── Status labels ───────────────────────────────────────
        ("status.running", Lang::En) => "Running",
        ("status.running", Lang::Ru) => "Запущен",
        ("status.loading", Lang::En) => "Loading available versions...",
        ("status.loading", Lang::Ru) => "Загрузка доступных версий...",
        ("status.no_instances", Lang::En) => "No instances. Press \"+\" to create one.",
        ("status.no_instances", Lang::Ru) => "Нет инстансов. Нажмите \"+\" чтобы создать.",
        ("status.no_log", Lang::En) => "No log file found. Launch the instance first.",
        ("status.no_log", Lang::Ru) => "Лог-файл не найден. Сначала запустите инстанс.",
        ("status.fetching", Lang::En) => "Fetching versions...",
        ("status.fetching", Lang::Ru) => "Загрузка версий...",
        ("status.creating", Lang::En) => "Creating instance...",
        ("status.creating", Lang::Ru) => "Создание инстанса...",

        // ── Toast messages ──────────────────────────────────────
        // Instance panel tabs
        ("panel.info", Lang::En) => "Info",
        ("panel.info", Lang::Ru) => "Инфо",
        ("panel.log", Lang::En) => "Log",
        ("panel.log", Lang::Ru) => "Лог",
        ("panel.settings", Lang::En) => "Settings",
        ("panel.settings", Lang::Ru) => "Настройки",

        // Info tab fields
        ("info.description", Lang::En) => "Description",
        ("info.description", Lang::Ru) => "Описание",
        ("info.core_version", Lang::En) => "Core version",
        ("info.core_version", Lang::Ru) => "Версия ядра",
        ("info.pid", Lang::En) => "PID",
        ("info.pid", Lang::Ru) => "PID",
        ("info.created_at", Lang::En) => "Created",
        ("info.created_at", Lang::Ru) => "Создан",
        ("info.disk_size", Lang::En) => "Disk size",
        ("info.disk_size", Lang::Ru) => "Размер на диске",
        ("info.dependencies", Lang::En) => "Dependencies",
        ("info.dependencies", Lang::Ru) => "Зависимости",
        ("info.path", Lang::En) => "Path",
        ("info.path", Lang::Ru) => "Путь",
        ("info.no_description", Lang::En) => "No description",
        ("info.no_description", Lang::Ru) => "Нет описания",
        ("info.no_dependencies", Lang::En) => "No dependencies",
        ("info.no_dependencies", Lang::Ru) => "Нет зависимостей",
        ("info.calculating", Lang::En) => "Calculating...",
        ("info.calculating", Lang::Ru) => "Подсчёт...",

        // Log filter
        ("log.filter_info", Lang::En) => "Info",
        ("log.filter_info", Lang::Ru) => "Инфо",
        ("log.filter_warn", Lang::En) => "Warn",
        ("log.filter_warn", Lang::Ru) => "Пред.",
        ("log.filter_error", Lang::En) => "Error",
        ("log.filter_error", Lang::Ru) => "Ошибки",

        // Instance settings tab
        ("isettings.change_icon", Lang::En) => "Change icon",
        ("isettings.change_icon", Lang::Ru) => "Сменить иконку",
        ("isettings.change_banner", Lang::En) => "Change banner",
        ("isettings.change_banner", Lang::Ru) => "Сменить баннер",
        ("isettings.save", Lang::En) => "Save",
        ("isettings.save", Lang::Ru) => "Сохранить",
        ("isettings.no_icon", Lang::En) => "No icon",
        ("isettings.no_icon", Lang::Ru) => "Нет иконки",
        ("isettings.no_banner", Lang::En) => "No banner",
        ("isettings.no_banner", Lang::Ru) => "Нет баннера",

        // Tooltips for image picker
        ("tip.info", Lang::En) => "Instance info",
        ("tip.info", Lang::Ru) => "Информация об инстансе",
        ("tip.formats", Lang::En) => "PNG, JPG, WebP, SVG",
        ("tip.formats", Lang::Ru) => "PNG, JPG, WebP, SVG",

        ("toast.locks_saved", Lang::En) => "All lock files saved",
        ("toast.cores_saved", Lang::En) => "Cores lock saved",
        ("toast.instances_saved", Lang::En) => "Instances lock saved",
        ("toast.cores_valid", Lang::En) => "Cores: all valid",
        ("toast.instances_valid", Lang::En) => "Instances: all valid",
        ("toast.worker_stopped", Lang::En) => "Background worker stopped",
        ("toast.locks_saved", Lang::Ru) => "Все lock-файлы сохранены",
        ("toast.cores_saved", Lang::Ru) => "Lock ядер сохранён",
        ("toast.instances_saved", Lang::Ru) => "Lock инстансов сохранён",
        ("toast.cores_valid", Lang::Ru) => "Ядра: всё в порядке",
        ("toast.instances_valid", Lang::Ru) => "Инстансы: всё в порядке",
        ("toast.worker_stopped", Lang::Ru) => "Фоновый воркер остановлен",

        // ── Log viewer ──────────────────────────────────────────
        ("log.title_prefix", Lang::En) => "Log",
        ("log.title_prefix", Lang::Ru) => "Лог",

        // ── Validation messages ─────────────────────────────────
        ("validation.hash_mismatch", Lang::En) => "Hash mismatch",
        ("validation.hash_mismatch", Lang::Ru) => "Хэш не совпадает",
        ("validation.not_found", Lang::En) => "Not found",
        ("validation.not_found", Lang::Ru) => "Не найден",
        ("validation.dir_not_found", Lang::En) => "Directory not found",
        ("validation.dir_not_found", Lang::Ru) => "Директория не найдена",

        // ── Fallback ────────────────────────────────────────────
        _ => {
            // Leak the key string to produce &'static str.
            // Only happens for developer-forgotten keys, not user input.
            Box::leak(key.to_owned().into_boxed_str())
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lang_default() {
        assert_eq!(Lang::default(), Lang::En);
    }

    #[test]
    fn test_translation_en() {
        assert_eq!(t("tab.cores", Lang::En), "Cores");
        assert_eq!(t("tab.instances", Lang::En), "Instances");
        assert_eq!(t("action.validate", Lang::En), "Validate");
    }

    #[test]
    fn test_translation_ru() {
        assert_eq!(t("tab.cores", Lang::Ru), "Ядра");
        assert_eq!(t("tab.instances", Lang::Ru), "Инстансы");
        assert_eq!(t("action.validate", Lang::Ru), "Проверить");
    }

    #[test]
    fn test_fallback_unknown_key() {
        assert_eq!(t("unknown.key.test", Lang::En), "unknown.key.test");
    }
}
