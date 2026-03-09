use super::AuthService;
use bcrypt::{DEFAULT_COST, hash, verify};
use uuid::Uuid;

impl AuthService {
    pub fn hash_password(password: &str) -> anyhow::Result<String> {
        Ok(hash(password, DEFAULT_COST)?)
    }

    pub fn verify_password(password: &str, hash: &str) -> anyhow::Result<bool> {
        Ok(verify(password, hash)?)
    }

    pub fn generate_reset_token() -> String {
        Uuid::new_v4().to_string()[..32]
            .to_uppercase()
            .chars()
            .map(|character| if character == '-' { 'A' } else { character })
            .collect()
    }

    pub fn is_reset_token_strong(token: &str) -> bool {
        token.len() >= 32 && token.len() <= 64
    }
}
