use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
pub use shared_types::image::{DeleteRequest, ImageResponse, ImageStatus, SetExpiryRequest};

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Image {
    pub id: uuid::Uuid,
    pub user_id: uuid::Uuid,
    pub filename: String,
    pub thumbnail: Option<String>,
    pub size: i64,
    pub hash: String,
    pub format: String,
    pub views: i32,
    #[sqlx(try_from = "String")]
    pub status: ImageStatus,
    pub expires_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    #[serde(skip)]
    #[sqlx(default)]
    pub total_count: Option<i64>,
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
            created_at: image.created_at,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::ImageStatus;

    #[test]
    fn image_status_parses_legacy_string_values() {
        assert_eq!(ImageStatus::parse("active"), ImageStatus::Active);
        assert_eq!(ImageStatus::parse("DELETED"), ImageStatus::Deleted);
        assert_eq!(ImageStatus::parse("other"), ImageStatus::Unknown);
    }
}
