use chrono::{DateTime, Utc};
use serde::Deserialize;

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
