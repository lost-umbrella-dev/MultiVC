//! Типы данных для миров

use super::common::{LinkResource, TagResource, UserResource};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Элемент списка миров
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorldListItem {
    /// ID мира
    pub id: i64,
    /// Slug мира
    pub slug: String,
    /// Название мира
    pub title: String,
    /// Описание мира
    pub description: String,
    /// Количество скачиваний
    pub downloads: i64,
    /// Количество лайков
    pub likes: i64,
    /// Дата последнего обновления
    pub last_update_date: DateTime<Utc>,
    /// Автор мира
    pub author: UserResource,
    /// Список тегов
    pub tags: Vec<TagResource>,
    /// URL логотипа
    pub logo_url: String,
}

/// Детальная информация о мире
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorldDetail {
    /// ID мира
    pub id: i64,
    /// Slug мира
    pub slug: String,
    /// Автор мира
    pub author: UserResource,
    /// Список контрибьюторов
    pub contributors: Vec<UserResource>,
    /// Название мира
    pub title: String,
    /// Описание мира
    pub description: String,
    /// Подробное описание
    pub detail_description: String,
    /// Количество скачиваний
    pub downloads: i64,
    /// Количество лайков
    pub likes: i64,
    /// Поставил ли текущий пользователь лайк
    pub is_liked: bool,
    /// Список тегов
    pub tags: Vec<TagResource>,
    /// Список ссылок
    pub links: Vec<LinkResource>,
    /// URL логотипа
    pub logo_url: String,
    /// Дата последнего обновления
    pub last_update_date: DateTime<Utc>,
}
