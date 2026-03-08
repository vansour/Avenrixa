//! 认证服务
//!
//! 提供密码哈希、JWT令牌生成和验证等功能

use bcrypt::{hash, verify, DEFAULT_COST};
use chrono::{Duration, Utc};
use jsonwebtoken::{decode, encode, Algorithm, DecodingKey, EncodingKey, Header, Validation};
use uuid::Uuid;

use super::claims::Claims;
use crate::config::Config;

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

    /// 生成访问令牌（短期，15分钟有效）
    pub fn generate_access_token(&self, user_id: Uuid, username: &str, role: &str) -> anyhow::Result<String> {
        let now = Utc::now();
        let exp = now + Duration::minutes(15);

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

    /// 生成刷新令牌（长期，7天有效）
    pub fn generate_refresh_token(&self, user_id: Uuid) -> anyhow::Result<String> {
        let now = Utc::now();
        let exp = now + Duration::hours(self.expiration_hours);

        let claims = Claims {
            sub: user_id,
            username: "".to_string(), // 刷新令牌不需要用户名
            role: "refresh".to_string(),
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

    /// 验证刷新令牌
    pub fn verify_refresh_token(&self, token: &str) -> anyhow::Result<Uuid> {
        let claims = self.verify_token(token)?;

        // 验证是否为刷新令牌
        if claims.role != "refresh" {
            return Err(anyhow::anyhow!("Not a refresh token"));
        }

        Ok(claims.sub)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 测试用测试配置
    fn test_config() -> Config {
        Config::default()
    }

    /// 创建测试用 AuthService（使用固定密钥）
    fn create_test_service() -> AuthService {
        let config = test_config();
        // SAFETY: 测试中设置环境变量
        unsafe {
            std::env::set_var("JWT_SECRET", "test_secret_key_for_testing_must_be_long_enough_32chars");
        }
        AuthService::new(&config).expect("Failed to create test auth service")
    }

    #[test]
    fn test_hash_password() {
        let password = "test_password_123";
        let hash = AuthService::hash_password(password).expect("Failed to hash password");

        // bcrypt 哈希应该以 $2b$ 开头
        assert!(hash.starts_with("$2b$"));
        assert_ne!(hash, password);
    }

    #[test]
    fn test_verify_password() {
        let password = "test_password_123";
        let hash = AuthService::hash_password(password).expect("Failed to hash password");

        // 正确密码应该验证通过
        assert!(AuthService::verify_password(password, &hash).expect("Failed to verify"));

        // 错误密码应该验证失败
        assert!(!AuthService::verify_password("wrong_password", &hash).expect("Failed to verify"));
    }

    #[test]
    fn test_generate_and_verify_token() {
        let service = create_test_service();
        let user_id = Uuid::new_v4();
        let username = "testuser";
        let role = "user";

        let token = service.generate_token(user_id, username, role).expect("Failed to generate token");

        // 验证令牌
        let claims = service.verify_token(&token).expect("Failed to verify token");
        assert_eq!(claims.sub, user_id);
        assert_eq!(claims.username, username);
        assert_eq!(claims.role, role);
    }

    #[test]
    fn test_generate_reset_token() {
        let token = AuthService::generate_reset_token();

        // 令牌长度应该是 32
        assert_eq!(token.len(), 32);

        // 令牌应该都是大写字母和数字
        assert!(token.chars().all(|c| c.is_ascii_uppercase() || c.is_ascii_digit()));
    }

    #[test]
    fn test_is_reset_token_strong() {
        // 有效令牌
        assert!(AuthService::is_reset_token_strong("ABCDEFGHIJKLMNOPQRSTUVWXYZ123456"));
        assert!(AuthService::is_reset_token_strong(&"A".repeat(64)));

        // 无效令牌
        assert!(!AuthService::is_reset_token_strong("SHORT"));
        assert!(!AuthService::is_reset_token_strong(&"A".repeat(31)));
    }

    #[test]
    fn test_generate_access_token() {
        let service = create_test_service();
        let user_id = Uuid::new_v4();

        let token = service.generate_access_token(user_id, "testuser", "user")
            .expect("Failed to generate access token");

        let claims = service.verify_token(&token).expect("Failed to verify token");
        assert_eq!(claims.sub, user_id);
        assert_eq!(claims.username, "testuser");
        assert_eq!(claims.role, "user");

        // 访问令牌应该有较短的过期时间（15分钟）
        let now = Utc::now().timestamp();
        let expected_exp = now + 15 * 60;
        // 允许 1 秒误差
        assert!((claims.exp - expected_exp).abs() <= 1);
    }

    #[test]
    fn test_generate_and_verify_refresh_token() {
        let service = create_test_service();
        let user_id = Uuid::new_v4();

        let token = service.generate_refresh_token(user_id).expect("Failed to generate refresh token");

        // 验证刷新令牌
        let result = service.verify_refresh_token(&token).expect("Failed to verify refresh token");
        assert_eq!(result, user_id);
    }

    #[test]
    fn test_verify_refresh_token_rejects_non_refresh_token() {
        let service = create_test_service();
        let user_id = Uuid::new_v4();

        // 生成普通访问令牌
        let access_token = service.generate_access_token(user_id, "testuser", "user")
            .expect("Failed to generate access token");

        // 尝试用 verify_refresh_token 验证应该失败
        let result = service.verify_refresh_token(&access_token);
        assert!(result.is_err());
    }

    #[test]
    fn test_invalid_token_rejected() {
        let service = create_test_service();

        // 无效令牌应该被拒绝
        let result = service.verify_token("invalid_token");
        assert!(result.is_err());
    }
}
