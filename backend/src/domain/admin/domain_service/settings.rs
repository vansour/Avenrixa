use super::AdminDomainService;
use crate::db::DatabasePool;
use crate::error::AppError;
use crate::models::Setting;
use crate::runtime_settings::{admin_setting_policy, mask_admin_setting_value};

#[derive(sqlx::FromRow)]
struct SettingRow {
    key: String,
    value: String,
}

impl AdminDomainService {
    pub async fn get_settings(&self) -> Result<Vec<Setting>, AppError> {
        let rows: Vec<SettingRow> = match &self.database {
            DatabasePool::Postgres(pool) => {
                sqlx::query_as("SELECT key, value FROM settings ORDER BY key")
                    .fetch_all(pool)
                    .await?
            }
            DatabasePool::Sqlite(pool) => {
                sqlx::query_as("SELECT key, value FROM settings ORDER BY key")
                    .fetch_all(pool)
                    .await?
            }
        };

        Ok(rows
            .into_iter()
            .map(|row| {
                let policy = admin_setting_policy(&row.key);
                Setting {
                    key: row.key.clone(),
                    value: mask_admin_setting_value(&row.key, &row.value),
                    editable: policy.editable,
                    sensitive: policy.sensitive,
                    masked: policy.masked,
                    requires_confirmation: policy.requires_confirmation,
                }
            })
            .collect())
    }
}
