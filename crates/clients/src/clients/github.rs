use crate::{
    clients::{Client, Item, github::client::GithubClient},
    error::Result,
};

pub mod client;
pub mod types;

pub struct GitHubGetOptions {
    pub version: String,
}

pub struct GitHubListOptions {
    pub search_version: Vec<String>,
}

impl Client for GithubClient {
    type ListOptions = GitHubListOptions;

    type GetOptions = GitHubGetOptions;

    fn client(&self) -> reqwest::Client {
        self.client.clone()
    }

    fn span(&self) -> tracing::Span {
        self.span.clone()
    }

    async fn list(&self, options: Self::ListOptions) -> Result<Vec<Item>> {
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

    async fn get(&self, options: Self::GetOptions) -> Result<Option<Item>> {
        Ok(self
            .list(GitHubListOptions {
                search_version: vec![options.version],
            })
            .await?
            .first()
            .cloned())
    }
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
