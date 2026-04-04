use std::path::PathBuf;

use clients::error::ClientError;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ComposerError {
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Serialise(#[from] toml::de::Error),
    #[error(transparent)]
    Desirialise(#[from] toml::ser::Error),
    #[error(transparent)]
    Client(#[from] ClientError),
    #[error(transparent)]
    Validation(#[from] ValidationErrors),
    #[error(transparent)]
    ValidationSingle(#[from] ValidationError),
    #[error("Failed to extract archive `{path}`")]
    Archive {
        path: PathBuf,
        #[source]
        source: ArchiveError,
    },
}

#[derive(Error, Debug)]
pub enum ArchiveError {
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Zip(#[from] zip::result::ZipError),
}

#[derive(Error, Debug)]
#[error("Validation failed with {count} fatal error(s)", count = .0.len())]
pub struct ValidationErrors(pub Vec<ValidationError>);

#[derive(Error, Debug)]
pub enum ValidationError {
    #[error("Failed to read `{path}`")]
    Read {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
}

pub type Result<T> = std::result::Result<T, ComposerError>;
