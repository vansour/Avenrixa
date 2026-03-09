use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use uuid::Uuid;

use crate::domain::image::repository::CategoryRepository;
use crate::models::Category;

pub struct MockCategoryRepository {
    pub categories: Arc<Mutex<Vec<Category>>>,
}

impl MockCategoryRepository {
    pub fn new() -> Self {
        Self {
            categories: Arc::new(Mutex::new(Vec::new())),
        }
    }
}

#[async_trait]
impl CategoryRepository for MockCategoryRepository {
    async fn find_categories_by_user(&self, user_id: Uuid) -> Result<Vec<Category>, sqlx::Error> {
        let categories = self.categories.lock().unwrap();
        Ok(categories
            .iter()
            .filter(|category| category.user_id == user_id)
            .cloned()
            .collect())
    }

    async fn find_category_by_id(&self, id: Uuid) -> Result<Option<Category>, sqlx::Error> {
        let categories = self.categories.lock().unwrap();
        Ok(categories
            .iter()
            .find(|category| category.id == id)
            .cloned())
    }

    async fn create_category(&self, category: &Category) -> Result<(), sqlx::Error> {
        let mut categories = self.categories.lock().unwrap();
        categories.push(category.clone());
        Ok(())
    }

    async fn delete_category(&self, id: Uuid) -> Result<(), sqlx::Error> {
        let mut categories = self.categories.lock().unwrap();
        categories.retain(|category| category.id != id);
        Ok(())
    }
}
