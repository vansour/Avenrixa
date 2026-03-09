use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize, sqlx::FromRow, Clone)]
#[allow(dead_code)]
pub struct Category {
    pub id: Uuid,
    pub user_id: Uuid,
    pub name: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct CreateCategoryRequest {
    pub name: String,
}

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

impl Image {
    pub fn url(&self) -> String {
        format!("/images/{}", self.filename)
    }

    pub fn thumbnail_url(&self) -> Option<String> {
        Some(format!("/thumbnails/{}.webp", self.hash))
    }

    pub fn size_formatted(&self) -> String {
        const KB: i64 = 1024;
        const MB: i64 = KB * 1024;
        const GB: i64 = MB * 1024;

        if self.size >= GB {
            format!("{:.2} GB", self.size as f64 / GB as f64)
        } else if self.size >= MB {
            format!("{:.2} MB", self.size as f64 / MB as f64)
        } else if self.size >= KB {
            format!("{:.2} KB", self.size as f64 / KB as f64)
        } else {
            format!("{} B", self.size)
        }
    }

    pub fn is_deleted(&self) -> bool {
        self.deleted_at.is_some()
    }

    pub fn is_expired(&self) -> bool {
        if let Some(expires_at) = self.expires_at {
            expires_at < Utc::now()
        } else {
            false
        }
    }
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

impl ImageResponse {
    pub fn url(&self) -> String {
        format!("/images/{}", self.filename)
    }

    pub fn thumbnail_url(&self) -> String {
        format!("/thumbnails/{}.webp", self.image_key)
    }
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
pub struct UpdateCategoryRequest {
    pub category_id: Option<Uuid>,
    pub tags: Option<Vec<String>>,
}

#[derive(Debug, Deserialize)]
pub struct SetExpiryRequest {
    pub expires_at: Option<DateTime<Utc>>,
}
