//! 认证服务
//!
//! 提供密码哈希、JWT令牌生成和验证等功能

mod password;
#[cfg(test)]
mod tests;
mod tokens;

use crate::config::Config;

#[derive(Clone)]
pub struct AuthService {
    pub(super) jwt_secret: String,
    pub(super) session_ttl_seconds: u64,
}

impl AuthService {
    pub const ACCESS_TOKEN_TTL_SECONDS: u64 = 15 * 60;

    pub fn new(config: &Config) -> Result<Self, String> {
        let jwt_secret = std::env::var("JWT_SECRET").map_err(|_| {
            format!(
                "JWT_SECRET environment variable is required. Please set a secure random string (at least {} characters)",
                config.server.jwt_secret_min_length
            )
        })?;

        if jwt_secret.len() < config.server.jwt_secret_min_length {
            return Err(format!(
                "JWT_SECRET must be at least {} characters long. Current length: {}",
                config.server.jwt_secret_min_length,
                jwt_secret.len()
            ));
        }

        #[cfg(debug_assertions)]
        {
            if jwt_secret.len() < 32 {
                eprintln!(
                    "WARNING: Using short JWT secret in debug mode. Use a longer secret in production!"
                );
            }
        }

        Ok(Self {
            jwt_secret,
            session_ttl_seconds: config.cookie.max_age_seconds.max(1),
        })
    }

    pub fn access_token_ttl_seconds(&self) -> u64 {
        Self::ACCESS_TOKEN_TTL_SECONDS
    }

    pub fn session_ttl_seconds(&self) -> u64 {
        self.session_ttl_seconds
    }
}
