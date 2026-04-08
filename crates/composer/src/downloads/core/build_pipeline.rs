use std::path::Path;

use builder::progress::{BuildProgress, BuildStage};
use chrono::Utc;
use clients::github::GithubClient;
use clients::item::{CoreOrigin, Item};
use clients::version::Version;
use tempfile::TempDir;
use tracing::Instrument;

use crate::downloads::commit_extracted_dir;
use crate::error::{ComposerError, Result};
use crate::item::LockItem;
use crate::utils::fs;

use super::executable;

pub async fn build_and_prepare(
    client: &GithubClient,
    item: Item,
    progress: BuildProgress,
    cores_dir: &Path,
) -> std::result::Result<(clients::hash::Hash, LockItem), (Version, ComposerError)> {
    let version = item.version.clone();
    let span = tracing::debug_span!("build.core", version = %version);

    match build_inner(client, &item, &progress, cores_dir).instrument(span).await {
        Ok(dir_hash) => Ok((
            dir_hash,
            LockItem {
                item,
                timestamp: Utc::now(),
                origin: CoreOrigin::Build,
            },
        )),
        Err(e) => Err((version, e)),
    }
}

async fn build_inner(
    client: &GithubClient,
    item: &Item,
    progress: &BuildProgress,
    cores_dir: &Path,
) -> Result<clients::hash::Hash> {
    let source_dir = cores_dir.join(".source");
    tokio::fs::create_dir_all(&source_dir).await?;

    let version_str = item.version.to_string();
    let version_dir = source_dir.join(&version_str);

    // Stage: DownloadSource — download zipball and extract if not already present
    progress.set_stage(BuildStage::DownloadSource);

    if !tokio::fs::try_exists(&version_dir).await.unwrap_or(false) {
        // Download zipball
        let zipball_url = item.zipball_url.as_deref().ok_or(ComposerError::ZipballUrlMissing)?;

        let zipball_path = source_dir.join(format!("{version_str}.zip"));

        tracing::debug!(url = %zipball_url, "downloading source zipball");
        let response = client
            .client
            .get(zipball_url)
            .send()
            .await
            .map_err(clients::error::ClientError::from)?;
        let bytes = response.bytes().await.map_err(clients::error::ClientError::from)?;
        tokio::fs::write(&zipball_path, &bytes).await?;

        // Extract zipball — GitHub creates {owner}-{repo}-{sha}/ inside
        let temp_extract = source_dir.join(format!(".tmp_{version_str}"));
        tokio::fs::create_dir_all(&temp_extract).await?;

        let zp = zipball_path.clone();
        let te = temp_extract.clone();
        tokio::task::spawn_blocking(move || crate::utils::archive::extract_zip(&zp, &te))
            .await
            .map_err(|e| ComposerError::Io(std::io::Error::other(e)))??;

        // Find the single extracted directory (GitHub zipball has one root dir)
        let mut entries = tokio::fs::read_dir(&temp_extract).await?;
        let extracted_root = if let Some(entry) = entries.next_entry().await? {
            entry.path()
        } else {
            return Err(ComposerError::Io(std::io::Error::other("empty zipball")));
        };

        // Rename to version_dir
        tokio::fs::rename(&extracted_root, &version_dir).await?;

        // Cleanup
        let _ = tokio::fs::remove_dir_all(&temp_extract).await;
        let _ = tokio::fs::remove_file(&zipball_path).await;
    }

    progress.set_fraction(1.0);

    // Stages InstallingDeps + CmakeConfigure + CmakeBuild handled by builder
    let build_result = builder::build_core(builder::BuildRequest {
        source_dir: version_dir.clone(),
        progress: progress.clone(),
    })
    .await?;

    // Stage: CopyArtifacts — copy exe + res to staging, compute hash, commit
    progress.set_stage(BuildStage::CopyArtifacts);

    let folder = cores_dir.to_path_buf();
    tokio::fs::create_dir_all(&folder).await?;
    let temp_dir = tokio::task::spawn_blocking(move || TempDir::new_in(folder))
        .await
        .map_err(|e| ComposerError::Io(std::io::Error::other(e)))??;

    let content_dir = temp_dir.path().join("content");
    tokio::fs::create_dir_all(&content_dir).await?;

    // Copy executable -> content/core.{ext}
    let dest_exe = content_dir.join(executable::CANONICAL_NAME);
    tokio::fs::copy(&build_result.executable, &dest_exe).await?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let perms = std::fs::Permissions::from_mode(0o755);
        tokio::fs::set_permissions(&dest_exe, perms).await?;
    }

    // Copy res/ directory
    if tokio::fs::try_exists(&build_result.res_dir).await? {
        copy_dir_recursive(&build_result.res_dir, &content_dir.join("res")).await?;
    }

    // Hash and commit
    let hash_dir = content_dir.clone();
    let dir_hash = tokio::task::spawn_blocking(move || fs::compute_directory_hash(&hash_dir))
        .await
        .map_err(|e| ComposerError::Io(std::io::Error::other(e)))??;

    commit_extracted_dir(&content_dir, &dir_hash, cores_dir).await?;

    Ok(dir_hash)
}

/// Recursively copy a directory.
async fn copy_dir_recursive(
    src: &Path,
    dst: &Path,
) -> Result<()> {
    tokio::fs::create_dir_all(dst).await?;
    let mut entries = tokio::fs::read_dir(src).await?;
    while let Some(entry) = entries.next_entry().await? {
        let src_path = entry.path();
        let dst_path = dst.join(entry.file_name());
        if entry.file_type().await?.is_dir() {
            Box::pin(copy_dir_recursive(&src_path, &dst_path)).await?;
        } else {
            tokio::fs::copy(&src_path, &dst_path).await?;
        }
    }
    Ok(())
}
