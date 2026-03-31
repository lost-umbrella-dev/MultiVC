//! Клиент для VoxelWorld API
//!
//! Этот модуль предоставляет типы данных и функции для взаимодействия с VoxelWorld API.

pub mod client;
pub mod types;

use serde::{Deserialize, Serialize};
// Re-export commonly used types for convenience
pub use types::{
    // Auth types
    auth::{
        BadRequestResponse, ErrorResponse, ForbiddenResponse, MeResponse, OneTimeTokenCheckRequest,
        OneTimeTokenCheckResponse, OneTimeTokenGenerateResponse, UnauthorizedResponse,
    },

    // Common types
    common::{LinkResource, TagResource, UserResource},

    // Mod types
    modifications::{ModDetail, ModListItem},

    // Texturepack types
    texturepack::{TexturePackDetail, TexturePackListItem},

    // Version types
    version::{
        EngineVersionResource, ProjectMinimalInfo, ProjectShortInfo, VersionDependenceResource, VersionDetailResource,
        VersionResource, VersionStatusResource,
    },

    // World types
    world::{WorldDetail, WorldListItem},
};

#[derive(Debug, Deserialize, Serialize)]
pub struct DataResponse<T> {
    pub data: T,
}
