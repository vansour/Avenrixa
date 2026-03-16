use crate::services::api_client::ApiClient;
use crate::types::api::{
    BootstrapStatusResponse, InstallBootstrapRequest, InstallBootstrapResponse,
    InstallStatusResponse, StorageDirectoryBrowseResponse, TestS3StorageConfigRequest,
    TestS3StorageConfigResponse, UpdateBootstrapDatabaseConfigRequest,
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

    pub async fn browse_storage_directories(
        &self,
        path: Option<&str>,
    ) -> Result<StorageDirectoryBrowseResponse> {
        let url = match path.map(str::trim).filter(|value| !value.is_empty()) {
            Some(path) => format!(
                "/api/v1/install/storage-directories?path={}",
                urlencoding::encode(path)
            ),
            None => "/api/v1/install/storage-directories".to_string(),
        };
        self.api_client.get_json(&url).await
    }

    pub async fn bootstrap_installation(
        &self,
        req: InstallBootstrapRequest,
    ) -> Result<InstallBootstrapResponse> {
        self.api_client
            .post_json_response("/api/v1/install/bootstrap", &req)
            .await
    }

    pub async fn test_s3_storage_config(
        &self,
        req: TestS3StorageConfigRequest,
    ) -> Result<TestS3StorageConfigResponse> {
        self.api_client
            .post_json_response("/api/v1/install/storage/s3/test", &req)
            .await
    }
}
