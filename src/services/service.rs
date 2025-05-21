use std::sync::Arc;

use futures::TryFutureExt;
use r2d2::Pool;
use redis::{Client, Commands};
use sqlx::{PgPool, Row, query, query_as};
use tracing::info;

use crate::db::pg::PostgrePool;
use crate::dtos::request::{self, RegisterRequest, UserUpdateRequest};
use crate::gitmodule::structs::{CommitDetail, CommitInfo, GitFileEntry, WebSocketManager};
use crate::gitmodule::{GitManager, structs};
use crate::models::message::{Message, MessageCreate, MessageType};
use crate::models::user::User;
use crate::models::{self, message};
use crate::shared::error::AppError;
use crate::shared::{jwt, setting};
use crate::vos::ReposVo;
use crate::vos::userdata::UserData;

#[derive(Clone)]
pub struct AppState {
    pub redis: RedisPool,
    pub git_service: GitService,
    pub ws_manager: WebSocketManager,
    // pub pg_db: PgPool,
    pub pg_db: PostgrePool,
}
impl AppState {
    pub async fn init_app() -> Result<Arc<AppState>, AppError> {
        // let pg_db = PgPool::connect("postgresql://postgres:@localhost:5432/mydb")
        //     .map_err(|e| {
        //         AppError::InternalServerError(format!("Failed to connect PG DataBase {}", e))
        //     })
        //     .await?;
        let pg_db = PostgrePool::new("postgresql://postgres:@localhost:5432/mydb").await;
        let redis = RedisPool::new();
        let git_service = GitService::new();
        let ws_manager = WebSocketManager::new();

        Ok(Arc::new(AppState {
            redis,
            git_service,
            pg_db,
            ws_manager,
        }))
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

        if self.validate_user_pass(&username, &password).await.is_ok() {
            jwt::generate_token(&username)
                .map_err(|_| AppError::InternalServerError("Failed to generate token".to_string()))
        } else {
            info!("asdasdasd???");
            Err(AppError::Unauthorized(
                "validate user password not success".to_string(),
            ))
        }
    }

    async fn chack_if_exist(&self, username: &str) -> Result<(), AppError> {
        let option = sqlx::query("SELECT username FROM users WHERE username = $1")
            .bind(&username)
            .fetch_optional(&self.pg_db.pool)
            .await
            .map_err(|e| AppError::InternalServerError(format!("Database query failed: {}", e)))?;

        if option.is_some() {
            return Err(AppError::BadRequest("Username already exists".to_string()));
        }
        Ok(())
    }

    pub async fn register(&self, payload: request::RegisterRequest) -> Result<(), AppError> {
        self.chack_if_exist(&payload.username).await?;

        let result = sqlx::query(
            r#"
        INSERT INTO users (username, email, password, created_at) 
        VALUES ($1, $2, $3, $4)
        RETURNING id
        "#,
        )
        .bind(&payload.username)
        .bind(&payload.email)
        .bind(&payload.password) // 注意：实际应用中应该对密码进行哈希处理
        .bind(chrono::Utc::now())
        .fetch_one(&self.pg_db.pool)
        .await
        .map_err(|e| {
            AppError::InternalServerError(format!(
                "Failed to register user. Error: {}, User:{}",
                e, payload.username
            ))
        })?;

        let user_id = result
            .try_get("id")
            .map_err(|e| AppError::InternalServerError(format!("Failed to get user ID: {}", e)))?;

        let user = User {
            id: Some(user_id),
            username: payload.username.clone(),
            email: payload.email.clone(),
            password: payload.password.clone(), // 注意：实际中应使用哈希后的密码
            avatar: None,
            created_at: chrono::Utc::now(),
            department_id: None,
        };

        if let Err(_) = self.redis.cache_user_to_redis(&user).await {
            info!("Failed to cache user to Redis: user is {}", user.username);
        }

        self.git_service
            .generate_repopath(&payload.username)
            .await?;

        Ok(())
    }

    pub async fn get_user_data(&self, user_id: String) -> Result<UserData, AppError> {
        let user = sqlx::query_as::<_, User>(
            r#"
            SELECT * FROM users WHERE username = $1
            "#,
        )
        .bind(&user_id)
        .fetch_optional(&self.pg_db.pool)
        .await
        .map_err(|e| AppError::InternalServerError(format!("Database query failed: {}", e)))?;

        match user {
            Some(user) => {
                // 更新Redis缓存
                if let Err(_) = self.redis.cache_user_to_redis(&user).await {
                    info!("Failed to cache user to Redis: user is {}", user.username);
                }
                Ok(UserData {
                    username: user.username,
                    email: user.email,
                    avatar: user.avatar,
                })
            }
            None => Err(AppError::NotFound(format!("User not found: {}", user_id))),
        }
    }

    pub async fn update_user_data(
        &self,
        user_id: &str,
        new_data: UserUpdateRequest,
    ) -> Result<(), AppError> {
        let user = sqlx::query_as::<_, User>(
            r#"
            UPDATE users 
            SET email = COALESCE($1, email),
                avatar = COALESCE($2, avatar),
                department_id = COALESCE($3, department_id)
            WHERE username = $4
            RETURNING id, username, email, password, avatar, created_at, department_id
            "#,
        )
        .bind(new_data.email)
        .bind(new_data.avatar)
        .bind(new_data.department_id)
        .bind(user_id)
        .fetch_optional(&self.pg_db.pool)
        .await
        .map_err(|e| AppError::InternalServerError(format!("Database query failed: {}", e)))?;

        match user {
            Some(updated_user) => {
                // 更新Redis缓存
                self.redis.cache_user_to_redis(&updated_user).await?;
                Ok(())
            }
            None => Err(AppError::NotFound(format!("User not found: {}", user_id))),
        }
    }

    pub async fn update_user_password(
        &self,
        username: &str,
        new_password: &str,
    ) -> Result<(), AppError> {
        let result = sqlx::query(
            r#"
        UPDATE users 
        SET password = $1 
        WHERE username = $2
        "#,
        )
        .bind(new_password)
        .bind(username)
        .execute(&self.pg_db.pool)
        .await
        .map_err(|e| AppError::InternalServerError(format!("Database query failed: {}", e)))?;

        if result.rows_affected() == 0 {
            return Err(AppError::NotFound(format!("User not found: {}", username)));
        }

        Ok(())
    }

    async fn validate_user_pass(&self, username: &str, password: &str) -> Result<(), AppError> {
        let user_from_redis = self.redis.get_user_from_redis(username);

        match user_from_redis {
            Ok(user) => {
                // Redis 中找到了用户，验证密码
                if user.password != password {
                    info!("REDIS:get {};expected{}", password, user.password);
                    return Err(AppError::Unauthorized(
                        "Invalid username or password".to_string(),
                    ));
                }

                return Ok(());
            }
            Err(e) => {
                // todo!()
                // 只有当错误是"用户不存在"时，才尝试数据库查询
                if !matches!(e, AppError::NotFound(_)) {
                    // println!("error is {:?}", e);
                    return Err(e);
                }

                // Redis 中没有找到用户，从 PostgreSQL 查询
                match self.get_user_from_db(username).await {
                    Ok(user) => {
                        // 验证密码
                        if user.password != password {
                            info!("DB :get {};expected{}", password, user.password);
                            return Err(AppError::Unauthorized(
                                "Invalid username or password".to_string(),
                            ));
                        }
                        // 验证成功后 缓存到redis
                        if let Err(_) = self.redis.cache_user_to_redis(&user).await {
                            info!("Failed to cache user to Redis, user is {:?}", user);
                        }
                        Ok(())
                    }
                    Err(e) => {
                        // 数据库中也没有找到用户
                        println!("error is {:?}", e);
                        Err(AppError::Unauthorized(
                            "Invalid username or password".to_string(),
                        ))
                    }
                }
            }
        }
    }

    async fn get_user_from_db(&self, username: &str) -> Result<User, AppError> {
        let user = sqlx::query_as::<_, User>("SELECT * FROM users WHERE username = $1")
            .bind(username)
            .fetch_optional(&self.pg_db.pool)
            .await
            .map_err(|e| AppError::InternalServerError(format!("Database query failed: {}", e)))?;
        info!("db get user:{:?}", user.as_ref().unwrap());
        match user {
            Some(user) => Ok(user),
            None => Err(AppError::NotFound(format!("User not found: {}", username))),
        }
    }

    pub async fn get_user_messages(&self, user_id: &str) -> Result<Vec<Message>, AppError> {
        info!("get user messages: {}", user_id);
        let messages = sqlx::query_as::<_, Message>(
            r#"
            SELECT * FROM messages WHERE username = $1
            "#,
        )
        .bind(user_id)
        .fetch_all(&self.pg_db.pool)
        .await
        .map_err(|e| AppError::InternalServerError(format!("Database query failed: {}", e)))?;
        info!("get user messages: {:?}", messages);

        Ok(messages)
    }

    pub async fn add_message_for_users(
        &self,
        user_id_vec: &Vec<String>,
        msg: String,
        message_type: Option<String>,
    ) -> Result<(), AppError> {
        // let message_type = message_type.unwrap_or(message::MessageType::default());

        let msg_type = MessageType::try_from(message_type.unwrap_or("system".to_string()))
            .map_err(|e| AppError::InternalServerError(format!("Invalid message type: {}", e)))?;

        info!("add message for users: {:?}", user_id_vec);

        self.pg_db
            .add_message_for_users(user_id_vec, msg, msg_type)
            .await
    }
}

#[derive(Clone)]
pub struct RedisPool {
    // redis: Client,
    pool: Pool<Client>,
}

impl RedisPool {
    pub fn new() -> Self {
        let redis = Client::open("redis://127.0.0.1:6379").expect("Failed to connect to Redis");
        let pool = r2d2::Pool::builder().build(redis).unwrap();
        RedisPool { pool }
    }
    pub async fn cache_user_to_redis(&self, user: &User) -> Result<(), AppError> {
        let mut conn = self
            .pool
            .get()
            .map_err(|_| AppError::InternalServerError(format!("Failed to connect to Redis")))?;

        let user_json = serde_json::to_string(user)
            .map_err(|_| AppError::InternalServerError("Serialization failed".to_string()))?;

        let _: () = conn
            .set_ex(format!("user:{}", user.username), user_json, 3600)
            .map_err(|_| AppError::InternalServerError(format!("Failed to cache user to Redis")))?;

        info!("success to cache user:{}", user.username);

        Ok(())
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
            department_id: None,
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

        println!("already insert user");

        self.generate_repopath(&payload.username).await?;
        Ok(())
    }

    pub fn get_user_from_redis(&self, username: &str) -> Result<User, AppError> {
        let mut conn = self
            .pool
            .get()
            .map_err(|_| AppError::InternalServerError("Redis connection failed".to_string()))?;

        let user_exists: bool = conn
            .exists(format!("user:{}", username))
            .map_err(|_| AppError::InternalServerError("Redis operation failed".to_string()))?;

        if !user_exists {
            return Err(AppError::NotFound(format!(
                "User not found in Redis: {}",
                username
            )));
        }

        let user_json: String = conn
            .get(format!("user:{}", username))
            .map_err(|_| AppError::InternalServerError("Redis operation failed".to_string()))?;

        let user: User = serde_json::from_str(&user_json)
            .map_err(|_| AppError::InternalServerError("Failed to parse user data".to_string()))?;

        Ok(user)
    }

    pub async fn generate_repopath(&self, user_name: &str) -> Result<(), AppError> {
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

    pub async fn generate_repopath(&self, user_name: &str) -> Result<(), AppError> {
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

    pub async fn clone_repo_for_user(
        &self,
        user_id: &str,
        repo_url: &str,
        repo_name: &str,
        ws_manager: &WebSocketManager,
    ) -> Result<String, AppError> {
        self.git_manager
            .clone_repository_for_user(user_id, repo_url, repo_name, ws_manager)
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

    pub async fn get_repo_commit_count(
        &self,
        user_id: &str,
        repo_name: &str,
    ) -> Result<usize, AppError> {
        //先从redis里面找，再去数据库里面找。都没有再去git_manager里面找
        let total_count = self
            .git_manager
            .get_total_commits_count(user_id, repo_name)?;
        Ok(total_count)
    }

    pub async fn get_repo_commit_histories(
        &self,
        user_id: &str,
        repo_name: &str,
        limit: usize,
        page: usize,
    ) -> Result<Vec<CommitInfo>, AppError> {
        let total_count = self.get_repo_commit_count(user_id, repo_name).await?;
        info!("get count ={}", total_count);
        if total_count == 0 {
            return Ok(vec![]);
        }
        if (page - 1) * limit > total_count {
            return Err(AppError::BadRequest(
                "Page number exceeds total commit count".to_string(),
            ));
        }
        self.git_manager
            .get_commit_histories(user_id, repo_name, limit, page, total_count)
    }

    pub async fn get_repos_data_for_users(&self, user_id: &str) -> Result<Vec<ReposVo>, AppError> {
        self.git_manager.get_repos_data_for_users(user_id)
    }

    pub async fn get_repo_commit_diff(
        &self,
        user_id: &str,
        repo_name: &str,
        commit_id: &str,
    ) -> Result<CommitDetail, AppError> {
        self.git_manager
            .get_commit_detail(user_id, repo_name, commit_id)
            .await
    }

    pub async fn list_repository_files(
        &self,
        user_id: &str,
        repo_name: &str,
        directory_path: Option<&str>,
        branch: Option<&str>,
    ) -> Result<Vec<GitFileEntry>, AppError> {
        self.git_manager
            .list_repository_files(user_id, repo_name, directory_path, branch)
    }

    pub async fn get_file_content(
        &self,
        user_id: &str,
        repo_name: &str,
        file_path: &str,
        branch: Option<&str>,
    ) -> Result<String, AppError> {
        self.git_manager
            .get_file_content(user_id, repo_name, file_path, branch)
    }

    pub async fn del_repo_for_user(&self, user_id: &str, repo_name: &str) -> Result<(), AppError> {
        self.git_manager.del_repo(user_id, repo_name).await
    }

    pub async fn update_repo_data(
        &self,
        user_id: &str,
        repo_name: &str,
        new_name: &str,
    ) -> Result<(), AppError> {
        self.git_manager
            .update_repo_data(user_id, repo_name, new_name)
            .await
    }

    pub async fn get_repo_branches(
        &self,
        user_id: &str,
        repo_name: &str,
    ) -> Result<Vec<String>, AppError> {
        self.git_manager.get_repo_branchs(user_id, repo_name).await
    }

    pub async fn pull_repo(
        &self,
        user_id: &str,
        repo_name: &str,
        branch: Option<&str>,
    ) -> Result<(), AppError> {
        self.git_manager.pull_repo(user_id, repo_name, branch).await
    }
}
