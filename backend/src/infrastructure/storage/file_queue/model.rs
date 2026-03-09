use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::DEFAULT_MAX_RETRIES;

fn default_max_retries() -> u8 {
    DEFAULT_MAX_RETRIES
}

/// 文件保存任务
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileSaveTask {
    #[serde(default)]
    pub task_id: String,
    pub image_id: String,
    pub storage_path: String,
    pub thumbnail_path: String,
    pub temp_image_path: String,
    pub thumbnail_data: Vec<u8>,
    #[serde(default)]
    pub attempts: u8,
    #[serde(default = "default_max_retries")]
    pub max_retries: u8,
    #[serde(default)]
    pub result_key: Option<String>,
}

impl FileSaveTask {
    pub(super) fn normalized(mut self) -> Self {
        if self.task_id.is_empty() {
            self.task_id = Uuid::new_v4().to_string();
        }
        if self.max_retries == 0 {
            self.max_retries = DEFAULT_MAX_RETRIES;
        }
        self
    }
}

/// 文件保存结果
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum FileSaveResult {
    Success,
    ImageFailed,
    ThumbnailFailed,
    Cancelled,
}

impl FileSaveResult {
    pub(super) fn as_str(&self) -> &'static str {
        match self {
            Self::Success => "success",
            Self::ImageFailed => "image_failed",
            Self::ThumbnailFailed => "thumbnail_failed",
            Self::Cancelled => "cancelled",
        }
    }

    pub(super) fn from_status(status: &str) -> Option<Self> {
        match status {
            "success" => Some(Self::Success),
            "image_failed" => Some(Self::ImageFailed),
            "thumbnail_failed" => Some(Self::ThumbnailFailed),
            "cancelled" => Some(Self::Cancelled),
            _ => None,
        }
    }
}
