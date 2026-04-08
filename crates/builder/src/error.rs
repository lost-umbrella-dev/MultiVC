use std::path::PathBuf;

#[derive(thiserror::Error, Debug)]
pub enum BuilderError {
    #[error(transparent)]
    Io(#[from] std::io::Error),

    #[error("CMake configure failed (exit code {code}): {stderr}")]
    CmakeConfigure {
        code: i32,
        stderr: String,
    },

    #[error("CMake build failed (exit code {code}): {stderr}")]
    CmakeBuild {
        code: i32,
        stderr: String,
    },

    #[error("Dependency installation failed: {message}")]
    DepsInstall {
        message: String,
    },

    #[error("MSYS2 not found at {path} and automatic install failed")]
    Msys2NotFound {
        path: PathBuf,
    },

    #[error("Required tool not found: {tool}")]
    ToolNotFound {
        tool: String,
    },
}
