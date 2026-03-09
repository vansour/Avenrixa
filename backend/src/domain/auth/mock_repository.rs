use crate::domain::auth::repository::{AuthRepository, PasswordResetStatus};
use crate::models::User;
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use std::collections::HashMap;
use std::sync::Arc;
use uuid::Uuid;

#[derive(Clone)]
pub struct MockAuthRepository {
    pub users: Arc<std::sync::Mutex<Vec<User>>>,
    pub reset_tokens: Arc<std::sync::Mutex<HashMap<String, (Uuid, DateTime<Utc>, bool)>>>,
}

impl MockAuthRepository {
    pub fn new() -> Self {
        Self {
            users: Arc::new(std::sync::Mutex::new(Vec::new())),
            reset_tokens: Arc::new(std::sync::Mutex::new(HashMap::new())),
        }
    }
}

#[async_trait]
impl AuthRepository for MockAuthRepository {
    async fn find_user_by_id(&self, id: Uuid) -> Result<Option<User>, sqlx::Error> {
        let users = self.users.lock().unwrap();
        Ok(users.iter().find(|u| u.id == id).cloned())
    }

    async fn find_user_by_username(&self, username: &str) -> Result<Option<User>, sqlx::Error> {
        let users = self.users.lock().unwrap();
        Ok(users.iter().find(|u| u.username == username).cloned())
    }

    async fn find_user_by_identity(&self, identity: &str) -> Result<Option<User>, sqlx::Error> {
        let users = self.users.lock().unwrap();
        Ok(users
            .iter()
            .find(|u| u.username == identity || u.email.as_deref() == Some(identity))
            .cloned())
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
}
