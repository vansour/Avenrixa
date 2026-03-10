use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize)]
pub struct BootstrapStatusResponse {
    pub mode: String,
    pub database_kind: String,
    pub database_configured: bool,
    pub database_url_masked: Option<String>,
    pub database_max_connections: Option<u32>,
    pub restart_required: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub runtime_error: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct UpdateBootstrapDatabaseConfigRequest {
    pub database_kind: String,
    pub database_url: String,
    pub database_max_connections: Option<u32>,
}

#[derive(Debug, Clone, Serialize)]
pub struct UpdateBootstrapDatabaseConfigResponse {
    pub database_kind: String,
    pub database_configured: bool,
    pub database_url_masked: String,
    pub database_max_connections: u32,
    pub restart_required: bool,
}
