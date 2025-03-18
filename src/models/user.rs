use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct User {
    pub id: Option<i32>, // 数据库自增ID
    pub username: String,
    pub email: String,
    pub password: String,
    pub avatar: Option<String>,
    pub created_at: DateTime<Utc>,
}
impl User {
    pub fn new(username: String, email: String, password: String) -> Self {
        User {
            id: None, // 新建用户时ID为None
            username,
            email,
            password, // 注意：实际使用时应该先加密
            avatar: None,
            created_at: Utc::now(),
        }
    }
}
