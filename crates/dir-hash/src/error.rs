use std::path::PathBuf;

#[derive(Debug, thiserror::Error)]
pub enum HashError {
    #[error("failed to read {path}: {source}")]
    Read {
        path: PathBuf,
        source: std::io::Error,
    },
}
