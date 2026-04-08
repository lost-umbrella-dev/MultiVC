mod linux;
mod macos;
mod nixos;
pub(crate) mod windows;

use crate::error::BuilderError;
use crate::platform::Platform;
use crate::progress::BuildProgress;

pub struct DepsStatus {
    pub missing: Vec<String>,
    pub install_command: Option<String>,
}

/// Check if build dependencies are present.
pub async fn check_deps(platform: &Platform) -> Result<DepsStatus, BuilderError> {
    match platform {
        Platform::Windows => windows::check().await,
        Platform::MacOS => macos::check().await,
        Platform::NixOS => nixos::check().await,
        Platform::Linux(distro) => linux::check(distro).await,
    }
}

/// Install missing dependencies.
pub async fn install_deps(
    platform: &Platform,
    progress: &BuildProgress,
) -> Result<(), BuilderError> {
    match platform {
        Platform::Windows => windows::install(progress).await,
        Platform::MacOS => macos::install(progress).await,
        Platform::NixOS => nixos::install(progress).await,
        Platform::Linux(distro) => linux::install(distro, progress).await,
    }
}
