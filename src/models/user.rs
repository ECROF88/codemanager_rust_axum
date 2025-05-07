use std::path::Path;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

#[derive(Debug, Serialize, Deserialize, FromRow)]
pub struct User {
    pub id: Option<i32>, // 数据库自增ID
    pub username: String,
    pub email: String,
    pub password: String,
    pub avatar: Option<String>,
    pub created_at: DateTime<Utc>,
    pub department_id: Option<i32>,
}
impl User {
    pub fn new(username: String, email: String, password: String) -> Self {
        User {
            id: None,
            username,
            email,
            password,
            avatar: None,
            created_at: Utc::now(),
            department_id: None,
        }
    }
}

#[derive(Debug, Serialize, Deserialize, FromRow)]
pub struct DepartMnet {
    name: String,
}

#[derive(Debug, Serialize, Deserialize, FromRow)]
pub struct UserRepo {
    pub id: Option<i32>,
    pub user_id: i32,
    pub path: String,
}
