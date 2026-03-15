use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
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

/// 图片项
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ImageItem {
    pub image_key: String,
    pub filename: String,
    pub size: i64,
    pub format: String,
    pub views: i32,
    pub status: ImageStatus,
    pub expires_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

impl ImageItem {
    pub fn url(&self) -> String {
        format!("/images/{}", self.filename)
    }

    pub fn thumbnail_url(&self) -> String {
        format!("/thumbnails/{}.webp", self.image_key)
    }

    pub fn display_name(&self) -> String {
        self.filename.clone()
    }

    pub fn created_at_label(&self) -> String {
        self.created_at.format("%Y-%m-%d %H:%M").to_string()
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

    pub fn is_expired(&self) -> bool {
        if let Some(expires_at) = self.expires_at {
            expires_at < Utc::now()
        } else {
            false
        }
    }
}

/// 分页响应
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaginatedResponse<T> {
    pub data: Vec<T>,
    pub total: u64,
    pub page: u32,
    pub page_size: u32,
}

#[cfg(test)]
mod tests {
    use super::ImageStatus;

    #[test]
    fn image_status_unknown_values_fall_back_safely() {
        assert_eq!(ImageStatus::parse("active"), ImageStatus::Active);
        assert_eq!(ImageStatus::parse("DELETED"), ImageStatus::Deleted);
        assert_eq!(ImageStatus::parse("archived"), ImageStatus::Unknown);
    }
}
