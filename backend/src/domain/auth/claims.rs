//! JWT Claims 结构体

use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    pub sub: Uuid,
    pub email: String,
    pub role: String,
    #[serde(default)]
    pub token_version: u64,
    pub exp: i64,
    pub iat: i64,
}
