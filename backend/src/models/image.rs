use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(from = "String", into = "String")]
pub enum ImageStatus {
    Active,
    Deleted,
    Unknown,
}

impl ImageStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Active => "active",
            Self::Deleted => "deleted",
            Self::Unknown => "unknown",
        }
    }

    pub fn parse(value: &str) -> Self {
        match value.trim().to_ascii_lowercase().as_str() {
            "active" => Self::Active,
            "deleted" => Self::Deleted,
            _ => Self::Unknown,
        }
    }

    pub fn is_active(self) -> bool {
        matches!(self, Self::Active)
    }
}

impl From<String> for ImageStatus {
    fn from(value: String) -> Self {
        Self::parse(&value)
    }
}

impl From<ImageStatus> for String {
    fn from(value: ImageStatus) -> Self {
        value.as_str().to_string()
    }
}

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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageResponse {
    pub image_key: String,
    pub filename: String,
    pub size: i64,
    pub format: String,
    pub views: i32,
    pub status: ImageStatus,
    pub expires_at: Option<DateTime<Utc>>,
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
            created_at: image.created_at,
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct DeleteRequest {
    pub image_keys: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub struct SetExpiryRequest {
    pub expires_at: Option<DateTime<Utc>>,
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
