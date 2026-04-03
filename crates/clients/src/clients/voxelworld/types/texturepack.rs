//! Типы данных для текстурпаков

use super::common::{LinkResource, TagResource, UserResource};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Элемент списка текстурпаков
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TexturePackListItem {
    /// ID текстурпака
    pub id: i64,
    /// Slug текстурпака
    pub slug: String,
    /// Название текстурпака
    pub title: String,
    /// Описание текстурпака
    pub description: String,
    /// Количество скачиваний
    pub downloads: i64,
    /// Количество лайков
    pub likes: i64,
    /// Дата последнего обновления
    pub last_update_date: DateTime<Utc>,
    /// Автор текстурпака
    pub author: UserResource,
    /// Список тегов
    pub tags: Vec<TagResource>,
    /// URL логотипа
    pub logo_url: String,
}

/// Детальная информация о текстурпаке
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TexturePackDetail {
    /// ID текстурпака
    pub id: i64,
    /// Slug текстурпака
    pub slug: String,
    /// Автор текстурпака
    pub author: UserResource,
    /// Список контрибьюторов
    pub contributors: Vec<UserResource>,
    /// Название текстурпака
    pub title: String,
    /// Описание текстурпака
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
