use std::path::Path;

use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;
use tracing::debug;

use crate::error::BuilderError;
use crate::platform::Platform;
use crate::progress::{BuildProgress, BuildStage};

/// Parse cmake progress output like `[ 42%] Building CXX object...`
/// Returns a fraction in 0.0..=1.0.
pub(crate) fn parse_cmake_progress(line: &str) -> Option<f32> {
    let start = line.find('[')? + 1;
    let end = line[start..].find('%')? + start;
    line[start..end].trim().parse::<f32>().ok().map(|p| p / 100.0)
}

/// Patch VoxelCore's CMakeLists.txt files to build with MSYS2 packages
/// instead of vcpkg.
///
/// Patches two files:
/// - `src/CMakeLists.txt`: replaces vcpkg-specific OpenAL/LuaJIT/vorbis
///   blocks with pkg-config equivalents
/// - `CMakeLists.txt` (root): adds `ws2_32` to link libraries (needed for
///   Sockets.cpp when statically linking with Clang)
///
/// A `# MULTIVC_PATCHED` marker prevents double-patching on rebuilds.
/// If the expected patterns are not found (e.g. newer VoxelCore version),
/// a warning is logged and the file is left unchanged.
pub(crate) async fn patch_cmakelists_for_msys2(source_dir: &Path) -> Result<(), BuilderError> {
    patch_src_cmakelists(source_dir).await?;
    patch_root_cmakelists(source_dir).await?;
    Ok(())
}

async fn patch_src_cmakelists(source_dir: &Path) -> Result<(), BuilderError> {
    let cmake_path = source_dir.join("src").join("CMakeLists.txt");
    let content = tokio::fs::read_to_string(&cmake_path).await?;

    if content.contains("# MULTIVC_PATCHED") {
        return Ok(());
    }

    let mut patched = content.clone();

    // Patch 1: OpenAL — replace vcpkg CONFIG mode with standard find_package
    const OPENAL_OLD: &str = "\
if(CMAKE_SYSTEM_NAME STREQUAL \"Windows\")
    # specific for vcpkg
    find_package(OpenAL CONFIG REQUIRED)
else()
    find_package(OpenAL REQUIRED)
endif()";
    const OPENAL_NEW: &str = "find_package(OpenAL REQUIRED)";

    patched = patched.replace(OPENAL_OLD, OPENAL_NEW);

    // Patch 2: vorbis+LuaJIT — replace vcpkg Windows branch with pkg-config
    const VORBIS_OLD: &str = "\
if(CMAKE_SYSTEM_NAME STREQUAL \"Windows\")
    find_package(vorbis REQUIRED)
    if(VCPKG_TARGET_TRIPLET MATCHES \"static\")
        add_library(luajit STATIC IMPORTED)
        set_target_properties(
            luajit
            PROPERTIES
                IMPORTED_LOCATION
                \"$ENV{VCPKG_ROOT}/packages/luajit_${VCPKG_TARGET_TRIPLET}/lib/libluajit-5.1.a\"
                INTERFACE_INCLUDE_DIRECTORIES
                \"$ENV{VCPKG_ROOT}/packages/luajit_${VCPKG_TARGET_TRIPLET}/include/\"
        )
    else()
        add_library(luajit SHARED IMPORTED)
        set_target_properties(
            luajit
            PROPERTIES
                IMPORTED_LOCATION
                \"$ENV{VCPKG_ROOT}/packages/luajit_${VCPKG_TARGET_TRIPLET}/bin/lua51.dll\"
                IMPORTED_IMPLIB
                \"$ENV{VCPKG_ROOT}/packages/luajit_${VCPKG_TARGET_TRIPLET}/lib/lua51.lib\"
                INTERFACE_INCLUDE_DIRECTORIES
                \"$ENV{VCPKG_ROOT}/packages/luajit_${VCPKG_TARGET_TRIPLET}/include/luajit\"
        )
    endif()

    add_library(luajit::luajit ALIAS luajit)
else()
    find_package(PkgConfig)

    pkg_check_modules(luajit REQUIRED IMPORTED_TARGET luajit)
    pkg_check_modules(vorbis REQUIRED IMPORTED_TARGET vorbis)
    pkg_check_modules(vorbisfile REQUIRED IMPORTED_TARGET vorbisfile)
    add_library(Vorbis::vorbis ALIAS PkgConfig::vorbis)
    add_library(Vorbis::vorbisfile ALIAS PkgConfig::vorbisfile)
    add_library(luajit::luajit ALIAS PkgConfig::luajit)
endif()";

    const VORBIS_NEW: &str = "\
find_package(PkgConfig)

pkg_check_modules(luajit REQUIRED IMPORTED_TARGET luajit)
pkg_check_modules(vorbis REQUIRED IMPORTED_TARGET vorbis)
pkg_check_modules(vorbisfile REQUIRED IMPORTED_TARGET vorbisfile)
add_library(Vorbis::vorbis ALIAS PkgConfig::vorbis)
add_library(Vorbis::vorbisfile ALIAS PkgConfig::vorbisfile)
add_library(luajit::luajit ALIAS PkgConfig::luajit)";

    patched = patched.replace(VORBIS_OLD, VORBIS_NEW);

    if patched == content {
        tracing::warn!(
            path = %cmake_path.display(),
            "MSYS2 cmake patch patterns not found — skipping patch"
        );
        return Ok(());
    }

    patched.insert_str(0, "# MULTIVC_PATCHED\n");
    tokio::fs::write(&cmake_path, patched.as_bytes()).await?;
    tracing::info!(path = %cmake_path.display(), "patched src/CMakeLists.txt for MSYS2 build");

    Ok(())
}

/// Add `ws2_32` to root CMakeLists.txt link libraries.
///
/// VoxelCore uses Sockets.cpp which calls Winsock2 functions. In vcpkg builds
/// `ws2_32` is pulled transitively via curl, but with MSYS2 static linking
/// it must be explicit.
async fn patch_root_cmakelists(source_dir: &Path) -> Result<(), BuilderError> {
    let cmake_path = source_dir.join("CMakeLists.txt");
    let content = tokio::fs::read_to_string(&cmake_path).await?;

    if content.contains("# MULTIVC_PATCHED") {
        return Ok(());
    }

    const LINK_OLD: &str =
        "target_link_libraries(VoxelEngine PRIVATE VoxelEngineSrc\n                                          $<$<PLATFORM_ID:Windows>:winmm>)";
    const LINK_NEW: &str =
        "target_link_libraries(VoxelEngine PRIVATE VoxelEngineSrc\n                                          $<$<PLATFORM_ID:Windows>:winmm>\n                                          $<$<PLATFORM_ID:Windows>:ws2_32>)";

    let patched = content.replace(LINK_OLD, LINK_NEW);

    if patched == content {
        tracing::warn!(
            path = %cmake_path.display(),
            "root CMakeLists.txt link pattern not found — skipping patch"
        );
        return Ok(());
    }

    let final_content = format!("# MULTIVC_PATCHED\n{patched}");
    tokio::fs::write(&cmake_path, final_content.as_bytes()).await?;
    tracing::info!(path = %cmake_path.display(), "patched root CMakeLists.txt for MSYS2 build");

    Ok(())
}

/// Run cmake configure in `source_dir/build/`.
///
/// On Windows the command runs inside MSYS2 via `msys2_command`.
/// On all other platforms it runs `cmake` directly.
pub async fn configure(
    source_dir: &Path,
    platform: &Platform,
    progress: &BuildProgress,
) -> Result<(), BuilderError> {
    progress.set_stage(BuildStage::CmakeConfigure);

    let build_dir = source_dir.join("build");
    tokio::fs::create_dir_all(&build_dir).await?;

    // Remove stale cmake cache to avoid generator mismatch errors
    let cache_file = build_dir.join("CMakeCache.txt");
    if tokio::fs::try_exists(&cache_file).await.unwrap_or(false) {
        let _ = tokio::fs::remove_file(&cache_file).await;
        let _ = tokio::fs::remove_dir_all(build_dir.join("CMakeFiles")).await;
    }

    let output = match platform {
        Platform::Windows => {
            let cmd = "cmake -G Ninja -DCMAKE_BUILD_TYPE=Release ..";
            debug!(cmd = %cmd, "cmake configure (Windows/MSYS2)");
            let mut c = crate::deps::windows::msys2_command(cmd);
            c.current_dir(&build_dir);
            c.output().await?
        },
        _ => {
            debug!("cmake configure (Unix)");
            Command::new("cmake")
                .args(["-DCMAKE_BUILD_TYPE=Release", ".."])
                .current_dir(&build_dir)
                .output()
                .await?
        },
    };

    if !output.status.success() {
        return Err(BuilderError::CmakeConfigure {
            code: output.status.code().unwrap_or(-1),
            stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
        });
    }

    progress.set_fraction(1.0);
    Ok(())
}

/// Run `cmake --build . --parallel` in `source_dir/build/`.
///
/// Stdout is piped and parsed line-by-line for progress percentages
/// (e.g. `[ 42%]`). Each parsed value is forwarded to
/// `progress.set_fraction`.
pub async fn build(
    source_dir: &Path,
    platform: &Platform,
    progress: &BuildProgress,
) -> Result<(), BuilderError> {
    progress.set_stage(BuildStage::CmakeBuild);

    let build_dir = source_dir.join("build");

    let mut child = match platform {
        Platform::Windows => {
            let cmd = "cmake --build . --parallel";
            debug!(cmd = %cmd, "cmake build (Windows/MSYS2)");
            let mut c = crate::deps::windows::msys2_command(cmd);
            c.current_dir(&build_dir)
                .stdout(std::process::Stdio::piped())
                .stderr(std::process::Stdio::piped());
            c.spawn()?
        },
        _ => {
            debug!("cmake build (Unix)");
            Command::new("cmake")
                .args(["--build", ".", "--parallel"])
                .current_dir(&build_dir)
                .stdout(std::process::Stdio::piped())
                .stderr(std::process::Stdio::piped())
                .spawn()?
        },
    };

    // Read stdout line-by-line for progress.
    if let Some(stdout) = child.stdout.take() {
        let reader = BufReader::new(stdout);
        let mut lines = reader.lines();
        while let Ok(Some(line)) = lines.next_line().await {
            if let Some(pct) = parse_cmake_progress(&line) {
                progress.set_fraction(pct);
            }
            debug!(line = %line, "cmake stdout");
        }
    }

    let output = child.wait_with_output().await?;

    if !output.status.success() {
        return Err(BuilderError::CmakeBuild {
            code: output.status.code().unwrap_or(-1),
            stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
        });
    }

    progress.set_fraction(1.0);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_cmake_progress() {
        assert_eq!(parse_cmake_progress("[  0%] Building CXX object"), Some(0.0));
        assert_eq!(parse_cmake_progress("[ 42%] Building CXX object..."), Some(0.42));
        assert_eq!(parse_cmake_progress("[100%] Linking CXX executable"), Some(1.0));
        assert_eq!(parse_cmake_progress("-- Configuring done"), None);
        assert_eq!(parse_cmake_progress("no bracket here"), None);
    }

    #[test]
    fn parse_progress_edge_cases() {
        assert_eq!(parse_cmake_progress(""), None);
        assert_eq!(parse_cmake_progress("[ no percent ]"), None);
        assert_eq!(parse_cmake_progress("[  3%] Building CXX object..."), Some(0.03));
        assert_eq!(parse_cmake_progress("[ 50%]"), Some(0.5));
    }
}
