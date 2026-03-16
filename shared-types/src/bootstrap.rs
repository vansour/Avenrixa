use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(from = "String", into = "String")]
pub enum BootstrapDatabaseKind {
    Postgres,
    MySql,
    Sqlite,
    Unknown,
}

impl BootstrapDatabaseKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Postgres => "postgresql",
            Self::MySql => "mysql",
            Self::Sqlite => "sqlite",
            Self::Unknown => "unknown",
        }
    }

    pub fn parse(value: &str) -> Self {
        match value.trim().to_ascii_lowercase().as_str() {
            "postgresql" | "postgres" => Self::Postgres,
            "mysql" | "mariadb" => Self::MySql,
            "sqlite" => Self::Sqlite,
            _ => Self::Unknown,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Postgres => "PostgreSQL",
            Self::MySql => "MySQL / MariaDB",
            Self::Sqlite => "SQLite",
            Self::Unknown => "未识别数据库",
        }
    }
}

impl From<String> for BootstrapDatabaseKind {
    fn from(value: String) -> Self {
        Self::parse(&value)
    }
}

impl From<BootstrapDatabaseKind> for String {
    fn from(value: BootstrapDatabaseKind) -> Self {
        value.as_str().to_string()
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BootstrapStatusResponse {
    pub mode: String,
    pub database_kind: BootstrapDatabaseKind,
    pub database_configured: bool,
    pub database_url_masked: Option<String>,
    pub cache_configured: bool,
    pub cache_url_masked: Option<String>,
    pub restart_required: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub runtime_error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateBootstrapDatabaseConfigRequest {
    pub database_kind: BootstrapDatabaseKind,
    pub database_url: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UpdateBootstrapDatabaseConfigResponse {
    pub database_kind: BootstrapDatabaseKind,
    pub database_configured: bool,
    pub database_url_masked: String,
    pub restart_required: bool,
}
