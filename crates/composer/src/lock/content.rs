use std::path::Path;

use serde::{Deserialize, Serialize};
use tracing::{Instrument, Span};

use crate::error::Result;
use crate::item::LockMap;
use crate::lock::Lock;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
/// Представляет lock файл контент-паков
pub struct ContentsLock {
    #[serde(flatten)]
    pub items: LockMap,
}

impl Lock for ContentsLock {
    fn span() -> Span {
        tracing::info_span!("lock", r#type = "contents")
    }

    fn items(&self) -> &LockMap {
        &self.items
    }

    async fn load(lock_file: &Path) -> Result<Self> {
        let span = tracing::debug_span!(
            parent: &Self::span(),
            "lock.load",
            file = %lock_file.display(),
        );

        let lock_file = lock_file.to_path_buf();

        async move {
            tracing::debug!("loading lock file");
            match tokio::fs::read(&lock_file).await {
                Ok(bytes) => {
                    let lock: Self = toml::from_slice(&bytes)?;
                    tracing::debug!(items = lock.items.len(), "lock file loaded");
                    Ok(lock)
                },
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                    tracing::debug!("lock file not found, creating default");
                    let lock = Self::default();
                    lock.save(&lock_file).await?;
                    Ok(lock)
                },
                Err(e) => Err(e.into()),
            }
        }
        .instrument(span)
        .await
    }
}
