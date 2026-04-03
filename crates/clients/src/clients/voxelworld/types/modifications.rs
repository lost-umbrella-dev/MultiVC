//! Типы данных для модов

use super::common::{LinkResource, TagResource, UserResource};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Элемент списка модов
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModListItem {
    /// ID мода
    pub id: i64,
    /// Slug мода
    pub slug: String,
    /// Название мода
    pub title: String,
    /// Описание мода
    pub description: String,
    /// Количество скачиваний
    pub downloads: i64,
    /// Количество лайков
    pub likes: i64,
    /// Дата последнего обновления
    pub last_update_date: Option<DateTime<Utc>>,
    /// Автор мода
    pub author: UserResource,
    /// Список тегов
    pub tags: Vec<TagResource>,
    /// URL логотипа
    pub logo_url: String,
}

/// Детальная информация о моде
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModDetail {
    /// ID мода
    pub id: i64,
    /// Slug мода
    pub slug: String,
    /// Автор мода
    pub author: UserResource,
    /// Список контрибьюторов
    pub contributors: Vec<UserResource>,
    /// Название мода
    pub title: String,
    /// Описание мода
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
