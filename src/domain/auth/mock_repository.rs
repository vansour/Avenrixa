use async_trait::async_trait;
use uuid::Uuid;
use crate::models::User;
use crate::domain::auth::repository::{AuthRepository, PasswordResetToken};
use std::sync::{Arc, Mutex};

pub struct MockAuthRepository {
    pub users: Arc<Mutex<Vec<User>>>,
    pub tokens: Arc<Mutex<Vec<PasswordResetToken>>>,
}

impl MockAuthRepository {
    pub fn new() -> Self {
        Self {
            users: Arc::new(Mutex::new(Vec::new())),
            tokens: Arc::new(Mutex::new(Vec::new())),
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

    async fn create_user(&self, user: &User) -> Result<(), sqlx::Error> {
        let mut users = self.users.lock().unwrap();
        users.push(user.clone());
        Ok(())
    }

    async fn update_user_password(&self, user_id: Uuid, password_hash: &str) -> Result<(), sqlx::Error> {
        let mut users = self.users.lock().unwrap();
        if let Some(user) = users.iter_mut().find(|u| u.id == user_id) {
            user.password_hash = password_hash.to_string();
        }
        Ok(())
    }

    async fn create_password_reset_token(&self, token: &PasswordResetToken) -> Result<(), sqlx::Error> {
        let mut tokens = self.tokens.lock().unwrap();
        tokens.push(token.clone());
        Ok(())
    }

    async fn find_password_reset_token(&self, token: &str) -> Result<Option<PasswordResetToken>, sqlx::Error> {
        let tokens = self.tokens.lock().unwrap();
        Ok(tokens.iter().find(|t| t.token == token).cloned())
    }

    async fn mark_token_used(&self, token_id: Uuid) -> Result<(), sqlx::Error> {
        let mut tokens = self.tokens.lock().unwrap();
        if let Some(token) = tokens.iter_mut().find(|t| t.id == token_id) {
            token.used_at = Some(chrono::Utc::now());
        }
        Ok(())
    }
}
