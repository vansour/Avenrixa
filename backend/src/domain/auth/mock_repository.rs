use crate::domain::auth::repository::{
    AuthRepository, EmailVerificationStatus, PasswordResetStatus,
};
use crate::models::User;
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use uuid::Uuid;

type TokenState = (Uuid, DateTime<Utc>, bool);
type TokenStore = Arc<Mutex<HashMap<String, TokenState>>>;

#[derive(Clone)]
pub struct MockAuthRepository {
    pub users: Arc<Mutex<Vec<User>>>,
    pub reset_tokens: TokenStore,
    pub verification_tokens: TokenStore,
}

impl MockAuthRepository {
    pub fn new() -> Self {
        Self {
            users: Arc::new(Mutex::new(Vec::new())),
            reset_tokens: Arc::new(Mutex::new(HashMap::new())),
            verification_tokens: Arc::new(Mutex::new(HashMap::new())),
        }
    }
}

#[async_trait]
impl AuthRepository for MockAuthRepository {
    async fn find_user_by_id(&self, id: Uuid) -> Result<Option<User>, sqlx::Error> {
        let users = self.users.lock().unwrap();
        Ok(users.iter().find(|u| u.id == id).cloned())
    }

    async fn find_user_by_email(&self, email: &str) -> Result<Option<User>, sqlx::Error> {
        let users = self.users.lock().unwrap();
        Ok(users.iter().find(|u| u.email == email).cloned())
    }

    async fn create_user(&self, user: &User) -> Result<(), sqlx::Error> {
        let mut users = self.users.lock().unwrap();
        users.push(user.clone());
        Ok(())
    }

    async fn update_user_password(
        &self,
        user_id: Uuid,
        password_hash: &str,
    ) -> Result<(), sqlx::Error> {
        let mut users = self.users.lock().unwrap();
        if let Some(user) = users.iter_mut().find(|u| u.id == user_id) {
            user.password_hash = password_hash.to_string();
        }
        Ok(())
    }

    async fn store_password_reset_token(
        &self,
        user_id: Uuid,
        token_hash: &str,
        expires_at: DateTime<Utc>,
    ) -> Result<(), sqlx::Error> {
        let mut reset_tokens = self.reset_tokens.lock().unwrap();
        reset_tokens.retain(|_, (stored_user_id, _, used)| *stored_user_id != user_id || *used);
        reset_tokens.insert(token_hash.to_string(), (user_id, expires_at, false));
        Ok(())
    }

    async fn reset_password_by_token(
        &self,
        token_hash: &str,
        password_hash: &str,
    ) -> Result<PasswordResetStatus, sqlx::Error> {
        let mut reset_tokens = self.reset_tokens.lock().unwrap();
        let Some((user_id, expires_at, used)) = reset_tokens.get_mut(token_hash) else {
            return Ok(PasswordResetStatus::Invalid);
        };
        if *used {
            return Ok(PasswordResetStatus::Invalid);
        }
        if *expires_at < Utc::now() {
            return Ok(PasswordResetStatus::Expired);
        }

        let mut users = self.users.lock().unwrap();
        let Some(user) = users.iter_mut().find(|u| u.id == *user_id) else {
            return Ok(PasswordResetStatus::Invalid);
        };
        user.password_hash = password_hash.to_string();
        *used = true;
        Ok(PasswordResetStatus::Applied(user.clone()))
    }

    async fn store_email_verification_token(
        &self,
        user_id: Uuid,
        token_hash: &str,
        expires_at: DateTime<Utc>,
    ) -> Result<(), sqlx::Error> {
        let mut verification_tokens = self.verification_tokens.lock().unwrap();
        verification_tokens
            .retain(|_, (stored_user_id, _, used)| *stored_user_id != user_id || *used);
        verification_tokens.insert(token_hash.to_string(), (user_id, expires_at, false));
        Ok(())
    }

    async fn verify_email_by_token(
        &self,
        token_hash: &str,
    ) -> Result<EmailVerificationStatus, sqlx::Error> {
        let mut verification_tokens = self.verification_tokens.lock().unwrap();
        let Some((user_id, expires_at, used)) = verification_tokens.get_mut(token_hash) else {
            return Ok(EmailVerificationStatus::Invalid);
        };
        if *used {
            return Ok(EmailVerificationStatus::Invalid);
        }
        if *expires_at < Utc::now() {
            return Ok(EmailVerificationStatus::Expired);
        }

        let mut users = self.users.lock().unwrap();
        let Some(user) = users.iter_mut().find(|u| u.id == *user_id) else {
            return Ok(EmailVerificationStatus::Invalid);
        };
        user.email_verified_at = Some(Utc::now());
        *used = true;
        Ok(EmailVerificationStatus::Applied(user.clone()))
    }
}
