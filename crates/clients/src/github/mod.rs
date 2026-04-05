pub mod types;

use std::sync::OnceLock;

use reqwest::Client;
use tracing::{Span, instrument};
use url::Url;

use crate::{
    USER_AGENT,
    error::{ClientError, Result},
    item::Item,
};
use types::{Asset, Release};

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

    pub fn span(&self) -> Span {
        self.span.clone()
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

    pub async fn list(&self, options: GitHubListOptions) -> Result<Vec<Item>> {
        Ok(self
            .get_all_releases()
            .await?
            .iter()
            .filter(|x| {
                options.search_version.is_empty() || options.search_version.iter().any(|v| x.tag_name.contains(v))
            })
            .filter_map(|x| {
                let asset = x.assets.iter().find(|a| asset_matches_current_system(&a.name));
                asset.map(|a| Item {
                    name: a.name.to_owned(),
                    version: x.tag_name.to_owned(),
                    url: a.url.to_owned(),
                    hash: a.digest.to_owned(),
                    size: a.size,
                    dependencies: None,
                    supported_engine: None,
                })
            })
            .collect())
    }

    pub async fn get(&self, options: GitHubGetOptions) -> Result<Option<Item>> {
        Ok(self
            .list(GitHubListOptions {
                search_version: vec![options.version],
            })
            .await?
            .first()
            .cloned())
    }
}

pub struct GitHubGetOptions {
    pub version: String,
}

pub struct GitHubListOptions {
    pub search_version: Vec<String>,
}

pub fn asset_matches_current_system(name: &str) -> bool {
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

#[instrument(level = "debug", skip(asset))]
pub fn get_system_asset(asset: &[Asset]) -> Option<&Asset> {
    asset.iter().find(|a| asset_matches_current_system(&a.name))
}
