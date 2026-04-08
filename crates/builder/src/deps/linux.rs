use tokio::process::Command;

use crate::error::BuilderError;
use crate::platform::LinuxDistro;
use crate::progress::{BuildProgress, BuildStage};

use super::DepsStatus;

const APT_PACKAGES: &[&str] = &[
    "build-essential",
    "cmake",
    "git",
    "libglfw3-dev",
    "libglew-dev",
    "libglm-dev",
    "libopenal-dev",
    "libpng-dev",
    "zlib1g-dev",
    "libluajit-5.1-dev",
    "libvorbis-dev",
    "libcurl4-openssl-dev",
    "libfreetype-dev",
    "libgtest-dev",
];

const DNF_PACKAGES: &[&str] = &[
    "cmake",
    "gcc-c++",
    "git",
    "glfw-devel",
    "glew-devel",
    "glm-devel",
    "openal-soft-devel",
    "libpng-devel",
    "zlib-devel",
    "luajit-devel",
    "libvorbis-devel",
    "libcurl-devel",
    "freetype-devel",
    "gtest-devel",
];

const PACMAN_PACKAGES: &[&str] = &[
    "cmake",
    "gcc",
    "git",
    "glfw",
    "glew",
    "glm",
    "openal",
    "libpng",
    "zlib",
    "luajit",
    "libvorbis",
    "curl",
    "freetype2",
    "gtest",
];

async fn which(tool: &str) -> bool {
    Command::new("which").arg(tool).output().await.map(|o| o.status.success()).unwrap_or(false)
}

/// Check a single dep via pkg-config, falling back to `which`.
async fn is_installed(pkg: &str) -> bool {
    let via_pkg = Command::new("pkg-config")
        .arg("--exists")
        .arg(pkg)
        .output()
        .await
        .map(|o| o.status.success())
        .unwrap_or(false);
    if via_pkg {
        return true;
    }
    // fall back to checking the binary name directly
    which(pkg).await
}

pub async fn check(distro: &LinuxDistro) -> Result<DepsStatus, BuilderError> {
    // cmake is the primary build tool; always check it
    let cmake_ok = is_installed("cmake").await;

    let mut missing = Vec::new();
    if !cmake_ok {
        missing.push("cmake".to_string());
    }

    let install_command = if !missing.is_empty() {
        Some(install_command_for(distro))
    } else {
        None
    };

    Ok(DepsStatus {
        missing,
        install_command,
    })
}

fn install_command_for(distro: &LinuxDistro) -> String {
    match distro {
        LinuxDistro::Debian => {
            format!("sudo apt-get install -y {}", APT_PACKAGES.join(" "))
        },
        LinuxDistro::Fedora => {
            format!("sudo dnf install -y {}", DNF_PACKAGES.join(" "))
        },
        LinuxDistro::Arch => {
            format!("sudo pacman -S --noconfirm {}", PACMAN_PACKAGES.join(" "))
        },
        LinuxDistro::Unknown => {
            "# Unknown distro — install cmake, gcc/g++, glfw, glew, glm, openal, \
             libpng, zlib, luajit, libvorbis, curl, freetype manually"
                .to_string()
        },
    }
}

pub async fn install(
    distro: &LinuxDistro,
    progress: &BuildProgress,
) -> Result<(), BuilderError> {
    progress.set_stage(BuildStage::InstallingDeps);

    let output = match distro {
        LinuxDistro::Debian => {
            Command::new("sudo")
                .args(["apt-get", "install", "-y"])
                .args(APT_PACKAGES)
                .output()
                .await?
        },
        LinuxDistro::Fedora => {
            Command::new("sudo").args(["dnf", "install", "-y"]).args(DNF_PACKAGES).output().await?
        },
        LinuxDistro::Arch => {
            Command::new("sudo")
                .args(["pacman", "-S", "--noconfirm"])
                .args(PACMAN_PACKAGES)
                .output()
                .await?
        },
        LinuxDistro::Unknown => {
            return Err(BuilderError::DepsInstall {
                message: "Unknown Linux distro — cannot auto-install dependencies. \
                          Please install cmake, gcc/g++, and VoxelCore build deps manually."
                    .to_string(),
            });
        },
    };

    if !output.status.success() {
        return Err(BuilderError::DepsInstall {
            message: String::from_utf8_lossy(&output.stderr).into_owned(),
        });
    }

    progress.set_fraction(1.0);
    Ok(())
}
