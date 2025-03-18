use r2d2::Pool;
use redis::{Client, Commands, Connection};

// use super::super::shared::error::AppError;
// use super::{dtos::request, model::User};
// use super::super::error;
use crate::dtos::request::{self};
use crate::models::user::{self, User};
use crate::shared::error::AppError;
use crate::shared::jwt;
use crate::vos::userdata::UserData;
#[derive(Clone)]
pub struct AuthService {
    // redis: Client,
    pool: Pool<Client>,
}

impl AuthService {
    pub fn new() -> Self {
        let redis = Client::open("redis://127.0.0.1:6379").expect("Failed to connect to Redis");
        // let client = redis::Client::open("redis://127.0.0.1/").unwrap();
        let pool = r2d2::Pool::builder().build(redis).unwrap();
        AuthService { pool }
    }
    pub async fn register(&self, payload: request::RegisterRequest) -> Result<(), AppError> {
        // TODO: 数据库插入逻辑
        // let newuser = User::new(payload.username.clone(), payload.email, payload.password);

        // let mut conn = self
        //     .redis
        //     .get_connection()
        //     .map_err(|_| AppError::InternalServerError("Redis connect failed".to_string()))?;
        let mut conn = self
            .pool
            .get()
            .map_err(|_| AppError::InternalServerError("Redis connect failed".to_string()))?;

        let exist: bool = conn
            .exists(format!("user:{}", payload.username))
            .map_err(|_| AppError::InternalServerError("Redis operation failed".to_string()))?;

        if exist {
            return Err(AppError::BadRequest("Username already exists".to_string()));
        }

        let user_id: i32 = conn
            .incr("user_id_counter", 1)
            .map_err(|_| AppError::InternalServerError("Failed to generate user ID".to_string()))?;

        let newuser = User {
            id: Some(user_id),
            username: payload.username.clone(),
            email: payload.email,
            password: payload.password,
            avatar: None,
            created_at: chrono::Utc::now(),
        };

        let user_json = serde_json::to_string(&newuser)
            .map_err(|_| AppError::InternalServerError("Serialization failed".to_string()))?;

        // 这里目前把username作为key，整个user作为value
        let _: () = conn
            .set(format!("user:{}", payload.username), user_json)
            .map_err(|_| AppError::InternalServerError("Failed to save user".to_string()))?;

        println!("already insert user ");

        Ok(())
    }

    pub async fn login(&self, payload: request::LoginRequest) -> Result<String, AppError> {
        // 验证用户名
        let username = match payload.identity {
            Some(name) => {
                let trimmed = name.trim();
                if trimmed.is_empty() {
                    return Err(AppError::BadRequest("Username cannot be empty".into()));
                }
                trimmed.to_string()
            }
            None => return Err(AppError::BadRequest("Username is required".into())),
        };

        // 验证密码
        let password = match payload.password {
            Some(pwd) => {
                let trimmed = pwd.trim();
                if trimmed.is_empty() {
                    return Err(AppError::BadRequest("Password cannot be empty".into()));
                }
                trimmed.to_string()
            }
            None => return Err(AppError::BadRequest("Password is required".into())),
        };

        // TODO: 数据库查询验证用户名密码

        // 这里模拟验证过程
        if password == "123456" {
            let token = jwt::generate_token(username)
                .map_err(|_| AppError::InternalServerError("Failed to generate token".into()))?;
            Ok(token)
        } else {
            Err(AppError::Unauthorized(
                "Invalid username or password".into(),
            ))
        }
    }

    pub async fn get_user_data(&self, user_id: String) -> Result<UserData, AppError> {
        let mut conn = self
            .pool
            .get()
            .map_err(|_| AppError::InternalServerError("Redis connect failed".to_string()))?;

        // 查找用户
        let user_json: String = conn
            .get(format!("user:{}", user_id))
            .map_err(|_| AppError::NotFound("User not found".to_string()))?;

        let user: User = serde_json::from_str(&user_json)
            .map_err(|_| AppError::InternalServerError("Failed to parse user data".to_string()))?;
        println!("{:#?}", user);
        Ok(UserData {
            username: user.username,
            email: user.email,
            avatar: user.avatar,
        })
    }
}
