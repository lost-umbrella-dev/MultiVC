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
/// Returns `&'static str` with no runtime allocation. Unknown keys
/// are returned as-is (fallback to key name).
///
/// # Examples
///
/// ```ignore
/// let text = t("tab.cores", Lang::En);  // "Cores"
/// let text = t("tab.cores", Lang::Ru);  // "Ядра"
/// ```
pub fn t(key: &str, lang: Lang) -> &'static str {
    match (key, lang) {
        // Tab names
        ("tab.cores", Lang::En) => "Cores",
        ("tab.cores", Lang::Ru) => "Ядра",
        ("tab.instances", Lang::En) => "Instances",
        ("tab.instances", Lang::Ru) => "Инстансы",
        // Toolbar actions
        ("action.validate", Lang::En) => "Validate",
        ("action.validate", Lang::Ru) => "Проверить",
        ("action.refresh", Lang::En) => "Fetch from GitHub",
        ("action.refresh", Lang::Ru) => "Загрузить с GitHub",
        ("action.new_instance", Lang::En) => "New instance",
        ("action.new_instance", Lang::Ru) => "Новый инстанс",
        // Settings modal
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
        // Column headers
        ("col.name", Lang::En) => "Name",
        ("col.name", Lang::Ru) => "Имя",
        ("col.version", Lang::En) => "Version",
        ("col.version", Lang::Ru) => "Версия",
        ("col.hash", Lang::En) => "Hash",
        ("col.hash", Lang::Ru) => "Хэш",
        ("col.actions", Lang::En) => "Actions",
        ("col.actions", Lang::Ru) => "Действия",
        ("col.last_launch", Lang::En) => "Last Launch",
        ("col.last_launch", Lang::Ru) => "Последний запуск",
        // Fallback: return unknown key as-is
        (key, _) => key,
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
        let key = "unknown.key";
        assert_eq!(t(key, Lang::En), key);
        assert_eq!(t(key, Lang::Ru), key);
    }

    #[test]
    fn test_serde_roundtrip() {
        let lang = Lang::Ru;
        let json = serde_json::to_string(&lang).unwrap();
        let deserialized: Lang = serde_json::from_str(&json).unwrap();
        assert_eq!(lang, deserialized);
    }
}
