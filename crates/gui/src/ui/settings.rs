use std::path::Path;

use serde::{Deserialize, Serialize};

use clients::item::{BUILD_AVAILABLE, RELEASE_AVAILABLE};
use composer::paths::PathOverrides;

/// Which core variants the user wants to see / install.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum InstallMode {
    OnlyRelease,
    OnlyBuild,
    #[default]
    Both,
}

impl InstallMode {
    pub fn has_release(self) -> bool {
        matches!(self, Self::OnlyRelease | Self::Both)
    }

    pub fn has_build(self) -> bool {
        matches!(self, Self::OnlyBuild | Self::Both)
    }
}

/// Persisted launcher settings.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SettingsLock {
    #[serde(default)]
    pub language: crate::ui::lang::Lang,
    #[serde(default)]
    pub paths: PathOverrides,
    #[serde(default)]
    pub install_mode: InstallMode,
}

impl SettingsLock {
    /// Loads settings from the given path. Returns default if file is missing and saves it.
    pub fn load(path: &Path) -> Self {
        let mut lock = match std::fs::read_to_string(path) {
            Ok(s) => toml::from_str(&s).unwrap_or_default(),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                let default = Self::default();
                let _ = default.save(path); // best-effort save
                default
            },
            Err(_) => Self::default(),
        };

        // Force valid mode for platform
        if lock.install_mode == InstallMode::OnlyRelease && !RELEASE_AVAILABLE {
            lock.install_mode = InstallMode::OnlyBuild;
        }
        if lock.install_mode == InstallMode::OnlyBuild && !BUILD_AVAILABLE {
            lock.install_mode = InstallMode::OnlyRelease;
        }

        lock
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
