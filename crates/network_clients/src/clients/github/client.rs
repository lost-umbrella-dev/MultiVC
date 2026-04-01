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

use crate::clients::USER_AGENT;

const BASE_URL: &str = "https://api.github.com/repos/";

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
            span: tracing::info_span!(
                "network_client",
                client = "github",
                repository = "{}/{}",
                repo_owner,
                repo
            ),
            repo_owner,
            repo,
        })
    }

    #[instrument(
        level = "debug",
        parent = &self.span,
        skip(self),
        err,
    )]
    pub async fn get_all_releases(&self) -> Result<Vec<Release>> {
        let resp = self
            .client
            .get(format!("{}/{}/{}/releases", BASE_URL, self.repo_owner, self.repo))
            .send()
            .await?;

        let releases = serde_json::from_str::<Vec<Release>>(&resp.text().await?)?;

        Ok(releases)
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
