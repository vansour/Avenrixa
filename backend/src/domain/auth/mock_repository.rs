use async_trait::async_trait;
use uuid::Uuid;
use crate::models::User;
use crate::domain::auth::repository::AuthRepository;
use std::sync::Arc;

pub struct MockAuthRepository {
    pub users: Arc<std::sync::Mutex<Vec<User>>>,
}

impl MockAuthRepository {
    pub fn new() -> Self {
        Self {
            users: Arc::new(std::sync::Mutex::new(Vec::new())),
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
}
