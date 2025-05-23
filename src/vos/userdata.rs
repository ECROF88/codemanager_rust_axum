use chrono::{DateTime, Utc};
use sqlx::FromRow;

#[derive(serde::Serialize, Debug)]
pub struct UserData {
    pub username: String,
    pub email: String,
    // 其他需要的字段
    pub avatar: Option<String>,
}

// interface UserVo {
//   phone: string;
//   email: string;
//   department_name: string;
//   create_time: string;
// }
#[derive(serde::Serialize, Debug, FromRow)]
pub struct MessagePageUserData {
    pub phone: String,
    pub email: String,
    pub department_name: Option<String>,
    #[sqlx(rename = "create_time")]
    pub create_time: DateTime<Utc>,
}
