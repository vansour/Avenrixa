use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(from = "String", into = "String")]
pub enum UserRole {
    Admin,
    User,
    Unknown,
}

impl UserRole {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Admin => "admin",
            Self::User => "user",
            Self::Unknown => "unknown",
        }
    }

    pub fn parse(value: &str) -> Self {
        match value.trim().to_ascii_lowercase().as_str() {
            "admin" => Self::Admin,
            "user" => Self::User,
            _ => Self::Unknown,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Admin => "管理员",
            Self::User => "普通用户",
            Self::Unknown => "未知角色",
        }
    }

    pub fn surface_class(self) -> &'static str {
        match self {
            Self::Admin => "is-admin",
            Self::User | Self::Unknown => "is-user",
        }
    }

    pub fn is_admin(self) -> bool {
        matches!(self, Self::Admin)
    }
}

impl From<String> for UserRole {
    fn from(value: String) -> Self {
        Self::parse(&value)
    }
}

impl From<UserRole> for String {
    fn from(value: UserRole) -> Self {
        value.as_str().to_string()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(from = "String", into = "String")]
pub enum HealthState {
    Healthy,
    Degraded,
    Unhealthy,
    Disabled,
    Bootstrapping,
    Unknown,
}

impl HealthState {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Healthy => "healthy",
            Self::Degraded => "degraded",
            Self::Unhealthy => "unhealthy",
            Self::Disabled => "disabled",
            Self::Bootstrapping => "bootstrapping",
            Self::Unknown => "unknown",
        }
    }

    pub fn parse(value: &str) -> Self {
        match value.trim().to_ascii_lowercase().as_str() {
            "healthy" => Self::Healthy,
            "degraded" => Self::Degraded,
            "unhealthy" => Self::Unhealthy,
            "disabled" => Self::Disabled,
            "bootstrapping" => Self::Bootstrapping,
            _ => Self::Unknown,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Healthy => "健康",
            Self::Degraded => "降级",
            Self::Unhealthy => "异常",
            Self::Disabled => "已禁用",
            Self::Bootstrapping => "引导中",
            Self::Unknown => "异常",
        }
    }

    pub fn surface_class(self) -> &'static str {
        match self {
            Self::Healthy | Self::Disabled => "is-healthy",
            Self::Degraded | Self::Unhealthy | Self::Bootstrapping | Self::Unknown => {
                "is-unhealthy"
            }
        }
    }
}

impl From<String> for HealthState {
    fn from(value: String) -> Self {
        Self::parse(&value)
    }
}

impl From<HealthState> for String {
    fn from(value: HealthState) -> Self {
        value.as_str().to_string()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(from = "String", into = "String")]
pub enum StorageBackendKind {
    Local,
    Unknown,
}

impl StorageBackendKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Local => "local",
            Self::Unknown => "unknown",
        }
    }

    pub fn parse(value: &str) -> Self {
        match value.trim().to_ascii_lowercase().as_str() {
            "local" => Self::Local,
            _ => Self::Unknown,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Local => "本地目录",
            Self::Unknown => "未知后端",
        }
    }
}

impl From<String> for StorageBackendKind {
    fn from(value: String) -> Self {
        Self::parse(&value)
    }
}

impl From<StorageBackendKind> for String {
    fn from(value: StorageBackendKind) -> Self {
        value.as_str().to_string()
    }
}
