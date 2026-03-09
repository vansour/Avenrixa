use crate::services::api_client::ApiClient;
use crate::types::api::{AdminSettingsConfig, UpdateAdminSettingsConfigRequest};
use crate::types::errors::Result;

/// 管理员设置服务
#[derive(Clone)]
pub struct SettingsService {
    api_client: ApiClient,
}

impl SettingsService {
    pub fn new(api_client: ApiClient) -> Self {
        Self { api_client }
    }

    pub async fn get_admin_settings_config(&self) -> Result<AdminSettingsConfig> {
        self.api_client.get_json("/api/v1/settings/config").await
    }

    pub async fn update_admin_settings_config(
        &self,
        req: UpdateAdminSettingsConfigRequest,
    ) -> Result<AdminSettingsConfig> {
        self.api_client
            .put_json_response("/api/v1/settings/config", &req)
            .await
    }
}
