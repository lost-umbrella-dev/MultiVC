use clients::item::Item;
use futures_util::{StreamExt, stream};

use crate::State;
use crate::error::{ComposerError, Result};
use crate::lock::Lock;
use crate::lock::core::CoresLock;
use crate::utils::download::{self, DownloadRequest};

impl State {
    /// Устанавливает ядра: скачивает, распаковывает, хэширует и регистрирует в lock.
    ///
    /// Каждый [`DownloadRequest`] может содержать per-item
    /// [`ProgressSink`](clients::prelude::ProgressSink) для отслеживания прогресса.
    pub async fn install_cores(&self, requests: Vec<DownloadRequest>) -> Result<Option<Vec<(Item, ComposerError)>>> {
        let total = requests.len();
        tracing::info!(total, "starting cores install");

        tokio::fs::create_dir_all(CoresLock::folder_name()).await?;

        let client = &self.clients.core;
        let results = stream::iter(
            requests
                .into_iter()
                .map(|request| async move { download::download_item::<CoresLock, _>(client, request).await }),
        )
        .buffer_unordered(download::PARALLELISM)
        .collect::<Vec<_>>()
        .await;

        let mut failed = Vec::new();
        let mut successful_count = 0usize;

        for result in results {
            match result {
                Ok((hash, lock_item)) => {
                    tracing::debug!(
                        name = %lock_item.item.name,
                        version = %lock_item.item.version,
                        hash = %hash,
                        "core installed",
                    );
                    self.cores.items().insert(hash, lock_item);
                    successful_count += 1;
                },
                Err((item, error)) => {
                    tracing::warn!(
                        name = %item.name,
                        version = %item.version,
                        error = %error,
                        "core install failed",
                    );
                    failed.push((item, error));
                },
            }
        }

        tracing::info!(
            successful = successful_count,
            failed = failed.len(),
            "cores install complete"
        );

        if failed.is_empty() { Ok(None) } else { Ok(Some(failed)) }
    }
}
