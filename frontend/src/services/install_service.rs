use crate::services::api_client::ApiClient;
use crate::types::api::{
    BootstrapStatusResponse, InstallBootstrapRequest, InstallBootstrapResponse,
    InstallStatusResponse, UpdateBootstrapDatabaseConfigRequest,
    UpdateBootstrapDatabaseConfigResponse,
};
use crate::types::errors::Result;

#[derive(Clone)]
pub struct InstallService {
    api_client: ApiClient,
}

impl InstallService {
    pub fn new(api_client: ApiClient) -> Self {
        Self { api_client }
    }

    pub async fn get_bootstrap_status(&self) -> Result<BootstrapStatusResponse> {
        self.api_client.get_json("/api/v1/bootstrap/status").await
    }

    pub async fn update_bootstrap_database_config(
        &self,
        req: UpdateBootstrapDatabaseConfigRequest,
    ) -> Result<UpdateBootstrapDatabaseConfigResponse> {
        self.api_client
            .put_json_response("/api/v1/bootstrap/database-config", &req)
            .await
    }

    pub async fn get_install_status(&self) -> Result<InstallStatusResponse> {
        self.api_client.get_json("/api/v1/install/status").await
    }

    pub async fn bootstrap_installation(
        &self,
        req: InstallBootstrapRequest,
    ) -> Result<InstallBootstrapResponse> {
        self.api_client
            .post_json_response("/api/v1/install/bootstrap", &req)
            .await
    }
}
