use async_trait::async_trait;
use uuid::Uuid;

use super::sql::CATEGORY_SELECT_COLUMNS;
use super::{CategoryRepository, PostgresCategoryRepository};
use crate::models::Category;

#[async_trait]
impl CategoryRepository for PostgresCategoryRepository {
    async fn find_categories_by_user(&self, user_id: Uuid) -> Result<Vec<Category>, sqlx::Error> {
        sqlx::query_as::<_, Category>(&format!(
            "SELECT {} FROM categories WHERE user_id = $1 ORDER BY created_at DESC",
            CATEGORY_SELECT_COLUMNS
        ))
        .bind(user_id)
        .fetch_all(&self.pool)
        .await
    }

    async fn find_category_by_id(&self, id: Uuid) -> Result<Option<Category>, sqlx::Error> {
        sqlx::query_as::<_, Category>(&format!(
            "SELECT {} FROM categories WHERE id = $1",
            CATEGORY_SELECT_COLUMNS
        ))
        .bind(id)
        .fetch_optional(&self.pool)
        .await
    }

    async fn create_category(&self, category: &Category) -> Result<(), sqlx::Error> {
        sqlx::query(
            "INSERT INTO categories (id, user_id, name, created_at) VALUES ($1, $2, $3, $4)",
        )
        .bind(category.id)
        .bind(category.user_id)
        .bind(&category.name)
        .bind(category.created_at)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    async fn delete_category(&self, id: Uuid) -> Result<(), sqlx::Error> {
        sqlx::query("DELETE FROM categories WHERE id = $1")
            .bind(id)
            .execute(&self.pool)
            .await?;

        Ok(())
    }
}
