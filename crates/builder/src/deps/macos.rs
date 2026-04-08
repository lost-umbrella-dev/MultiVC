use tokio::process::Command;

use crate::error::BuilderError;
use crate::progress::{BuildProgress, BuildStage};

use super::DepsStatus;

const BREW_PACKAGES: &[&str] = &[
    "cmake",
    "glfw",
    "glew",
    "glm",
    "openal-soft",
    "libpng",
    "luajit",
    "libvorbis",
    "curl",
    "freetype",
    "googletest",
];

async fn which(tool: &str) -> bool {
    Command::new("which").arg(tool).output().await.map(|o| o.status.success()).unwrap_or(false)
}

pub async fn check() -> Result<DepsStatus, BuilderError> {
    let cmake_ok = which("cmake").await;
    let brew_ok = which("brew").await;

    let mut missing = Vec::new();
    if !cmake_ok {
        missing.push("cmake".to_string());
    }
    if !brew_ok {
        missing.push("brew".to_string());
    }

    let install_command = if !missing.is_empty() {
        let pkgs = BREW_PACKAGES.join(" ");
        Some(format!("brew install {pkgs}"))
    } else {
        None
    };

    Ok(DepsStatus {
        missing,
        install_command,
    })
}

pub async fn install(progress: &BuildProgress) -> Result<(), BuilderError> {
    progress.set_stage(BuildStage::InstallingDeps);

    let brew_ok = which("brew").await;
    if !brew_ok {
        return Err(BuilderError::ToolNotFound {
            tool: "brew".to_string(),
        });
    }

    let mut args = vec!["install"];
    args.extend_from_slice(BREW_PACKAGES);

    let output = Command::new("brew").args(&args).output().await?;

    if !output.status.success() {
        return Err(BuilderError::DepsInstall {
            message: String::from_utf8_lossy(&output.stderr).into_owned(),
        });
    }

    progress.set_fraction(1.0);
    Ok(())
}
