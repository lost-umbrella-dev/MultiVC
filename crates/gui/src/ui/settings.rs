use std::path::Path;

use serde::{Deserialize, Serialize};

use composer::paths::PathOverrides;

/// Persisted launcher settings.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SettingsLock {
    #[serde(default)]
    pub language: crate::ui::lang::Lang,
    #[serde(default)]
    pub paths: PathOverrides,
}

impl SettingsLock {
    /// Loads settings from the given path. Returns default if file is missing and saves it.
    pub fn load(path: &Path) -> Self {
        match std::fs::read_to_string(path) {
            Ok(s) => toml::from_str(&s).unwrap_or_default(),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                let default = Self::default();
                let _ = default.save(path); // best-effort save
                default
            },
            Err(_) => Self::default(),
        }
    }

    /// Saves settings to disk synchronously.
    pub fn save(
        &self,
        path: &Path,
    ) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let s = toml::to_string_pretty(self)?;
        std::fs::write(path, s)?;
        Ok(())
    }
}
