use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Image {
    pub id: Uuid,
    pub user_id: Uuid,
    pub category_id: Option<Uuid>,
    pub filename: String,
    pub thumbnail: Option<String>,
    pub original_filename: Option<String>,
    pub size: i64,
    pub hash: String,
    pub format: String,
    pub views: i32,
    pub status: String,
    pub expires_at: Option<DateTime<Utc>>,
    pub deleted_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    #[serde(skip)]
    #[sqlx(default)]
    pub total_count: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageResponse {
    pub image_key: String,
    pub filename: String,
    pub size: i64,
    pub format: String,
    pub views: i32,
    pub status: String,
    pub expires_at: Option<DateTime<Utc>>,
    pub deleted_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

impl From<Image> for ImageResponse {
    fn from(image: Image) -> Self {
        Self {
            image_key: image.hash,
            filename: image.filename,
            size: image.size,
            format: image.format,
            views: image.views,
            status: image.status,
            expires_at: image.expires_at,
            deleted_at: image.deleted_at,
            created_at: image.created_at,
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct RestoreRequest {
    pub image_keys: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub struct DeleteRequest {
    pub image_keys: Vec<String>,
    pub permanent: bool,
}

#[derive(Debug, Deserialize)]
pub struct UpdateImageRequest {
    pub tags: Option<Vec<String>>,
}

#[derive(Debug, Deserialize)]
pub struct SetExpiryRequest {
    pub expires_at: Option<DateTime<Utc>>,
}
