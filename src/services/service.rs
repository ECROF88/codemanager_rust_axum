use r2d2::Pool;
use redis::{Client, Commands};

use crate::dtos::request::{self};
use crate::gitmodule::{CommitInfo, GitManager};
use crate::models::user::User;
use crate::shared::error::AppError;
use crate::shared::{jwt, setting};
use crate::vos::ReposVo;
use crate::vos::userdata::UserData;

#[derive(Clone)]
pub struct AppState {
    pub auth_service: AuthService,
    pub git_service: GitService,
}

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

    fn validate_user_pass(&self, username: &str, password: &str) -> Result<(), AppError> {
        let mut conn = self
            .pool
            .get()
            .map_err(|_| AppError::InternalServerError("Redis connection failed".to_string()))?;

        let user_exists: bool = conn
            .exists(format!("user:{}", username))
            .map_err(|_| AppError::InternalServerError("Redis operation failed".to_string()))?;

        if !user_exists {
            return Err(AppError::Unauthorized(
                "Invalid username or password".to_string(),
            ));
        }

        let user_json: String = conn
            .get(format!("user:{}", username))
            .map_err(|_| AppError::InternalServerError("Redis operation failed".to_string()))?;

        let user: User = serde_json::from_str(&user_json)
            .map_err(|_| AppError::InternalServerError("Failed to parse user data".to_string()))?;

        if user.password != password {
            return Err(AppError::Unauthorized(
                "Invalid username or password".to_string(),
            ));
        }

        Ok(())
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
        // 使用user_id 作为key
        // let _: () = conn
        //     .set(user_id, user_json)
        //     .map_err(|_| AppError::InternalServerError("Failed to save user".to_string()))?;

        println!("already insert user ");

        self.generate_repopath(&payload.username).await?;
        Ok(())
    }

    async fn generate_repopath(&self, user_name: &str) -> Result<(), AppError> {
        let setting = setting::get_config();
        let base_path = std::str::from_utf8(&setting.git_path.repositories_path)
            .map_err(|e| AppError::InternalServerError(format!("Invalid UTF-8 in path: {}", e)))?;

        // 构建用户特定的仓库路径
        let user_path = format!("{}/{}", base_path, user_name);

        // 创建目录
        std::fs::create_dir_all(&user_path).map_err(|e| {
            AppError::InternalServerError(format!("Failed to create directory: {}", e))
        })?;

        println!(
            "Created repository path for user id:{} path:{}",
            user_name, user_path
        );

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

        if self.validate_user_pass(&username, &password).is_ok() {
            jwt::generate_token(&username)
                .map_err(|_| AppError::InternalServerError("Failed to generate token".to_string()))
        } else {
            Err(AppError::Unauthorized(
                "validate user password not success".to_string(),
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

#[derive(Clone)]
pub struct GitService {
    git_manager: GitManager,
    pool: Pool<Client>,
}

impl GitService {
    pub fn new() -> Self {
        let redis = Client::open("redis://127.0.0.1:6379").expect("Failed to connect to Redis");
        let pool = r2d2::Pool::builder().build(redis).unwrap();
        let setting = setting::get_config();
        let base_path = &setting.git_path.repositories_path;

        let base_path_str =
            std::str::from_utf8(base_path).expect("Invalid UTF-8 sequence in base path");
        let git_manager = GitManager::new(base_path_str);

        Self { git_manager, pool }
    }

    pub async fn clone_repo_for_user(
        &self,
        user_id: &str,
        repo_url: &str,
        repo_name: &str,
    ) -> Result<String, AppError> {
        self.git_manager
            .clone_repository_for_user(user_id, repo_url, repo_name)
            .await
    }

    // 用户提交更改的方法
    pub async fn commit_changes(
        &self,
        user_id: &str,
        repo_name: &str,
        message: &str,
        paths: &[&str],
        // user_data: &UserData, // 从认证服务获取的用户数据
    ) -> Result<String, AppError> {
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
        // let email = user.email;

        self.git_manager.commit_for_user(
            user_id,
            repo_name,
            message,
            paths,
            // &user_data.username,
            // &user_data.email,
            &user.email,
        )
    }

    pub async fn get_repo_commit_histories(
        &self,
        user_id: &str,
        repo_name: &str,
        limit: usize,
    ) -> Result<Vec<CommitInfo>, AppError> {
        self.git_manager
            .get_commit_histories(user_id, repo_name, limit)
    }

    pub async fn get_repos_data_for_users(&self, user_id: &str) -> Result<Vec<ReposVo>, AppError> {
        self.git_manager.get_repos_data_for_users(user_id)
    }
}
