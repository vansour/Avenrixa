use crate::services::api_client::ApiClient;
use crate::types::api::{
    AdminUserSummary, AuditLogResponse, BackupResponse, HealthStatus, PaginationParams, Setting,
    SystemStats, UpdateSettingRequest, UserUpdateRequest,
};
use crate::types::errors::Result;

/// 管理员能力服务
#[derive(Clone)]
pub struct AdminService {
    api_client: ApiClient,
}

impl AdminService {
    pub fn new(api_client: ApiClient) -> Self {
        Self { api_client }
    }

    pub async fn get_health_status(&self) -> Result<HealthStatus> {
        self.api_client.get_json("/health").await
    }

    pub async fn get_system_stats(&self) -> Result<SystemStats> {
        self.api_client.get_json("/api/v1/stats").await
    }

    pub async fn get_users(&self) -> Result<Vec<AdminUserSummary>> {
        self.api_client.get_json("/api/v1/users").await
    }

    pub async fn update_user_role(&self, user_id: &str, role: String) -> Result<()> {
        let url = format!("/api/v1/users/{}", urlencoding::encode(user_id));
        self.api_client
            .put_json_no_response(&url, &UserUpdateRequest { role: Some(role) })
            .await
    }

    pub async fn get_audit_logs(&self, params: PaginationParams) -> Result<AuditLogResponse> {
        let query_params = Self::build_query_params(&params);
        let url = if query_params.is_empty() {
            "/api/v1/audit-logs".to_string()
        } else {
            format!("/api/v1/audit-logs?{}", query_params)
        };

        self.api_client.get_json(&url).await
    }

    pub async fn get_raw_settings(&self) -> Result<Vec<Setting>> {
        self.api_client.get_json("/api/v1/settings").await
    }

    pub async fn update_setting(&self, key: &str, value: String) -> Result<()> {
        let url = format!("/api/v1/settings/{}", urlencoding::encode(key));
        self.api_client
            .put_json_no_response(&url, &UpdateSettingRequest { value })
            .await
    }

    pub async fn cleanup_deleted_files(&self) -> Result<Vec<String>> {
        self.api_client
            .post_json_response("/api/v1/cleanup", &serde_json::json!({}))
            .await
    }

    pub async fn cleanup_expired_images(&self) -> Result<i64> {
        self.api_client
            .post_json_response("/api/v1/cleanup/expired", &serde_json::json!({}))
            .await
    }

    pub async fn backup_database(&self) -> Result<BackupResponse> {
        self.api_client
            .post_json_response("/api/v1/backup", &serde_json::json!({}))
            .await
    }

    fn build_query_params(params: &PaginationParams) -> String {
        let mut query_parts = Vec::new();

        if let Some(page) = params.page {
            query_parts.push(format!("page={}", page));
        }
        if let Some(page_size) = params.page_size {
            query_parts.push(format!("page_size={}", page_size));
        }

        query_parts.join("&")
    }
}
