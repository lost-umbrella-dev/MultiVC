pub mod download;
pub mod error;
pub mod github;
pub mod hash;
pub mod item;
pub mod version;

pub use download::{DownloadProgress, ProgressSink, download};

use crate::github::GithubClient;

pub static USER_AGENT: &str = "MultiVC/0.0 (discord@towinok)";

/// Представляет набор клиентов для различных сервисов
pub struct Clients {
    pub core: GithubClient,
}

impl Clients {
    pub fn new(core: GithubClient) -> Self {
        Self {
            core,
        }
    }
}

pub mod prelude {
    pub use crate::Clients;
    pub use crate::download::{DownloadProgress, ProgressSink, download};
    pub use crate::error::{ClientError, Result};
    pub use crate::hash::Hash;
    pub use crate::item::{Item, ItemDependence};
    pub use crate::version::Version;
}
