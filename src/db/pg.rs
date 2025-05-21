use crate::{
    models::message::{MessageStatus, MessageType},
    shared::error::AppError,
};

#[derive(Clone)]
pub struct PostgrePool {
    pub pool: sqlx::PgPool,
}

impl PostgrePool {
    pub async fn new(database_url: &str) -> Self {
        let pool = sqlx::PgPool::connect(database_url)
            .await
            .expect("Failed to create database connection pool");
        PostgrePool { pool }
    }

    pub async fn close(&self) {
        self.pool.close().await;
    }

    pub async fn add_message_for_users(
        &self,
        usernames: &Vec<String>, // Changed from user_id_vec for clarity, assuming these are usernames
        content_str: String,     // Changed from msg for clarity
        msg_type: MessageType,
    ) -> Result<(), AppError> {
        if usernames.is_empty() {
            return Ok(()); // Nothing to do
        }

        // Start a transaction
        let mut tx = self.pool.begin().await.map_err(|e| {
            AppError::InternalServerError(format!("Failed to begin transaction: {}", e))
        })?;

        // SQL query uses 'username' and 'type' column names from your schema
        let query_str = r#"
            INSERT INTO messages (username, content, type, status) 
            VALUES ($1, $2, $3, $4)
        "#; // Added status with default 'unread'

        for username in usernames {
            sqlx::query(query_str)
                .bind(username) // username is TEXT in DB, matches String here
                .bind(&content_str)
                .bind(&msg_type.to_string()) // MessageType has #[derive(sqlx::Type)]
                .bind(MessageStatus::default()) // Explicitly set default status
                .execute(&mut *tx) // Use the transaction
                .await
                .map_err(|e| {
                    AppError::InternalServerError(format!(
                        "Database insert failed for user '{}': {}",
                        username, e
                    ))
                })?;
        }

        // Commit the transaction
        tx.commit().await.map_err(|e| {
            AppError::InternalServerError(format!("Failed to commit transaction: {}", e))
        })?;

        Ok(())
    }
}
