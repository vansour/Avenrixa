use bcrypt::{hash, verify, DEFAULT_COST};
use chrono::{Duration, Utc};
use jsonwebtoken::{decode, encode, Algorithm, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::config::Config;

#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    pub sub: Uuid,
    pub username: String,
    pub role: String,
    pub exp: i64,
    pub iat: i64,
}

#[derive(Clone)]
pub struct AuthService {
    jwt_secret: String,
    expiration_hours: i64,
}

impl AuthService {
    pub fn new(config: &Config) -> Result<Self, String> {
        // 强制要求环境变量配置 JWT_SECRET，不使用默认值
        let jwt_secret = std::env::var("JWT_SECRET")
            .map_err(|_| format!(
                "JWT_SECRET environment variable is required. Please set a secure random string (at least {} characters)",
                config.server.jwt_secret_min_length
            ))?;

        // 验证 JWT 密钥长度
        if jwt_secret.len() < config.server.jwt_secret_min_length {
            return Err(format!(
                "JWT_SECRET must be at least {} characters long. Current length: {}",
                config.server.jwt_secret_min_length,
                jwt_secret.len()
            ));
        }

        // 生产环境警告
        #[cfg(debug_assertions)]
        {
            if jwt_secret.len() < 32 {
                eprintln!("WARNING: Using short JWT secret in debug mode. Use a longer secret in production!");
            }
        }

        Ok(Self {
            jwt_secret,
            expiration_hours: 24 * 7,
        })
    }

    pub fn hash_password(password: &str) -> anyhow::Result<String> {
        Ok(hash(password, DEFAULT_COST)?)
    }

    pub fn verify_password(password: &str, hash: &str) -> anyhow::Result<bool> {
        Ok(verify(password, hash)?)
    }

    pub fn generate_token(&self, user_id: Uuid, username: &str, role: &str) -> anyhow::Result<String> {
        let now = Utc::now();
        let exp = now + Duration::hours(self.expiration_hours);

        let claims = Claims {
            sub: user_id,
            username: username.to_string(),
            role: role.to_string(),
            exp: exp.timestamp(),
            iat: now.timestamp(),
        };

        encode(
            &Header::default(),
            &claims,
            &EncodingKey::from_secret(self.jwt_secret.as_ref()),
        )
        .map_err(Into::into)
    }

    pub fn verify_token(&self, token: &str) -> anyhow::Result<Claims> {
        decode::<Claims>(
            token,
            &DecodingKey::from_secret(self.jwt_secret.as_ref()),
            &Validation::new(Algorithm::HS256),
        )
        .map(|data| data.claims)
        .map_err(Into::into)
    }

    /// 生成随机重置令牌
    pub fn generate_reset_token() -> String {
        Uuid::new_v4().to_string()[..32].to_uppercase().chars().map(|c| if c == '-' { 'A' } else { c }).collect()
    }

    /// 验证重置令牌强度
    pub fn is_reset_token_strong(token: &str) -> bool {
        token.len() >= 32 && token.len() <= 64
    }

    /// 修改密码
    /// 注意：此函数已被弃用，密码修改逻辑已移至 handlers/auth.rs
    #[deprecated(since = "0.1.0", note = "密码修改逻辑请使用 handlers::auth::change_password")]
    pub fn change_password(_current: &str, new: &str) -> anyhow::Result<String> {
        Self::hash_password(new)
    }
}
