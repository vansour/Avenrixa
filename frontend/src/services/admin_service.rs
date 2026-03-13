use crate::services::api_client::ApiClient;
use crate::types::api::{
    AdminUserSummary, AuditLogResponse, BackupFileSummary, BackupResponse,
    BackupRestorePrecheckResponse, BackupRestoreScheduleResponse, BackupRestoreStatusResponse,
    HealthStatus, PaginationParams, Setting, SystemStats, UpdateSettingRequest, UserUpdateRequest,
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

    pub async fn get_backups(&self) -> Result<Vec<BackupFileSummary>> {
        self.api_client.get_json("/api/v1/backups").await
    }

    pub async fn get_backup_restore_status(&self) -> Result<BackupRestoreStatusResponse> {
        self.api_client
            .get_json("/api/v1/backup-restore/status")
            .await
    }

    pub fn backup_download_url(&self, filename: &str) -> String {
        self.api_client.url(&format!(
            "/api/v1/backups/{}",
            urlencoding::encode(filename)
        ))
    }

    pub async fn precheck_backup_restore(
        &self,
        filename: &str,
    ) -> Result<BackupRestorePrecheckResponse> {
        let url = format!(
            "/api/v1/backups/{}/restore/precheck",
            urlencoding::encode(filename)
        );
        self.api_client
            .post_json_response(&url, &serde_json::json!({}))
            .await
    }

    pub async fn schedule_backup_restore(
        &self,
        filename: &str,
    ) -> Result<BackupRestoreScheduleResponse> {
        let url = format!("/api/v1/backups/{}/restore", urlencoding::encode(filename));
        self.api_client
            .post_json_response(&url, &serde_json::json!({}))
            .await
    }

    pub async fn delete_backup(&self, filename: &str) -> Result<()> {
        let url = format!("/api/v1/backups/{}", urlencoding::encode(filename));
        self.api_client.delete(&url).await
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_query_params_keeps_declared_pagination_order() {
        let params = PaginationParams {
            page: Some(3),
            page_size: Some(50),
            tag: None,
        };

        assert_eq!(AdminService::build_query_params(&params), "page=3&page_size=50");
    }

    #[test]
    fn build_query_params_omits_absent_values() {
        let params = PaginationParams {
            page: None,
            page_size: Some(25),
            tag: Some("ignored".to_string()),
        };

        assert_eq!(AdminService::build_query_params(&params), "page_size=25");
    }

    #[test]
    fn backup_download_url_encodes_filename_against_api_base() {
        let service = AdminService::new(ApiClient::new("https://img.example.com/app/".to_string()));

        let url = service.backup_download_url("nightly backup.sql.gz");

        assert_eq!(
            url,
            "https://img.example.com/app/api/v1/backups/nightly%20backup.sql.gz"
        );
    }
}
