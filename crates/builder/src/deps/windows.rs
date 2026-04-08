use std::path::Path;

use tokio::process::Command;

use crate::error::BuilderError;
use crate::progress::{BuildProgress, BuildStage};

use super::DepsStatus;

const MSYS2_BASH: &str = r"C:\msys64\usr\bin\bash.exe";

const MINGW_PACKAGES: &[&str] = &[
    "mingw-w64-clang-x86_64-toolchain",
    "mingw-w64-clang-x86_64-cmake",
    // VoxelCore build dependencies
    "mingw-w64-clang-x86_64-openal",
    "mingw-w64-clang-x86_64-glfw",
    "mingw-w64-clang-x86_64-glew",
    "mingw-w64-clang-x86_64-glm",
    "mingw-w64-clang-x86_64-libpng",
    "mingw-w64-clang-x86_64-zlib",
    "mingw-w64-clang-x86_64-luajit",
    "mingw-w64-clang-x86_64-libvorbis",
    "mingw-w64-clang-x86_64-curl",
    "mingw-w64-clang-x86_64-freetype",
    "mingw-w64-clang-x86_64-entt",
    "mingw-w64-clang-x86_64-gtest",
];

/// Build a Command that runs `cmd` inside the MSYS2 CLANG64 environment.
pub(crate) fn msys2_command(cmd: &str) -> Command {
    let mut c = Command::new(MSYS2_BASH);
    c.env("CHERE_INVOKING", "yes");
    c.env("MSYSTEM", "CLANG64");
    c.args(["-lc", cmd]);
    c
}

fn msys2_present() -> bool {
    Path::new(MSYS2_BASH).exists()
}

/// Check whether the MINGW_PACKAGES are installed by querying pacman inside MSYS2.
async fn toolchain_installed() -> bool {
    let pkg_list = MINGW_PACKAGES.join(" ");
    let cmd = format!("pacman -Q {pkg_list} 2>/dev/null && echo OK");
    msys2_command(&cmd)
        .output()
        .await
        .map(|o| {
            o.status.success()
                && String::from_utf8_lossy(&o.stdout).lines().any(|l| l.trim() == "OK")
        })
        .unwrap_or(false)
}

pub async fn check() -> Result<DepsStatus, BuilderError> {
    if !msys2_present() {
        return Ok(DepsStatus {
            missing: vec!["MSYS2".to_string()],
            install_command: Some(
                "winget install --accept-package-agreements --accept-source-agreements -e MSYS2.MSYS2"
                    .to_string(),
            ),
        });
    }

    if !toolchain_installed().await {
        let pkgs = MINGW_PACKAGES.join(" ");
        return Ok(DepsStatus {
            missing: MINGW_PACKAGES.iter().map(|s| s.to_string()).collect(),
            install_command: Some(format!(
                r"C:\msys64\usr\bin\bash.exe -lc 'pacman -S --noconfirm {pkgs}'"
            )),
        });
    }

    Ok(DepsStatus {
        missing: vec![],
        install_command: None,
    })
}

pub async fn install(progress: &BuildProgress) -> Result<(), BuilderError> {
    progress.set_stage(BuildStage::InstallingDeps);

    // Step 1 — install MSYS2 via winget if absent
    if !msys2_present() {
        let output = Command::new("winget")
            .args([
                "install",
                "--accept-package-agreements",
                "--accept-source-agreements",
                "-e",
                "MSYS2.MSYS2",
            ])
            .output()
            .await?;

        if !output.status.success() {
            return Err(BuilderError::Msys2NotFound {
                path: Path::new(MSYS2_BASH).to_path_buf(),
            });
        }

        // After winget installs MSYS2 the bash binary should now exist
        if !msys2_present() {
            return Err(BuilderError::Msys2NotFound {
                path: Path::new(MSYS2_BASH).to_path_buf(),
            });
        }
    }

    progress.set_fraction(0.3);

    // Step 2 — install toolchain packages via pacman inside MSYS2
    let pkgs = MINGW_PACKAGES.join(" ");
    let cmd = format!("pacman -S --noconfirm {pkgs}");
    let output = msys2_command(&cmd).output().await?;

    if !output.status.success() {
        return Err(BuilderError::DepsInstall {
            message: String::from_utf8_lossy(&output.stderr).into_owned(),
        });
    }

    progress.set_fraction(1.0);
    Ok(())
}
