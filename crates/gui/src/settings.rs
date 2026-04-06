use serde::{Deserialize, Serialize};

/// Persisted launcher settings.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SettingsLock {
    #[serde(default)]
    pub language: crate::lang::Lang,
}

impl SettingsLock {
    const FILE_PATH: &'static str = "settings.toml";

    /// Loads settings from disk. Returns default if file is missing and saves it.
    pub fn load() -> Self {
        match std::fs::read_to_string(Self::FILE_PATH) {
            Ok(s) => toml::from_str(&s).unwrap_or_default(),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                let default = Self::default();
                let _ = default.save(); // best-effort save
                default
            },
            Err(_) => Self::default(),
        }
    }

    /// Saves settings to disk synchronously.
    pub fn save(&self) -> Result<(), Box<dyn std::error::Error>> {
        let s = toml::to_string_pretty(self)?;
        std::fs::write(Self::FILE_PATH, s)?;
        Ok(())
    }
}
