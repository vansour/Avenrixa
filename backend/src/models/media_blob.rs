use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct MediaBlob {
    pub storage_key: String,
    pub media_kind: String,
    pub content_hash: Option<String>,
    pub ref_count: i64,
    pub status: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl MediaBlob {
    pub fn is_deleted(&self) -> bool {
        self.status == "deleted"
    }
}
