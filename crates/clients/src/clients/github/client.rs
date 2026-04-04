use std::sync::OnceLock;

use crate::{
    clients::github::{
        asset_matches_current_system,
        types::{Asset, Release},
    },
    error::{ClientError, Result},
};
use bytes::Bytes;
use reqwest::Client;
use tracing::{Span, instrument};
use url::Url;

use crate::clients::USER_AGENT;

static BASE: OnceLock<Url> = OnceLock::new();
fn base_url() -> &'static Url {
    BASE.get_or_init(|| Url::parse("https://api.github.com").expect("invalid BASE_URL"))
}

/// Клиент для взаимодействия с Github
pub struct GithubClient {
    /// HTTP клиент для выполнения запросов
    pub client: Client,
    pub span: Span,
    repo: String,
    repo_owner: String,
}

impl GithubClient {
    pub fn new(repo_owner: String, repo: String) -> Result<Self> {
        Ok(Self {
            client: Client::builder()
                .user_agent(USER_AGENT)
                .connection_verbose(true)
                .https_only(true)
                .build()?,
            span: tracing::info_span!("clients", client = "github", repo_owner, repo),
            repo_owner,
            repo,
        })
    }

    fn releases_url(&self) -> Url {
        let mut url = base_url().clone();
        url.path_segments_mut()
            .expect("invalid BASE_URL")
            .push("repos")
            .push(&self.repo_owner)
            .push(&self.repo)
            .push("releases");
        url
    }

    #[instrument(
        level = "debug",
        parent = &self.span,
        skip(self),
        err,
    )]
    pub async fn get_all_releases(&self) -> Result<Vec<Release>> {
        let url = self.releases_url();
        let text = self.client.get(url.as_str()).send().await?.text().await?;

        serde_json::from_str::<Vec<Release>>(&text)
            .inspect_err(|e| tracing::error!(%url, body = text, error = %e, "failed to parse releases"))
            .map_err(Into::into)
    }

    #[instrument(
        level = "debug",
        parent = &self.span,
        skip(self),
        err,
    )]
    pub async fn get_latest_release(&self) -> Result<Release> {
        self.get_all_releases()
            .await?
            .first()
            .cloned()
            .ok_or(ClientError::NotFound)
    }

    #[instrument(level = "debug", skip(self), err)]
    pub async fn download_release(&self, release: &Release) -> Result<Bytes> {
        let asset = get_system_asset(&release.assets).ok_or(crate::error::ClientError::NotFound)?;
        Ok(self.client.get(&asset.url).send().await?.bytes().await?)
    }
}

#[instrument(level = "debug", skip(asset))]
pub fn get_system_asset(asset: &[Asset]) -> Option<&Asset> {
    asset.iter().find(|a| asset_matches_current_system(&a.name))
}
