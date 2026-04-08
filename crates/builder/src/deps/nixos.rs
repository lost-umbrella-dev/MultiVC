use crate::error::BuilderError;
use crate::progress::BuildProgress;

use super::DepsStatus;

pub async fn check() -> Result<DepsStatus, BuilderError> {
    Ok(DepsStatus {
        missing: vec![],
        install_command: None,
    })
}

pub async fn install(_progress: &BuildProgress) -> Result<(), BuilderError> {
    Ok(())
}
