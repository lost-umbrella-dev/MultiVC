use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

/// Resolved application paths.
///
/// `Default` returns platform-standard directories via the `dirs` crate.
/// Use [`AppPaths::resolve`] to handle portable mode and settings overrides.
#[derive(Debug, Clone)]
pub struct AppPaths {
    /// Root directory — settings.toml lives here, default parent for cores/instances.
    pub root_dir: PathBuf,
    /// Resolved cores directory (root_dir/cores unless overridden in settings).
    pub cores_dir: PathBuf,
    /// Resolved instances directory (root_dir/instances unless overridden in settings).
    pub instances_dir: PathBuf,
}

impl Default for AppPaths {
    fn default() -> Self {
        let root_dir =
            dirs::data_local_dir().expect("no data dir available on this platform").join("MultiVC");
        Self {
            cores_dir: root_dir.join("cores"),
            instances_dir: root_dir.join("instances"),
            root_dir,
        }
    }
}

impl AppPaths {
    /// Settings file path: `root_dir/settings.toml`.
    pub fn settings_path(&self) -> PathBuf {
        self.root_dir.join("settings.toml")
    }

    /// Resolve application paths.
    ///
    /// - `is_portable = true`: forces portable mode (exe directory), creates
    ///   `settings.toml` if absent.
    /// - `is_portable = false`: auto-detects portable mode by checking for
    ///   `settings.toml` next to the executable; falls back to platform dirs.
    pub fn resolve(is_portable: bool) -> Self {
        let exe_dir = std::env::current_exe().ok().and_then(|p| p.parent().map(Path::to_path_buf));

        let portable_dir = if is_portable {
            exe_dir.clone()
        } else {
            exe_dir.as_ref().filter(|dir| dir.join("settings.toml").exists()).cloned()
        };

        let mut paths = if let Some(dir) = portable_dir {
            Self {
                cores_dir: dir.join("cores"),
                instances_dir: dir.join("instances"),
                root_dir: dir,
            }
        } else {
            Self::default()
        };

        let _ = std::fs::create_dir_all(&paths.root_dir);

        let settings_path = paths.settings_path();

        if is_portable && !settings_path.exists() {
            let default = Settings::default();
            let _ = default.save(&settings_path);
        }

        if let Some(settings) = Settings::load(&settings_path) {
            if let Some(cores) = settings.paths.cores {
                paths.cores_dir = cores;
            }
            if let Some(instances) = settings.paths.instances {
                paths.instances_dir = instances;
            }
        }

        let _ = std::fs::create_dir_all(&paths.cores_dir);
        let _ = std::fs::create_dir_all(&paths.instances_dir);

        paths
    }
}

/// Minimal settings for path resolution.
///
/// GUI has its own `SettingsLock` with additional fields (language, etc.)
/// that is a superset of this struct.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Settings {
    #[serde(default)]
    pub paths: PathOverrides,
}

/// Optional path overrides in settings.toml.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PathOverrides {
    pub cores: Option<PathBuf>,
    pub instances: Option<PathBuf>,
}

impl Settings {
    fn load(path: &Path) -> Option<Self> {
        let content = std::fs::read_to_string(path).ok()?;
        toml::from_str(&content).ok()
    }

    fn save(
        &self,
        path: &Path,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let s = toml::to_string_pretty(self)?;
        std::fs::write(path, s)?;
        Ok(())
    }
}
