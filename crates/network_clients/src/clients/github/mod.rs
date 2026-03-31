use crate::error::{ClientError, Result};
use bytes::Bytes;
use chrono::{DateTime, Utc};
use reqwest::Client;
use serde::Deserialize;
use tracing::{Span, instrument};

use crate::clients::USER_AGENT;

#[derive(Debug, Deserialize, Clone)]
pub struct Release {
    /// API Ссылка на релиз
    pub url: String,
    /// Ссылка на человекочитаемый релиз
    pub html_url: String,
    /// vXx.Xx.Xx
    pub tag_name: String,
    /// Дата публикации
    pub published_at: DateTime<Utc>,
    /// Ссылка на source tarball
    pub tarball_url: String,
    /// Ссылка на source zipball
    pub zipball_url: String,
    /// Сообщение релиза
    pub body: String,
    pub assets: Vec<Asset>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct Asset {
    /// API Ссылка на релиз
    pub url: String,
    /// Название файла
    pub name: String,
    /// vXx.Xx.Xx
    pub tag_name: String,
    /// Размер файла в байтах
    pub size: u64,
    /// Download url
    pub browser_download_url: String,
    /// SHA256
    pub digest: String,
}

const BASE_URL: &str = "https://api.github.com/repos/";

/// Клиент для взаимодействия с Github
pub struct GithubClient {
    /// HTTP клиент для выполнения запросов
    client: Client,
    span: Span,
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
        let asset = get_system_asset(&release.assets)?;
        Ok(self.client.get(&asset.url).send().await?.bytes().await?)
    }
}

#[instrument(level = "debug", skip(asset), err)]
pub fn get_system_asset(asset: &[Asset]) -> Result<&Asset> {
    asset
        .iter()
        .find(|a| asset_matches_current_system(&a.name))
        .ok_or(ClientError::NotFound)
}

fn asset_matches_current_system(name: &str) -> bool {
    let name = name.to_ascii_lowercase();

    match std::env::consts::OS {
        "windows" => {
            (name.ends_with(".zip") || name.ends_with(".msi") || name.ends_with(".exe"))
                && (name.contains("win") || name.contains("windows"))
        },
        "macos" => {
            (name.ends_with(".dmg") || name.ends_with(".pkg") || name.ends_with(".zip"))
                && (name.contains("mac") || name.contains("osx") || name.contains("darwin") || name.contains("macos"))
        },
        "linux" => {
            (name.ends_with(".appimage")
                || name.ends_with(".deb")
                || name.ends_with(".rpm")
                || name.ends_with(".tar.gz"))
                && !name.contains("win")
                && !name.contains("mac")
        },
        _ => false,
    }
}
