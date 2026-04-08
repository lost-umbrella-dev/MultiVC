use std::path::PathBuf;

use crate::error::BuilderError;
use crate::platform;
use crate::progress::{BuildProgress, BuildStage};

pub struct BuildRequest {
    /// Path to extracted source directory (contains CMakeLists.txt, src/, res/).
    pub source_dir: PathBuf,
    /// Progress reporter.
    pub progress: BuildProgress,
}

pub struct BuildResult {
    /// Path to the built executable.
    pub executable: PathBuf,
    /// Path to the res/ directory in source.
    pub res_dir: PathBuf,
}

/// Execute the full build pipeline.
///
/// Stages:
/// 1. InstallingDeps — detect platform, install missing build dependencies
/// 2. (DownloadSource skipped — composer handles download/extract)
/// 3. CmakeConfigure — run cmake configure
/// 4. CmakeBuild — run cmake --build with progress parsing
/// 5. Return paths to executable + res/
pub async fn build_core(request: BuildRequest) -> Result<BuildResult, BuilderError> {
    let platform = platform::detect();

    // Stage 1: Install deps
    request.progress.set_stage(BuildStage::InstallingDeps);
    crate::deps::install_deps(&platform, &request.progress).await?;

    // Stage 2: DownloadSource — skipped, set stage briefly for UI consistency
    request.progress.set_stage(BuildStage::DownloadSource);
    request.progress.set_fraction(1.0); // already done

    // Stage 3: CMake configure
    // On Windows: patch CMakeLists.txt to use pkg-config instead of vcpkg
    if matches!(platform, platform::Platform::Windows) {
        crate::cmake::patch_cmakelists_for_msys2(&request.source_dir).await?;
    }

    crate::cmake::configure(&request.source_dir, &platform, &request.progress)
        .await?;

    // Stage 4: CMake build
    // cmake::build sets CmakeBuild stage internally
    crate::cmake::build(&request.source_dir, &platform, &request.progress).await?;

    // Find built executable
    let build_dir = request.source_dir.join("build");
    let exe_name = if cfg!(target_os = "windows") {
        "VoxelEngine.exe"
    } else {
        "VoxelEngine"
    };
    let executable = build_dir.join(exe_name);

    // res/ is in source root
    let res_dir = request.source_dir.join("res");

    request.progress.set_stage(BuildStage::Done);

    Ok(BuildResult {
        executable,
        res_dir,
    })
}
